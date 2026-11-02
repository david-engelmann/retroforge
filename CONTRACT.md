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
