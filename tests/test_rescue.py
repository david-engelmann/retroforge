"""RF-9 to RF-13, rescue moments."""
import json
from retroforge.digest import digest_of
from retroforge.moments import rescue_moments
from retroforge.proposals import build_proposals, validate_proposals

T0 = "2026-01-05T09:00:00.000Z"


def msg(seq, role, sidechain=False):
    return {"seq": seq, "kind": "message", "sidechain": sidechain,
            "timestamp": T0, "role": role, "text": f"turn {seq}"}


def call(seq, tool="Edit"):
    return {"seq": seq, "kind": "tool_call", "sidechain": False,
            "timestamp": T0, "call_id": f"c{seq}", "tool": tool,
            "input": {"path": "a.py"}}


def tape(events, started=T0):
    return {"tape_version": "1.0",
            "run": {"harness": "claude-code", "session_id": "s1",
                    "started_at": started},
            "events": events,
            "outcome": {"status": "completed",
                        "basis": "final-assistant-message"}}


def rescue_tape(tool="Edit", rescues=2):
    events = [msg(0, "user")]
    seq = 1
    for _ in range(rescues):
        events.append(call(seq, tool)); seq += 1
        events.append(msg(seq, "assistant")); seq += 1
        events.append(msg(seq, "user")); seq += 1
    return tape(events)


def test_a_rescue_needs_an_agent_action_since_the_last_user_turn():
    events = [msg(0, "user"), msg(1, "user"), call(2),
              msg(3, "user"), msg(4, "assistant"), msg(5, "user")]
    found = rescue_moments(tape(events))
    assert [m["seq"] for m in found] == [3, 5]
    assert all(m["tool"] == "Edit" for m in found)


def test_the_first_user_message_is_never_a_rescue():
    events = [call(0), msg(1, "user"), call(2), msg(3, "user")]
    found = rescue_moments(tape(events))
    assert [m["seq"] for m in found] == [3]


def test_sidechain_turns_neither_rescue_nor_attribute():
    events = [msg(0, "user"), call(1),
              {"seq": 2, "kind": "tool_call", "sidechain": True,
               "timestamp": T0, "call_id": "cx", "tool": "Bash",
               "input": {}},
              msg(3, "user", sidechain=True), msg(4, "user")]
    found = rescue_moments(tape(events))
    assert found == [{"seq": 4, "tool": "Edit"}]


def test_an_assistant_message_alone_is_an_agent_action():
    events = [msg(0, "user"), msg(1, "assistant"), msg(2, "user"),
              msg(3, "assistant"), msg(4, "user")]
    found = rescue_moments(tape(events))
    assert [m["seq"] for m in found] == [2, 4]
    assert all(m["tool"] is None for m in found)


def test_two_attributed_rescues_emit_one_rescue_proposal():
    one = rescue_tape()
    digest = digest_of(one)
    document = build_proposals([("a.json", one, digest)])
    assert [p["id"] for p in document["proposals"]] == [
        "rules-edit-rescue-Edit"]
    proposal = document["proposals"][0]
    assert "followed Edit work 2 times" in proposal["hunk"]["text"]
    assert all(c["kind"] == "message"
               for c in proposal["citations"])
    assert validate_proposals(document, {digest: one}) == []
    assert "unattributed_rescues" not in document


def test_below_threshold_and_unattributed_accounting():
    events = [msg(0, "user"), msg(1, "assistant"), msg(2, "user"),
              call(3), msg(4, "user")]
    one = tape(events)
    digest = digest_of(one)
    document = build_proposals([("a.json", one, digest)])
    assert document["proposals"] == []
    assert document["unattributed_rescues"] == 1
    assert validate_proposals(document, {digest: one}) == []
    bad = dict(document, unattributed_rescues=-1)
    assert validate_proposals(bad, {digest: one}) != []
    bad = dict(document, unattributed_rescues=True)
    assert validate_proposals(bad, {digest: one}) != []


def test_rescues_accumulate_across_tapes_and_follow_redo_in_order():
    first = rescue_tape(rescues=1)
    second = rescue_tape(rescues=1)
    second["run"]["session_id"] = "s2"
    d1, d2 = digest_of(first), digest_of(second)
    document = build_proposals([("a.json", first, d1),
                                ("b.json", second, d2)])
    assert [p["id"] for p in document["proposals"]] == [
        "rules-edit-rescue-Edit"]
    assert len(document["proposals"][0]["citations"]) == 2


def result(seq, call_id, is_error):
    return {"seq": seq, "kind": "tool_result", "sidechain": False,
            "timestamp": T0, "call_id": call_id,
            "is_error": is_error, "text": "boom" if is_error else "ok"}


def test_the_same_tape_twice_is_one_tape():
    one = rescue_tape(rescues=1)
    digest = digest_of(one)
    document = build_proposals([("a.json", one, digest),
                                ("copy.json", one, digest)])
    assert document["proposals"] == []
    two = rescue_tape(rescues=2)
    d2 = digest_of(two)
    document = build_proposals([("a.json", two, d2),
                                ("copy.json", two, d2)])
    assert len(document["proposals"]) == 1
    assert len(document["proposals"][0]["citations"]) == 2


def test_attribution_is_the_most_recent_tool_call():
    events = [msg(0, "user"), call(1, "Edit"), call(2, "Bash"),
              msg(3, "user"), call(4, "Edit"), msg(5, "user")]
    found = rescue_moments(tape(events))
    assert found == [{"seq": 3, "tool": "Bash"},
                     {"seq": 5, "tool": "Edit"}]


def test_redo_precedes_rescue_for_one_tool_in_one_period():
    events = [msg(0, "user")]
    seq = 1
    for i in range(2):
        events.append(call(seq, "Edit"))
        events.append(result(seq + 1, f"c{seq}", True)); seq += 2
    events.append(call(seq, "Edit")); seq += 1
    for _ in range(2):
        events.append(msg(seq, "assistant")); seq += 1
        events.append(msg(seq, "user")); seq += 1
    one = tape(events)
    digest = digest_of(one)
    document = build_proposals([("a.json", one, digest)])
    assert [p["id"] for p in document["proposals"]] == [
        "rules-edit-Edit", "rules-edit-rescue-Edit"]
    assert validate_proposals(document, {digest: one}) == []


def test_a_tool_named_to_collide_still_yields_a_valid_file():
    events = [msg(0, "user")]
    seq = 1
    for tool in ("Edit", "rescue-Edit"):
        for _ in range(2):
            events.append(call(seq, tool)); seq += 1
            events.append(msg(seq, "assistant")); seq += 1
            events.append(msg(seq, "user")); seq += 1
    for i in range(2):
        events.append(call(seq, "rescue-Edit"))
        events.append(result(seq + 1, f"c{seq}", True)); seq += 2
    events.append(call(seq, "rescue-Edit")); seq += 1
    one = tape(events)
    digest = digest_of(one)
    document = build_proposals([("a.json", one, digest)])
    ids = [p["id"] for p in document["proposals"]]
    assert len(ids) == len(set(ids))
    assert validate_proposals(document, {digest: one}) == []


def test_a_message_citation_must_name_an_operator_turn():
    one = rescue_tape(rescues=2)
    digest = digest_of(one)
    document = build_proposals([("a.json", one, digest)])
    proposal = document["proposals"][0]
    forged = json.loads(json.dumps(document))
    assistant_seq = next(
        e["seq"] for e in one["events"]
        if e["kind"] == "message" and e["role"] == "assistant")
    forged["proposals"][0]["citations"][0]["seq"] = assistant_seq
    defects = validate_proposals(forged, {digest: one})
    assert any("not a main-chain user turn" in d for d in defects)
    assert proposal["citations"][0]["kind"] == "message"
