"""RF-1 to RF-8, evidence cited proposals."""
import json
import pathlib

import pytest

from retroforge.cli import main
from retroforge.digest import digest_of
from retroforge.moments import redo_moments
from retroforge.proposals import build_proposals, validate_proposals

FIXTURE = (pathlib.Path(__file__).parent / "fixtures"
           / "tape_synthetic.json")
T0 = "2026-01-05T09:00:00.000Z"
T1 = "2026-01-06T09:00:00.000Z"


def call(seq, call_id, tool="Read", path="a.py", ts=T0):
    return {"seq": seq, "kind": "tool_call", "sidechain": False,
            "timestamp": ts, "call_id": call_id, "tool": tool,
            "input": {"path": path}}


def result(seq, call_id, is_error, ts=T0):
    return {"seq": seq, "kind": "tool_result", "sidechain": False,
            "timestamp": ts, "call_id": call_id,
            "is_error": is_error, "text": "boom" if is_error else "ok"}


def redo_tape(started=T0, failures=2):
    events = []
    for i in range(failures):
        events.append(call(2 * i, f"c{i}", ts=started))
        events.append(result(2 * i + 1, f"c{i}", True, ts=started))
    events.append(call(2 * failures, "cx", ts=started))
    return {"tape_version": "1.0",
            "run": {"harness": "claude-code", "session_id": "s1",
                    "started_at": started},
            "events": events,
            "outcome": {"status": "completed",
                        "basis": "final-assistant-message"}}


def test_the_pinned_digest_matches_the_flighttape_golden():
    tape = json.loads(FIXTURE.read_text(encoding="utf-8"))
    assert digest_of(tape) == (
        "cc9c4e01fca418dcc33114caf75fae1f6bf0d401f297b5d61f6776903fa955a3")


def test_a_redo_needs_the_same_input_and_an_error_between():
    tape = redo_tape(failures=1)
    assert redo_moments(tape) == [
        {"seq": 2, "earlier_seq": 0, "tool": "Read"}]
    changed = redo_tape(failures=1)
    changed["events"][2]["input"] = {"path": "other.py"}
    assert redo_moments(changed) == []
    succeeded = redo_tape(failures=1)
    succeeded["events"][1]["is_error"] = False
    assert redo_moments(succeeded) == []
    reordered = redo_tape(failures=1)
    reordered["events"][0]["input"] = {"path": "a.py", "x": 1}
    reordered["events"][2]["input"] = {"x": 1, "path": "a.py"}
    assert len(redo_moments(reordered)) == 1


def test_two_redo_moments_emit_one_proposal_with_resolving_citations():
    tape = redo_tape()
    digest = digest_of(tape)
    document = build_proposals([("a.json", tape, digest)])
    assert document["proposal_version"] == "1.0"
    assert document["period"] == {"from": T0, "to": T0}
    assert len(document["proposals"]) == 1
    proposal = document["proposals"][0]
    assert proposal["type"] == "rules-edit"
    assert proposal["hunk"]["action"] == "add"
    assert "Read" in proposal["hunk"]["text"]
    assert len(proposal["citations"]) == 2
    assert validate_proposals(document, {digest: tape}) == []
    stub = proposal["registry"]
    assert stub["experiment_id"].startswith("exp-")
    assert stub["baseline_period"] == {"from": T0, "to": T0}
    again = build_proposals([("a.json", tape, digest)])
    assert again == document


def test_below_the_threshold_no_proposal_is_emitted():
    tape = redo_tape(failures=1)
    document = build_proposals([("a.json", tape, digest_of(tape))])
    assert document["proposals"] == []


def test_the_validator_names_every_broken_citation():
    tape = redo_tape()
    digest = digest_of(tape)
    document = build_proposals([("a.json", tape, digest)])
    proposal = document["proposals"][0]
    proposal["citations"] = proposal["citations"][:1]
    defects = validate_proposals(document, {digest: tape})
    assert any("fewer than two" in d for d in defects)

    document = build_proposals([("a.json", tape, digest)])
    document["proposals"][0]["citations"][0]["tape"] = "0" * 64
    defects = validate_proposals(document, {digest: tape})
    assert any("outside the input set" in d for d in defects)

    document = build_proposals([("a.json", tape, digest)])
    document["proposals"][0]["citations"][0]["kind"] = "note"
    defects = validate_proposals(document, {digest: tape})
    assert any("wrong kind" in d for d in defects)

    document = build_proposals([("a.json", tape, digest)])
    document["proposals"][0]["citations"][0]["seq"] = 99
    defects = validate_proposals(document, {digest: tape})
    assert any("names no event" in d for d in defects)


def write(directory, name, value):
    path = directory / name
    if isinstance(value, str):
        path.write_text(value, encoding="utf-8")
    else:
        path.write_text(json.dumps(value), encoding="utf-8")
    return path


def test_cli_propose_over_a_mixed_directory(tmp_path, capsys):
    write(tmp_path, "a.json", redo_tape())
    write(tmp_path, "b.json", "{not json")
    write(tmp_path, "notes.txt", "ignored")
    out = tmp_path / "proposal.json"
    assert main(["propose", str(tmp_path), "-o", str(out)]) == 0
    err = capsys.readouterr().err
    assert "b.json is excluded" in err
    document = json.loads(out.read_text(encoding="utf-8"))
    assert len(document["proposals"]) == 1


def test_cli_no_valid_tapes_writes_nothing(tmp_path, capsys):
    write(tmp_path, "b.json", "{not json")
    out = tmp_path / "proposal.json"
    assert main(["propose", str(tmp_path), "-o", str(out)]) == 1
    assert "no valid tapes" in capsys.readouterr().err
    assert not out.exists()
    assert main(["propose", str(tmp_path / "missing")]) == 1


def test_cli_propose_to_stdout_is_byte_deterministic(tmp_path, capsys):
    write(tmp_path, "a.json", redo_tape(started=T0))
    write(tmp_path, "z.json", redo_tape(started=T1))
    assert main(["propose", str(tmp_path)]) == 0
    first = capsys.readouterr().out
    assert main(["propose", str(tmp_path)]) == 0
    assert capsys.readouterr().out == first
    document = json.loads(first)
    assert document["period"] == {"from": T0, "to": T1}
    assert len(document["proposals"][0]["citations"]) == 4



from retroforge.digest import canonical_bytes
from retroforge.evidence import EvidencePackError, gate_citations
from retroforge.tapes import EVENT_KINDS, tape_defects


def test_canonical_form_1_battery_against_the_reference_values():
    cases = [
        (42, b"42"),
        (150.0, b"150"),
        (0.000001, b"0.000001"),
        (1e-7, b"1e-7"),
        (1e21, b"1e+21"),
        (123456789.123456789, b"123456789.12345679"),
        (5e-324, b"5e-324"),
        (-0.0, b"0"),
        ({"b": 1e21, "a": [0.000001, 2.5]},
         b'{"a":[0.000001,2.5],"b":1e+21}'),
        ({"￿": 1, "\U00010000": 2},
         b'{"\xf0\x90\x80\x80":2,"\xef\xbf\xbf":1}'),
    ]
    for value, expected in cases:
        assert canonical_bytes(value) == expected, value


def test_the_kind_enum_is_the_formats_eight():
    assert EVENT_KINDS == {
        "session_start", "message", "tool_call", "tool_result",
        "diff", "spend", "note", "session_end"}


def test_pairing_and_shape_defects_exclude_a_tape():
    dangling = redo_tape(failures=1)
    dangling["events"].insert(0, result(0, "ghost", True))
    for index, event in enumerate(dangling["events"]):
        event["seq"] = index
    assert any("answers no earlier" in d
               for d in tape_defects(dangling))
    listed = redo_tape(failures=1)
    listed["events"][0]["tool"] = ["Read"]
    assert any("tool must be" in d for d in tape_defects(listed))
    missing = redo_tape(failures=1)
    del missing["events"][0]["tool"]
    assert any("tool must be" in d for d in tape_defects(missing))
    dup = redo_tape(failures=1)
    dup["events"][2]["call_id"] = "c0"
    assert any("used twice" in d for d in tape_defects(dup))
    bad_seq = redo_tape(failures=1)
    bad_seq["events"][0]["seq"] = True
    assert any("seq is not 0" in d for d in tape_defects(bad_seq))
    bad_error = redo_tape(failures=1)
    bad_error["events"][1]["is_error"] = 1
    assert any("is_error must be" in d
               for d in tape_defects(bad_error))
    future = {"tape_version": "2.0", "run": {}, "events": [
        call(0, "c0")]}
    assert any("tape_version" in d for d in tape_defects(future))


def test_an_error_must_answer_the_earlier_call_before_the_repeat():
    tape = {"tape_version": "1.0", "run": {"started_at": T0},
            "events": [
                call(0, "a"), result(1, "a", False),
                call(2, "b"), result(3, "b", True),
                call(4, "c")],
            "outcome": {"status": "completed",
                        "basis": "final-assistant-message"}}
    assert redo_moments(tape) == [
        {"seq": 4, "earlier_seq": 2, "tool": "Read"}]
    late = {"tape_version": "1.0", "run": {"started_at": T0},
            "events": [
                call(0, "a"), call(1, "b"), result(2, "a", True)],
            "outcome": {"status": "completed",
                        "basis": "final-assistant-message"}}
    assert redo_moments(late) == []


def test_the_experiment_id_moves_with_the_baseline_period():
    redo = redo_tape()
    quiet = {"tape_version": "1.0",
             "run": {"harness": "claude-code", "session_id": "q",
                     "started_at": T1},
             "events": [call(0, "only", tool="Write", ts=T1)],
             "outcome": {"status": "completed",
                         "basis": "final-assistant-message"}}
    base = build_proposals([("a.json", redo, digest_of(redo))])
    wider = build_proposals(
        [("a.json", redo, digest_of(redo)),
         ("z.json", quiet, digest_of(quiet))])
    first = base["proposals"][0]
    second = wider["proposals"][0]
    assert first["citations"] == second["citations"]
    assert second["registry"]["baseline_period"] == {
        "from": T0, "to": T1}
    assert first["registry"]["experiment_id"] != \
        second["registry"]["experiment_id"]


def test_the_validator_checks_period_registry_and_duplicate_ids():
    tape = redo_tape()
    digest = digest_of(tape)
    document = build_proposals([("a.json", tape, digest)])
    document["proposals"][0]["registry"]["experiment_id"] = "x"
    defects = validate_proposals(document, {digest: tape})
    assert any("experiment_id" in d for d in defects)
    document = build_proposals([("a.json", tape, digest)])
    del document["period"]
    defects = validate_proposals(document, {digest: tape})
    assert any("period" in d for d in defects)
    document = build_proposals([("a.json", tape, digest)])
    document["proposals"].append(dict(document["proposals"][0]))
    defects = validate_proposals(document, {digest: tape})
    assert any("used twice" in d for d in defects)


def test_gate_citations_refuse_until_the_format_is_pinned():
    with pytest.raises(EvidencePackError):
        gate_citations("pack.json")


def test_cli_the_output_never_overwrites_an_input_tape(
        tmp_path, capsys):
    tape_path = write(tmp_path, "a.json", redo_tape())
    before = tape_path.read_text(encoding="utf-8")
    assert main(["propose", str(tmp_path),
                 "-o", str(tape_path)]) == 1
    assert "never writes over" in capsys.readouterr().err
    assert tape_path.read_text(encoding="utf-8") == before


def test_cli_a_write_failure_is_a_named_error(tmp_path, capsys):
    write(tmp_path, "a.json", redo_tape())
    assert main(["propose", str(tmp_path),
                 "-o", str(tmp_path)]) == 1
    assert "cannot write the proposal file" in \
        capsys.readouterr().err


def test_cli_orderings_are_filename_then_tool(tmp_path, capsys):
    zeta = redo_tape()
    zeta["events"] = [
        call(0, "z0", tool="Zeta"), result(1, "z0", True),
        call(2, "z1", tool="Zeta"), result(3, "z1", True),
        call(4, "z2", tool="Zeta")]
    write(tmp_path, "a.json", zeta)
    alpha = redo_tape()
    alpha["events"] = [
        call(0, "a0", tool="Alpha"), result(1, "a0", True),
        call(2, "a1", tool="Alpha"), result(3, "a1", True),
        call(4, "a2", tool="Alpha")]
    write(tmp_path, "z.json", alpha)
    assert main(["propose", str(tmp_path)]) == 0
    document = json.loads(capsys.readouterr().out)
    assert [p["id"] for p in document["proposals"]] == [
        "rules-edit-Alpha", "rules-edit-Zeta"]


def test_cli_citations_follow_filename_order(tmp_path, capsys):
    first = redo_tape(failures=1)
    second = redo_tape(failures=1)
    second["run"]["session_id"] = "other"
    write(tmp_path, "a.json", first)
    write(tmp_path, "z.json", second)
    assert main(["propose", str(tmp_path)]) == 0
    document = json.loads(capsys.readouterr().out)
    cited = [c["tape"] for c in
             document["proposals"][0]["citations"]]
    assert cited == [digest_of(first), digest_of(second)]


def test_cli_a_rerun_may_overwrite_its_own_proposal(
        tmp_path, capsys):
    write(tmp_path, "a.json", redo_tape())
    out = tmp_path / "proposal.json"
    assert main(["propose", str(tmp_path), "-o", str(out)]) == 0
    assert main(["propose", str(tmp_path), "-o", str(out)]) == 0
    err = capsys.readouterr().err
    assert "proposal.json is excluded" in err


def test_an_unhashable_kind_is_a_defect_not_a_crash():
    tape = redo_tape(failures=1)
    tape["events"][0]["kind"] = ["tool_call"]
    defects = tape_defects(tape)
    assert any("kind is not in the format enum" in d
               for d in defects)


def test_canonical_form_renders_big_integers_exactly():
    assert canonical_bytes(2 ** 60 + 1) == b"1152921504606846977"


def test_identical_calls_cite_the_earliest_errored_call():
    tape = redo_tape(failures=3)
    moments = redo_moments(tape)
    assert [m["seq"] for m in moments] == [2, 4, 6]
    assert all(m["earlier_seq"] == 0 for m in moments)


def test_a_failed_write_leaves_the_earlier_proposal_intact(
        tmp_path, monkeypatch):
    import os as _os
    write(tmp_path, "a.json", redo_tape())
    out = tmp_path / "proposal.json"
    assert main(["propose", str(tmp_path), "-o", str(out)]) == 0
    before = out.read_text(encoding="utf-8")

    def broken_replace(source, target):
        raise OSError("simulated replace failure")

    monkeypatch.setattr(_os, "replace", broken_replace)
    assert main(["propose", str(tmp_path), "-o", str(out)]) == 1
    assert out.read_text(encoding="utf-8") == before
    assert not list(tmp_path.glob("proposal.json.*.tmp"))
