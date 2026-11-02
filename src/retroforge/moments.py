"""Redo moments (RF-4).

A redo moment is a tool call whose tool and input match an
earlier tool call in the same tape, with an error tool
result answering the earlier call in between. The moment is
recorded at the later call's seq. Inputs are compared by
their canonical form 1 bytes, so two inputs that differ
only in key order are the same call, and two that differ in
any value are not. The tape has already passed the format
checks in tapes.py, so a result's call id answers an
earlier call and arrives after it; an error recorded for a
call is therefore always in between that call and any
later repeat examined here. Earlier calls are indexed by
their tool and input, so detection stays linear in the
size of the tape.
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
    calls = {}
    first_errored = {}
    for event in tape["events"]:
        kind = event.get("kind")
        if kind == "tool_result":
            if event.get("is_error") is True:
                found = calls.get(event.get("call_id"))
                if found is not None:
                    key, seq = found
                    current = first_errored.get(key)
                    if current is None or seq < current:
                        first_errored[key] = seq
            continue
        if kind != "tool_call":
            continue
        try:
            key = (event.get("tool"),
                   canonical_bytes(event.get("input")))
        except DigestError:
            continue
        if key in first_errored:
            moments.append({"seq": event["seq"],
                            "earlier_seq": first_errored[key],
                            "tool": event.get("tool")})
        call_id = event.get("call_id")
        if isinstance(call_id, str):
            calls[call_id] = (key, event["seq"])
    return moments


def rescue_moments(tape):
    """Every rescue moment in one valid tape, in seq
    order (RF-9, RF-10).

    A rescue moment is a main-chain user message that
    is not the tape's first main-chain user message and
    that arrives after at least one agent action since
    the previous main-chain user message, an agent
    action being a main-chain assistant message or a
    main-chain tool call. Each moment is a dict with
    the message's seq and the tool it is attributed
    to, the tool of the most recent main-chain tool
    call before it, or None when no main-chain tool
    call precedes it.
    """
    moments = []
    seen_user = False
    agent_acted = False
    last_tool = None
    for event in tape["events"]:
        if event.get("sidechain") is not False:
            continue
        kind = event.get("kind")
        if kind == "tool_call":
            agent_acted = True
            last_tool = event.get("tool")
        elif kind == "message" and event.get("role") == "assistant":
            agent_acted = True
        elif kind == "message" and event.get("role") == "user":
            if seen_user and agent_acted:
                moments.append({"seq": event["seq"],
                                "tool": last_tool})
            seen_user = True
            agent_acted = False
    return moments
