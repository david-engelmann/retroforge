"""Reading flighttape tapes (RF-3).

Retroforge reads tapes at flighttape format version 1.0
only, never raw harness session files. The checks here are
the ones citation resolution depends on: the value is an
object carrying tape_version 1.0, a run object, and an
events array whose seq values run consecutively from 0 and
whose kinds are in the format's enum. A file that fails to
load or fails these checks is excluded with its reason
named, in the rollup discipline, and the run proceeds on
the tapes that remain.
"""
from __future__ import annotations

import json

EVENT_KINDS = {
    "session_start", "message", "tool_call", "tool_result",
    "diff", "spend", "note", "session_end",
}


class _Failed:
    def __repr__(self):
        return "FAILED"


FAILED = _Failed()


def load_tape_file(path):
    """Read one tape file, (tape, None) or (FAILED, reason)."""
    try:
        text = path.read_bytes()
    except OSError:
        return FAILED, "cannot read the tape file"
    try:
        text = text.decode("utf-8")
    except UnicodeDecodeError:
        return FAILED, "the file is not valid UTF-8"
    try:
        tape = json.loads(text)
    except RecursionError:
        return FAILED, "the file is not valid JSON"
    except ValueError as error:
        if "integer string conversion" in str(error):
            return FAILED, "the file holds a number too large to read"
        return FAILED, "the file is not valid JSON"
    defects = tape_defects(tape)
    if defects:
        return FAILED, "not a valid tape, " + "; ".join(defects)
    return tape, None


def tape_defects(tape):
    """The format checks citation resolution depends on."""
    if not isinstance(tape, dict):
        return ["the tape is not an object"]
    defects = []
    if tape.get("tape_version") != "1.0":
        defects.append("tape_version is not 1.0")
    if not isinstance(tape.get("run"), dict):
        defects.append("run is not an object")
    events = tape.get("events")
    if not isinstance(events, list) or not events:
        defects.append("events must hold at least one event")
        return defects
    for index, event in enumerate(events):
        if not isinstance(event, dict):
            defects.append(f"events[{index}] is not an object")
            continue
        if event.get("seq") != index:
            defects.append(
                f"events[{index}].seq is not {index}")
        if event.get("kind") not in EVENT_KINDS:
            defects.append(
                f"events[{index}].kind is not in the format enum")
    return defects
