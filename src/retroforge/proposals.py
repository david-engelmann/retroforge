"""Proposal files (RF-1, RF-2, RF-5, RF-6, RF-7).

A proposal file is the run's only output: a JSON object
with a format version, the period its tapes cover, and its
proposals. Every proposal carries at least two citations,
and every citation must resolve against the tape set it was
built from, a tape digest under canonical form 1, a seq,
and the kind of the event at that seq. The validator here
is the enforcement of obligation one: a proposal with no
evidence, or with evidence that does not resolve, is
invalid and the validator names each broken citation.
"""
from __future__ import annotations

from .digest import digest_of
from .moments import redo_moments

PROPOSAL_VERSION = "1.0"
PROPOSAL_TYPES = ("rules-edit", "hook-change", "roster-note")
HUNK_ACTIONS = ("add", "strike", "rewrite")
REDO_THRESHOLD = 2


def build_proposals(tapes, target="AGENTS.md"):
    """Build a proposal file from (name, tape, digest) triples.

    The triples are taken in the order given, which the CLI
    supplies in filename order; proposals come out ordered
    by tool name, so the file is deterministic (RF-7). The
    registry stub on each proposal derives its experiment id
    from the proposal's own content (RF-6).
    """
    starts = [tape["run"].get("started_at") for _, tape, _ in tapes]
    starts = [value for value in starts if isinstance(value, str)]
    period = {"from": min(starts) if starts else None,
              "to": max(starts) if starts else None}
    by_tool = {}
    for _, tape, digest in tapes:
        for moment in redo_moments(tape):
            by_tool.setdefault(moment["tool"], []).append(
                (digest, moment))
    proposals = []
    for tool in sorted(by_tool, key=lambda name: (name is None, name)):
        moments = by_tool[tool]
        if len(moments) < REDO_THRESHOLD:
            continue
        citations = [{"tape": digest, "seq": moment["seq"],
                      "kind": "tool_call"}
                     for digest, moment in moments]
        proposal = {
            "id": f"rules-edit-{tool}",
            "type": "rules-edit",
            "target": target,
            "hunk": {
                "action": "add",
                "text": (f"Before calling {tool} again after a "
                         f"failure, stop and read the error, then "
                         f"change the input or the approach. This "
                         f"run repeated a failed {tool} call "
                         f"{len(moments)} times across the period."),
            },
            "rationale": (
                f"{tool} was called again with the same input "
                f"after an error {len(moments)} times in the "
                f"period, at the cited moments."),
            "citations": citations,
        }
        proposal["registry"] = {
            "experiment_id": "exp-" + digest_of(
                {key: proposal[key]
                 for key in ("type", "target", "hunk",
                             "citations")})[:16],
            "baseline_period": period,
        }
        proposals.append(proposal)
    return {"proposal_version": PROPOSAL_VERSION,
            "period": period,
            "proposals": proposals}


def validate_proposals(document, tapes_by_digest):
    """Name every defect in a proposal file (RF-1, RF-2).

    tapes_by_digest maps each input tape's canonical form 1
    digest to the tape itself, the ground every citation is
    resolved against.
    """
    if not isinstance(document, dict):
        return ["the proposal file is not an object"]
    defects = []
    if document.get("proposal_version") != PROPOSAL_VERSION:
        defects.append("proposal_version is not 1.0")
    proposals = document.get("proposals")
    if not isinstance(proposals, list):
        defects.append("proposals is not an array")
        return defects
    for index, proposal in enumerate(proposals):
        where = f"proposals[{index}]"
        if not isinstance(proposal, dict):
            defects.append(f"{where} is not an object")
            continue
        for field in ("id", "type", "target", "hunk",
                      "rationale", "citations", "registry"):
            if field not in proposal:
                defects.append(f"{where} is missing {field}")
        if proposal.get("type") not in PROPOSAL_TYPES:
            defects.append(f"{where}.type is not a proposal type")
        hunk = proposal.get("hunk")
        if isinstance(hunk, dict):
            if hunk.get("action") not in HUNK_ACTIONS:
                defects.append(
                    f"{where}.hunk.action is not a hunk action")
            if not isinstance(hunk.get("text"), str) \
                    or not hunk.get("text"):
                defects.append(f"{where}.hunk.text is empty")
        elif "hunk" in proposal:
            defects.append(f"{where}.hunk is not an object")
        citations = proposal.get("citations")
        if isinstance(citations, list):
            if len(citations) < 2:
                defects.append(
                    f"{where} carries fewer than two citations")
            for cindex, citation in enumerate(citations):
                defects.extend(_citation_defects(
                    f"{where}.citations[{cindex}]", citation,
                    tapes_by_digest))
        elif "citations" in proposal:
            defects.append(f"{where}.citations is not an array")
    return defects


def _citation_defects(where, citation, tapes_by_digest):
    if not isinstance(citation, dict):
        return [f"{where} is not an object"]
    digest = citation.get("tape")
    tape = tapes_by_digest.get(digest) \
        if isinstance(digest, str) else None
    if tape is None:
        return [f"{where} names a tape outside the input set"]
    seq = citation.get("seq")
    events = tape["events"]
    if not isinstance(seq, int) or isinstance(seq, bool) \
            or not 0 <= seq < len(events):
        return [f"{where} names no event in its tape"]
    if events[seq].get("kind") != citation.get("kind"):
        return [f"{where} states the wrong kind for its event"]
    return []
