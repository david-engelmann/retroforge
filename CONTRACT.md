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

## Round 3. The Rust port (RF-15 to RF-22)

- RF-15. The port reproduces the behavior of
  Rounds 1 and 2 exactly, the Python on main at
  09b4686 standing as the behavioral spec, its
  tests included. Every divergence found is
  repaired or named as a deviation in the round
  dossier; an unnamed divergence is a defect.
  Test, the ported suite and the differential
  harness of RF-21.
- RF-16. The tool is a library crate with a thin
  binary, the family shape. Tape JSON reading and
  the canonical value model come from the
  flighttape crate pinned at ba8d4b5. The format
  checks of tapes.py are this tool's own subset
  with its own phrases and stand in this crate
  over the crate's parsed values; they are not
  the flighttape validator, whose acceptance is
  the format's whole rule. There is no onrecord
  dependency: the digest resemblance makes no
  claim-store dependency. Test, Cargo.toml and
  the ported suite.
- RF-17. The loading discipline is probed at the
  shipped CLI before coding, per surface: the
  tape file read (bytes, strict UTF-8, the
  Python's named reasons for an unreadable file,
  a non-UTF-8 file, a non-JSON file, and a number
  too large to read), the JSON reader's
  acceptance where it differs from the Python's
  (integer literal bounds, depth, duplicate
  keys, constants), the structural checks in
  tapes.py's order with its exact phrases, the
  directory listing in filename order with the
  .json suffix filter, exclusion lines printed
  to stderr while the run proceeds, the no
  valid tapes refusal, and the refusal when the
  resolved output path is one of the input
  tapes. Depth decisions are made in parse
  order, never by a whole-file pre-check. Test,
  the differential harness.
- RF-18. Proposal and tape digests render
  canonical form 1 in a local digest module with
  CPython repr-exact digits over the crate's
  value model, the onrecord port's pattern,
  because experiment ids are behavior
  verdictloop consumes and the flighttape
  crate's renderer is ECMAScript-faithful and
  runs one digit long on exactly terminating
  expansions. A digest failure excludes the tape
  with the reason the CLI names. Test, the
  ported suite's id pins and the harness.
- RF-19. Rendering is byte-exact. The proposal
  document renders as json.dumps with an indent
  of two and keys sorted as the Python sorts
  strings, plus a trailing newline; floats
  render by CPython's repr. The output file
  write is atomic in the Python's mkstemp shape
  (a sibling temp file, replaced on success,
  unlinked on failure). Stdout writes follow the
  Python's stdio law, including the sink
  replacement after a failed stdout write so
  the shutdown flush cannot mask the refusal.
  Argparse help and error bytes are pinned from
  captures at COLUMNS=80. Test, the ported suite
  and the harness's help battery.
- RF-20. The Python tests port file for file
  (tests/test_proposals.py, tests/test_rescue.py),
  every fixture kept. Cargo tests are the
  primary suite. Test, cargo test.
- RF-21. A differential harness runs the whole
  command surface against the Python oracle:
  directory shapes, tape files sound and
  defective in every tapes.py phrase, digest
  failures, moment shapes crossing both
  thresholds, duplicate tapes under RF-14,
  proposal id collisions under RF-11a, output
  to stdout, to a fresh path, onto an input
  tape, and into failure, and the argument
  surface. Files are compared after every step,
  not only streams. Test, the harness run
  recorded in the dossier.
- RF-22. The round dossier carries the port
  report: port or rewrite with reasons, the
  Rust and Python line share measured on the
  landing tree, the bindings determination by
  consumer shape, and a complete non-Rust
  remainder register with a written
  justification per item. An unjustified
  non-Rust remainder is a defect.
- RF-23. Round 4 re-pins the flighttape crate
  from ba8d4b550850179722f48b8f9c909e199efe3548 to
  eebce0be75a9c2ffa534635e37160c7cbef5d96e, its
  Round 12 merge, so the whole line stands on one
  flighttape rev. The differential harness re-run
  is the proof of what moves: a control run
  against the re-pinned binary over the standing
  corpus, its failures enumerated before any
  harness edit, and no behavior outside the
  crate's timestamp grammar may move silently.
- RF-24. The output guard's resolver peels the
  last component of any kind, as secondbill's
  repair and wallofwrong's WW-25 do, and the
  harness gains the interleave cells on both
  guard sides (the owed file's item 2 shape).
- RF-25. The round probes the resolver's loop
  posture against the oracle and takes the
  outcome the oracle dictates: if the Python
  counts an unresolvable output as naming an
  input, the resolver becomes ELOOP-fallible in
  wallofwrong's WW-25a shape (the loop judged on
  the finished path's prefixes); if the Python
  crashes uncaught on a loop, the shape stands
  as the named crash class with harness cells
  asserting it. Either way the loop cells land
  in the harness in this round, and the dossier
  records which branch the oracle took.
- RF-26. The round's verification is the full
  local gate plus the differential harness at
  zero failures, RF-23 to RF-25 cells included.
  CodeRabbit's advisory pass on Round 3's PR 4
  posted no findings (zero entries on the review
  endpoints), so no disposition is owed from it.
