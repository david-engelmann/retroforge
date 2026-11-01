# retroforge

The retrospective that ends in a pull request against your own agent setup, every proposed hunk annotated with the trace moments that justify it.

## What this is

Observability tools capture traces and stop at dashboards and scores, so the retro's real product, a proposed and reviewable change to the setup, never gets produced. Retroforge reads a period of flight tapes plus gate evidence from the ledger and opens one pull request proposing rules file edits, hook changes, and roster notes. Each proposal carries its evidence links or it does not ship. The boundary with its sibling is one sentence. Retroforge proposes the changes, verdictloop keeps the registry that scores whether they worked.

## Status

Seeded in October 2026. Local setup is in progress on the setup
branch. The remote seeding pass is waiting on a token access fix, so
work is banked locally until pushes clear. This repo is in phase 3
of the Retrospective Line and its build waits on the phase gate, the
main chat confirmation that the Gate Truth evidence pack schema has
merged and passed acceptance. Design work may proceed, build does
not start before the gate.

## Programme status

- Closed perfection rounds, 0 of 100. A round counts only when a
  full cycle of work, audit, review, red team, and approval has
  closed.
- Contract, stub in place, not yet met.
- Local gate, python3 scripts/codepoint_scan.py and, once tests
  exist, python3 -m pytest tests -q. CI runs the same commands.

## Licence

MIT. See LICENSE.
