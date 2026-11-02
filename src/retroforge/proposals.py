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
from .moments import redo_moments, rescue_moments

PROPOSAL_VERSION = "1.0"
PROPOSAL_TYPES = ("rules-edit", "hook-change", "roster-note")
HUNK_ACTIONS = ("add", "strike", "rewrite")
REDO_THRESHOLD = 2
RESCUE_THRESHOLD = 2


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
    rescues_by_tool = {}
    unattributed_rescues = 0
    seen_digests = set()
    for _, tape, digest in tapes:
        if digest in seen_digests:
            continue
        seen_digests.add(digest)
        for moment in redo_moments(tape):
            by_tool.setdefault(moment["tool"], []).append(
                (digest, moment))
        for moment in rescue_moments(tape):
            if moment["tool"] is None:
                unattributed_rescues += 1
            else:
                rescues_by_tool.setdefault(
                    moment["tool"], []).append((digest, moment))
    tools = sorted(set(by_tool) | set(rescues_by_tool),
                   key=lambda name: (name is None, name))
    proposals = []

    def emit(proposal):
        if any(other["id"] == proposal["id"]
               for other in proposals):
            suffix = proposal["registry"][
                "experiment_id"][len("exp-"):][:8]
            proposal["id"] = f"{proposal['id']}-{suffix}"
        proposals.append(proposal)

    for tool in tools:
        moments = by_tool.get(tool, [])
        if len(moments) >= REDO_THRESHOLD:
            citations = [{"tape": digest, "seq": moment["seq"],
                          "kind": "tool_call"}
                         for digest, moment in moments]
            emit(_proposal(
                f"rules-edit-{tool}", target,
                (f"Before calling {tool} again after a "
                 f"failure, stop and read the error, then "
                 f"change the input or the approach. This "
                 f"run repeated a failed {tool} call "
                 f"{len(moments)} times across the period."),
                (f"{tool} was called again with the same input "
                 f"after an error {len(moments)} times in the "
                 f"period, at the cited moments."),
                citations, period))
        found = rescues_by_tool.get(tool, [])
        if len(found) >= RESCUE_THRESHOLD:
            citations = [{"tape": digest, "seq": moment["seq"],
                          "kind": "message"}
                         for digest, moment in found]
            pid = f"rules-edit-rescue-{tool}"
            emit(_proposal(
                pid, target,
                (f"Before acting on {tool} output, state what "
                 f"it shows and what happens next, so the "
                 f"operator can redirect early. In this "
                 f"period the operator's turn followed "
                 f"{tool} work {len(found)} times, each time "
                 f"after the agent had acted."),
                (f"The operator's turn followed {tool} work "
                 f"{len(found)} times in the period, each "
                 f"time after the agent had acted, at the "
                 f"cited moments."),
                citations, period))
    document = {"proposal_version": PROPOSAL_VERSION,
                "period": period,
                "proposals": proposals}
    if unattributed_rescues:
        document["unattributed_rescues"] = unattributed_rescues
    return document


def _proposal(pid, target, hunk_text, rationale, citations,
              period):
    proposal = {
        "id": pid,
        "type": "rules-edit",
        "target": target,
        "hunk": {"action": "add", "text": hunk_text},
        "rationale": rationale,
        "citations": citations,
    }
    proposal["registry"] = {
        "experiment_id": "exp-" + digest_of(
            {key: proposal[key]
             for key in ("type", "target", "hunk", "rationale",
                         "citations")}
            | {"baseline_period": period})[:16],
        "baseline_period": period,
    }
    return proposal


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
    if "unattributed_rescues" in document:
        count = document["unattributed_rescues"]
        if not isinstance(count, int) \
                or isinstance(count, bool) or count < 0:
            defects.append(
                "unattributed_rescues is not a non-negative "
                "integer")
    period = document.get("period")
    if not isinstance(period, dict):
        defects.append("period is not an object")
    else:
        for key in ("from", "to"):
            if key not in period or not (
                    period[key] is None
                    or isinstance(period[key], str)):
                defects.append(
                    f"period.{key} must be a string or null")
    proposals = document.get("proposals")
    if not isinstance(proposals, list):
        defects.append("proposals is not an array")
        return defects
    seen_ids = set()
    for index, proposal in enumerate(proposals):
        where = f"proposals[{index}]"
        if not isinstance(proposal, dict):
            defects.append(f"{where} is not an object")
            continue
        for field in ("id", "type", "target", "hunk",
                      "rationale", "citations", "registry"):
            if field not in proposal:
                defects.append(f"{where} is missing {field}")
        for field in ("id", "target", "rationale"):
            if field in proposal and (
                    not isinstance(proposal[field], str)
                    or not proposal[field]):
                defects.append(
                    f"{where}.{field} must be a non-empty string")
        if isinstance(proposal.get("id"), str):
            if proposal["id"] in seen_ids:
                defects.append(
                    f"{where}.id {proposal['id']!r} is used twice")
            seen_ids.add(proposal["id"])
        registry = proposal.get("registry")
        if isinstance(registry, dict):
            experiment = registry.get("experiment_id")
            if not isinstance(experiment, str)                     or not experiment.startswith("exp-"):
                defects.append(
                    f"{where}.registry.experiment_id is not an "
                    f"experiment id")
            if not isinstance(registry.get("baseline_period"), dict):
                defects.append(
                    f"{where}.registry.baseline_period is not an "
                    f"object")
        elif "registry" in proposal:
            defects.append(f"{where}.registry is not an object")
        if proposal.get("type") not in PROPOSAL_TYPES:
            defects.append(f"{where}.type is not a proposal type")
        hunk = proposal.get("hunk")
        if isinstance(hunk, dict):
            if hunk.get("action") not in HUNK_ACTIONS:
                defects.append(
                    f"{where}.hunk.action is not a hunk action")
            if (not isinstance(hunk.get("text"), str)
                    or not hunk.get("text")):
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
    tape = (tapes_by_digest.get(digest)
            if isinstance(digest, str) else None)
    if tape is None:
        return [f"{where} names a tape outside the input set"]
    seq = citation.get("seq")
    events = tape["events"]
    if (not isinstance(seq, int) or isinstance(seq, bool)
            or not 0 <= seq < len(events)):
        return [f"{where} names no event in its tape"]
    if events[seq].get("kind") != citation.get("kind"):
        return [f"{where} states the wrong kind for its event"]
    if citation.get("kind") == "message":
        event = events[seq]
        if event.get("role") != "user" \
                or event.get("sidechain") is not False:
            return [f"{where} cites a message that is not a "
                    f"main-chain user turn"]
    return []
