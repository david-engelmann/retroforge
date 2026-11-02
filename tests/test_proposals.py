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
