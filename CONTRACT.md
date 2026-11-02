# retroforge Contract, stub

The full Contract is the first build act of this repo phase. This
stub records the obligations the setup already knows it owes. Each
obligation becomes executable before any feature code that depends
on it lands.

- RF-1. Every proposal is one of three types, a rules file edit, a hook change, or a roster note. No other proposal type ships.
- RF-2. Every proposed hunk carries evidence links to tape moments, or the proposal does not ship.
- RF-3. Output is a pull request or a diff for a human. Nothing is auto applied to a live setup.
- RF-4. The first real run is local, on the operator fleet, reported as numbers only. Nothing real is committed to this repo.

## Round 1. Evidence cited proposals (RF-1 to RF-8)

- RF-1. A proposal file is a JSON object carrying a format
  version, the period it covers, and a list of proposals.
  Each proposal carries an id, a type (rules-edit,
  hook-change, or roster-note), the file it targets, a hunk
  (an action of add, strike, or rewrite, with the text), a
  rationale, and a list of citations. Test,
  tests/test_proposals.py.
- RF-2. Every proposal carries at least two citations, and
  every citation resolves: its tape digest names a tape in
  the input set under flighttape canonical form 1, and its
  seq names an event in that tape of the kind the citation
  states. A proposal file that breaks this rule is invalid,
  and the validator names each broken citation. This is
  obligation one; a proposal with no evidence does not
  ship. Test, tests/test_proposals.py.
- RF-3. Retroforge reads flighttape tapes at format
  version 1.0 only, never raw harness session files. A
  file in the input set that fails to load or validate as
  a tape is excluded with its reason named, in the rollup
  discipline, and the run proceeds on the tapes that
  remain; a run with no valid tapes produces no proposal
  file and exits 1. Test, tests/test_proposals.py.
- RF-4. A redo moment is a tool call whose tool and input
  match an earlier tool call in the same tape, with an
  error tool result answering the earlier call in between.
  The moment is recorded at the later call's seq. Test,
  tests/test_proposals.py.
- RF-5. When redo moments for the same tool reach two
  across the period, the run emits one rules-edit proposal
  for that tool, an added bullet naming the tool and the
  repeated failure, citing the redo moments for that tool.
  Below the threshold, no proposal is emitted for that
  tool. Test, tests/test_proposals.py.
- RF-6. Each proposal carries a registry stub, an
  experiment id derived deterministically from the
  proposal's own content and the baseline period it was
  measured against, so the later verdictloop join is a
  fill-in, not a redesign. Test, tests/test_proposals.py.
- RF-7. The proposal file is deterministic: the same
  input set and rules file produce byte-identical output,
  tapes read in filename order, proposals in a stated
  order. Test, tests/test_proposals.py.
- RF-8. Retroforge never writes to the setup tree it
  reads. Its only output is the proposal file, written to
  the path named on the command line or to stdout, and a
  write failure is a named error with exit 1. Gate
  evidence enters proposals only as citations behind the
  evidence pack adapter stub; no Gate Truth format is
  consumed in this round. Test, tests/test_proposals.py.

Advisory amendment recorded in the Round 1 landing. The
stub contract at the head of this file describes the
output as a pull request or a diff for a human. That is
the tool's end state, not this round's: RF-8 governs
Round 1, and the only output is the proposal file, which a
later round learns to open as a pull request. The stub
sentence is superseded for every round until that one.

## Round 2. Rescue moments (RF-9 to RF-13)

- RF-9. A rescue moment is a main-chain user message
  (kind message, role user, sidechain false) that is
  not the first main-chain user message in its tape
  and that arrives after at least one agent action
  since the previous main-chain user message, where
  an agent action is a main-chain assistant message
  or a main-chain tool call. A user message that
  follows another user message with no agent action
  between them is a continuation, not a rescue. The
  moment is recorded at the user message's seq. Test,
  tests/test_rescue.py.
- RF-10. A rescue moment is attributed to the tool of
  the most recent main-chain tool call before it in
  the same tape. A moment with no earlier main-chain
  tool call is unattributed: it is counted in the
  proposal file's unattributed_rescues field, which
  is present only when the count is positive, and it
  is never cited. Test, tests/test_rescue.py.
- RF-11. When attributed rescue moments for the same
  tool reach two across the period, the run emits one
  additional rules-edit proposal for that tool, with
  id rules-edit-rescue-<tool>, a hunk naming the tool
  and the number of times the operator stepped in,
  and citations to the rescue moments, each stating
  kind message. Below the threshold, no rescue
  proposal is emitted for that tool. The redo
  proposal's id and text are unchanged from Round 1.
  Test, tests/test_rescue.py.
- RF-12. Proposal order is by tool name; within one
  tool the redo proposal precedes the rescue
  proposal. The file stays deterministic under RF-7.
  Test, tests/test_rescue.py.
- RF-13. The proposal version stays 1.0; the
  unattributed_rescues field is additive and
  optional. When present it must be a non-negative
  integer, and the validator names any other value
  as a defect. Test, tests/test_rescue.py.

## Round 2 amendments, after the reviews (recorded before the repair code)

- RF-2a. A citation stating kind message resolves
  only when the event at its seq is a main-chain
  user message (role user, sidechain false). A
  citation to an assistant message or a sidechain
  message names a defect, the same as a wrong kind.
  Test, tests/test_rescue.py.
- RF-11a. Proposal ids are unique within the file by
  construction or not at all: when any proposal's
  id would equal an id already emitted in the same
  file (a tool named to collide, in either
  direction), the later proposal's id carries a
  suffix of a dash and the first eight characters
  of its own registry experiment id's digest, which
  is content derived and deterministic. Emitting a file its own
  validator refuses is a build defect, never a
  validation error at write time. Test,
  tests/test_rescue.py.
- RF-11b. The rescue hunk and rationale state the
  counted fact in its counted form: the operator's
  turn followed the tool's work, after the agent
  had acted. The stronger reading (the operator
  interrupted or corrected that work) is the
  design's signal name, not a claim the count
  makes. Test, tests/test_rescue.py.
- RF-14. The period's evidence is the set of
  distinct tapes. A tape whose canonical digest
  repeats an earlier tape in the input order adds
  no moments to either reader pass; the same tape
  filed twice is one tape. This repairs a Round 1
  seam the redo pass shared. Test,
  tests/test_rescue.py.
