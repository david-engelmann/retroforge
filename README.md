# retroforge

The retrospective that ends in a pull request against your own agent setup, every proposed hunk annotated with the trace moments that justify it.

## What this is

Observability tools capture traces and stop at dashboards and scores, so the retro's real product, a proposed and reviewable change to the setup, never gets produced. Retroforge reads a period of flight tapes plus gate evidence from the ledger and opens one pull request proposing rules file edits, hook changes, and roster notes. Each proposal carries its evidence links or it does not ship. The boundary with its sibling is one sentence. Retroforge proposes the changes, verdictloop keeps the registry that scores whether they worked.

## Status

Round 3 is built and in review on its branch. Setup and Rounds
1 and 2 landed earlier in the programme; the build proceeds on
the flighttape formats only, with gate evidence behind the
adapter stub, under the programme's package A ruling.

## Programme status

- Closed perfection rounds, 5 of 100. A round counts only when a
  full cycle of work, audit, review, red team, and approval has
  closed.
- Round 1 adds the propose command. It reads a directory of
  flighttape tapes, names the files it excludes and why, finds
  redo moments where a failed call was repeated with the same
  input, and writes one proposal file whose rules edits cite
  the tape moments behind them, every citation resolved
  against the input set before anything is written.
  Obligations RF-1 to RF-8, met, with the reviews recorded
  with the programme. The build opened under the programme's
  package A ruling, on the flighttape formats only, with gate
  evidence behind the adapter stub.
- Round 2 adds the rescue pass. A rescue moment is a user turn
  that follows agent action since the previous user turn,
  attributed to the tool whose work it followed; two for one
  tool emit a second rules edit citing the operator turns.
  The run counts distinct tapes, proposal ids cannot collide,
  and message citations resolve only to main-chain user turns.
  Obligations RF-9 to RF-14, met, with the reviews recorded
  with the programme.
- Round 3 ports the tool to Rust. The Python of Rounds 1 and 2
  stands as the behavioral spec and leaves the tree; the crate
  reproduces it exactly, the proposal document rendering under
  the same escaping rules, the argument parser matching token
  by token, and the tape loader deciding its refusals in parse
  order. The tests ported file for file and a differential
  harness holds the port against the Python across the whole
  command surface. Obligations RF-15 to RF-22, met, with the
  reviews recorded with the programme.
- Local gate, python3 scripts/codepoint_scan.py, cargo fmt
  --check, cargo clippy --all-targets -- -D warnings, and
  cargo test. CI runs the same commands.

## Licence

MIT. See LICENSE.
