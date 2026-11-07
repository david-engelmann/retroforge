# retroforge

The retrospective that ends in a pull request against your own agent setup, every proposed hunk annotated with the trace moments that justify it.

Retroforge reads a directory of flighttape tapes from a period of agent runs and turns what went wrong into a proposal file a human can review. It finds the moments where a failed tool call was repeated with the same input, and the moments where the operator had to step in, and it writes rules file edits, hook changes, and roster notes that cite the tape moments behind them. Nothing is applied on its own. The proposal file is the product, and a person decides what lands. Retroforge proposes the changes, and touchstone keeps the registry that scores whether they worked.

[![ci](https://github.com/david-engelmann/retroforge/actions/workflows/ci.yml/badge.svg)](https://github.com/david-engelmann/retroforge/actions/workflows/ci.yml)

```console
$ mkdir tapes
$ cp tests/fixtures/tape_synthetic.json tapes/
$ retroforge propose tapes
{
  "period": {
    "from": "2026-01-01T09:00:00.000Z",
    "to": "2026-01-01T09:00:00.000Z"
  },
  "proposal_version": "1.0",
  "proposals": []
}
```

The fixture tape carries no repeated failure, so its proposal list is empty. The quickstart builds a tape that does.

## Install

A Rust toolchain and Git are the prerequisites. Clone the repo and install the command from it.

```console
$ git clone https://github.com/david-engelmann/retroforge
$ cd retroforge
$ cargo install --path .
```

The crate is named retroforge. It reads flighttape tapes through the flighttape crate at a pinned revision, and cargo fetches that dependency during the install. The installed command lands in Cargo's bin directory, $HOME/.cargo/bin by default, and the quickstart below assumes that directory is on the PATH.

## Quickstart

Write one tape holding a repeated failure, then run the command over the directory that holds it.

```console
$ mkdir tapes
$ cat > tapes/run1.json <<'EOF'
{
  "tape_version": "1.0",
  "run": {"harness": "claude-code", "session_id": "s1", "started_at": "2026-01-05T09:00:00.000Z"},
  "events": [
    {"seq": 0, "kind": "tool_call", "sidechain": false, "timestamp": "2026-01-05T09:00:00.000Z", "call_id": "c0", "tool": "Read", "input": {"path": "a.py"}},
    {"seq": 1, "kind": "tool_result", "sidechain": false, "timestamp": "2026-01-05T09:00:00.000Z", "call_id": "c0", "is_error": true, "text": "boom"},
    {"seq": 2, "kind": "tool_call", "sidechain": false, "timestamp": "2026-01-05T09:00:00.000Z", "call_id": "c1", "tool": "Read", "input": {"path": "a.py"}},
    {"seq": 3, "kind": "tool_result", "sidechain": false, "timestamp": "2026-01-05T09:00:00.000Z", "call_id": "c1", "is_error": true, "text": "boom"},
    {"seq": 4, "kind": "tool_call", "sidechain": false, "timestamp": "2026-01-05T09:00:00.000Z", "call_id": "cx", "tool": "Read", "input": {"path": "a.py"}}
  ],
  "outcome": {"status": "completed", "basis": "final-assistant-message"}
}
EOF
$ retroforge propose tapes -o proposals.json
$ cat proposals.json
{
  "period": {
    "from": "2026-01-05T09:00:00.000Z",
    "to": "2026-01-05T09:00:00.000Z"
  },
  "proposal_version": "1.0",
  "proposals": [
    {
      "citations": [
        {
          "kind": "tool_call",
          "seq": 2,
          "tape": "47f1b05b6206bff08e2a625368cb188491fa57593f680db250bbd6e6de407200"
        },
        {
          "kind": "tool_call",
          "seq": 4,
          "tape": "47f1b05b6206bff08e2a625368cb188491fa57593f680db250bbd6e6de407200"
        }
      ],
      "hunk": {
        "action": "add",
        "text": "Before calling Read again after a failure, stop and read the error, then change the input or the approach. This run repeated a failed Read call 2 times across the period."
      },
      "id": "rules-edit-Read",
      "rationale": "Read was called again with the same input after an error 2 times in the period, at the cited moments.",
      "registry": {
        "baseline_period": {
          "from": "2026-01-05T09:00:00.000Z",
          "to": "2026-01-05T09:00:00.000Z"
        },
        "experiment_id": "exp-00beff503e75f106"
      },
      "target": "AGENTS.md",
      "type": "rules-edit"
    }
  ]
}
```

Two failed Read calls with the same input, and a third call after them, cross the threshold of two. The proposal cites the later calls by their seq numbers on the tape digest, and its registry stub carries the experiment id the touchstone join later fills in.

## Usage

The command has one subcommand, propose. It takes the directory to read, and every tape file in it that fails to load is excluded with its reason named on stderr while the run proceeds on the tapes that remain. Without -o the proposal document goes to stdout. With -o it is written through a sibling temporary file and replaced into place, so a failed write leaves the earlier proposal file intact. A run with no valid tapes writes nothing and exits 1, and a directory that does not exist is refused by name. In the captured run below, the tapes directory also holds a file named broken.json whose contents are not valid JSON, which is why the run prints its exclusion line.

```console
$ retroforge propose tapes -o proposals.json
error, broken.json is excluded, the file is not valid JSON
```

The rules edits aim at AGENTS.md by default. The target is set with --target, and every proposal in the run carries it.

```console
$ retroforge propose tapes | grep '"target"'
      "target": "AGENTS.md",
$ retroforge propose tapes --target RULES.md | grep '"target"'
      "target": "RULES.md",
```

Every proposal carries at least two citations, and each citation is resolved against the input set before anything is written. Retroforge never modifies the files it reads, its guard refuses an output path that names one of the input tapes, and the output path itself is the caller's choice.

## Status

The propose command is complete at its contracted scope and runs as a Rust binary, with the Python of the first two rounds standing as its behavioral spec. Gate evidence citations are not wired yet, because the evidence pack format is not pinned for this line, and the adapter refuses by name until it is. Closed perfection rounds, 7 of 100. A round counts only when a full cycle of work, audit, review, red team, and approval has closed.

## License

MIT. See the LICENSE file.
