"""The retroforge command line.

One command in this round, propose: read every tape file in
a directory, find the redo moments, and write the proposal
file. Retroforge never writes to the setup tree it reads;
the proposal file, to the path named with -o or to stdout,
is its only output (RF-8). Read and write failures are
named errors with exit 1, never tracebacks.
"""
from __future__ import annotations

import argparse
import json
import os
import tempfile
import pathlib
import sys

from .digest import DigestError, digest_of
from .proposals import build_proposals, validate_proposals
from .tapes import FAILED, load_tape_file


def _write_atomically(path, text):
    """Write through a sibling temp file, replaced on success.

    A failed write leaves the earlier proposal intact
    instead of truncating it and writing half a file.
    """
    handle, temp_name = tempfile.mkstemp(
        dir=str(path.parent), prefix=path.name + ".", suffix=".tmp")
    try:
        with os.fdopen(handle, "w", encoding="utf-8") as stream:
            stream.write(text)
            stream.flush()
        os.replace(temp_name, path)
    except OSError:
        try:
            os.unlink(temp_name)
        except OSError:
            pass
        raise


def _propose(source_dir, output, target):
    directory = pathlib.Path(source_dir)
    if not directory.is_dir():
        print(f"error, not a directory, {source_dir}",
              file=sys.stderr)
        return 1
    tapes = []
    by_digest = {}
    try:
        entries = sorted(directory.iterdir(), key=lambda p: p.name)
    except OSError:
        print(f"error, cannot list the directory, {source_dir}",
              file=sys.stderr)
        return 1

    for path in entries:
        if path.suffix != ".json":
            continue
        tape, reason, defects = load_tape_file(path)
        if tape is FAILED:
            print(f"error, {path.name} is excluded, {reason}",
                  file=sys.stderr)
            for defect in defects:
                print(f"defect, {defect}", file=sys.stderr)
            continue
        try:
            digest = digest_of(tape)
        except DigestError as error:
            print(f"error, {path.name} is excluded, the tape "
                  f"cannot be digested, {error}", file=sys.stderr)
            continue
        tapes.append((path.name, tape, digest))
        by_digest[digest] = tape
    if not tapes:
        print("error, no valid tapes in the directory",
              file=sys.stderr)
        return 1
    if output:
        resolved = pathlib.Path(output).resolve()
        loaded = {(directory / name).resolve()
                  for name, _, _ in tapes}
        if resolved in loaded:
            print("error, the output path is one of the input "
                  "tapes, the run never writes over its own "
                  "evidence", file=sys.stderr)
            return 1
    document = build_proposals(tapes, target=target)
    defects = validate_proposals(document, by_digest)
    if defects:
        print("error, the proposal file does not validate",
              file=sys.stderr)
        for defect in defects:
            print(f"defect, {defect}", file=sys.stderr)
        return 1
    text = json.dumps(document, indent=2, sort_keys=True) + "\n"
    try:
        if output:
            _write_atomically(pathlib.Path(output), text)
        else:
            sys.stdout.write(text)
            sys.stdout.flush()
    except OSError:
        if output:
            print(f"error, cannot write the proposal file, {output}",
                  file=sys.stderr)
        else:
            print("error, cannot write the proposal file to stdout",
                  file=sys.stderr)
            # The broken stream would fail again at shutdown
            # and mask this refusal with exit 120 and an
            # ignored exception dump, so it is replaced with
            # a sink before returning.
            try:
                sys.stdout.close()
            except OSError:
                pass
            sys.stdout = open(os.devnull, "w", encoding="utf-8")
        return 1
    return 0


def main(argv=None):
    parser = argparse.ArgumentParser(
        prog="retroforge",
        description="Retrospectives that end in a pull request")
    sub = parser.add_subparsers(dest="command", required=True)
    propose = sub.add_parser(
        "propose", help="propose edits from a directory of tapes")
    propose.add_argument("source_dir")
    propose.add_argument("--target", default="AGENTS.md")
    propose.add_argument("-o", "--output")
    args = parser.parse_args(argv)
    if args.command == "propose":
        return _propose(args.source_dir, args.output, args.target)


if __name__ == "__main__":
    raise SystemExit(main())
