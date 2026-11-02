"""Redo moments (RF-4).

A redo moment is a tool call whose tool and input match an
earlier tool call in the same tape, with an error tool
result answering the earlier call in between. The moment is
recorded at the later call's seq. Inputs are compared by
their canonical form 1 bytes, so two inputs that differ
only in key order are the same call, and two that differ in
any value are not.
"""
from __future__ import annotations

from .digest import DigestError, canonical_bytes


def redo_moments(tape):
    """Every redo moment in one valid tape, in seq order.

    Each moment is a dict with the later call's seq, the
    earlier call's seq, and the tool name. A call whose
    input cannot be canonicalized is skipped; it can never
    be shown to match another call.
    """
    moments = []
    earlier = []
    errors_between = set()
    for event in tape["events"]:
        kind = event.get("kind")
        if kind == "tool_result":
            if event.get("is_error") is True:
                call_id = event.get("call_id")
                if isinstance(call_id, str):
                    errors_between.add(call_id)
            continue
        if kind != "tool_call":
            continue
        try:
            key = (event.get("tool"),
                   canonical_bytes(event.get("input")))
        except DigestError:
            continue
        for past_key, past_seq, past_call_id in earlier:
            if past_key == key and past_call_id in errors_between:
                moments.append({"seq": event["seq"],
                                "earlier_seq": past_seq,
                                "tool": event.get("tool")})
                break
        call_id = event.get("call_id")
        earlier.append((key, event["seq"],
                        call_id if isinstance(call_id, str) else None))
    return moments
