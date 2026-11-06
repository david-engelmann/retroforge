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
  measured against, so the later touchstone join is a
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
  touchstone consumes and the flighttape
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
- RF-25a. Amendment, on the red team's Round 4
  Major: a dangling final component. When the
  output's final component is a symlink whose
  target does not exist, the resolver reads the
  target spelling and resolves it against the
  link's parent before the lexical finish, as
  realpath does; a dangling output whose target
  spelling lands on a tape is refused at the
  guard, and a dangling output naming no tape
  proceeds as before. The harness gains the
  sharp cell (the target spelling through a
  missing component onto a tape) and the agreeing
  cell (a dangling output naming no tape replaces
  the link on both sides).
- RF-27. Round 5 discharges the two probes filed
  in this line's Round 5 owed file: the dangling
  link chain and the deleted working directory,
  both at the output guard. The chain question:
  Round 4's dangling repair (RF-25a) splices a
  dangling final component once; bluepencil's
  Round 4 red team proved a chain of dangling
  links defeats a single splice and its repair
  iterated to the kernel's 40 link bound. This
  crate's oracle resolves chains fully (subject
  to its own walk), so the probe pass measures
  chain lengths at the guard (onto the tape
  input, away, and through a cancelled chain)
  on the oracle and the port and fixes the
  divergence set before any repair is written.
- RF-28. The deleted-cwd question: from a
  deleted working directory, a relative output
  spelling against this crate's oracle and
  port. The oracle crashes uncaught out of
  resolve failures on this surface already
  (RF-25's loop posture); whether its guard
  call raises the same way on a failed getcwd,
  and what the port's resolver does there, is
  measured cell by cell (relative onto,
  relative away, absolute onto, absolute away)
  before any repair is written. Repairs follow
  the measurements under new clauses if the
  probe set expands.
- RF-29. The differential harness gains the
  proven cells with control runs against the
  pre-repair binary failing exactly the new
  cells, and the standing harness stays clean.
- RF-30. README records 5 of 100 only in the
  closure commit written after this round's
  three reviews, fixes, and owner verification.
- RF-31. Amendment, on the Round 5 probe pass: the
  resolver becomes the interpreter's walk itself, a
  transcription of posixpath._joinrealpath with
  strict=False over raw bytes, as in unheard's
  UH-25 shape. It follows chains of dangling and
  sound links alike, splice by splice, so a chain
  whose final spelling lands on a tape is refused
  at the guard at every length the oracle resolves,
  and the single-splice dangling repair (RF-25a),
  the canonicalize peel, and the finished-path loop
  walk retire. A link whose target the walk cannot
  read fails the resolution, as the interpreter's
  uncaught raise does.
- RF-32. The walk carries the interpreter's
  recursion budget, measured at this CLI: a
  resolution nesting 990 splices resolves, and the
  991st splice raises RecursionError in the Python,
  in the dangling, existing-final, and prefix
  families alike; a splice call entered past depth
  990 fails the resolution here (the crash class).
  The depth counts nesting, not splices in total.
  The closing stat of the finished path still
  reports a loop as the crash class (RF-25).
- RF-33. A relative spelling whose working
  directory cannot be read fails the resolution
  (the crash class), on the output side and the
  tape side alike: the interpreter raises
  FileNotFoundError out of the abspath at the end
  of every relative resolve from a deleted working
  directory, and from such a directory no relative
  component can be stat'ed, so no relative walk
  completes there. Absolute spellings never read
  the working directory, and their deleted-cwd
  cells already agree.
- RF-34. The harness gains the proven cells: the
  chain boundaries (dangling onto at 2, 41, 990,
  and the 991 crash cell; dangling away at 42 and
  990; existing-final onto at 41; prefix onto and
  away at 41), the cancelled-loop final spelling,
  the dangling cycles, and the deleted-cwd launch
  family (relative onto and away with an absolute
  source, absolute onto and away, the ".." source
  family, and a stdout control), with control runs
  against the pre-repair binary failing exactly
  the divergence cells. The absolute-intermediate
  chain shape is probe-pinned instead: a harness
  seed cannot spell a per-side absolute target.
- RF-35. Amendment, on the Round 5 red team's
  Major M1: the write consumes the output spelling
  as pathlib's construction hands it to
  _write_atomically. Construction drops every
  empty component and every "." component,
  wherever it stands in the spelling; a ".."
  component and every other component stand
  whole; a leading pair of separators stands as
  the anchor and a longer leading run collapses
  to one separator; a spelling with no component
  left stands as "." (or as its anchor). The temp
  file is created in the constructed path's
  parent, its prefix is the constructed path's
  name and a dot, and the replace target is the
  constructed path itself, the parent and the
  name taken under pathlib's decomposition: the
  name is the last component, empty only for an
  anchor alone or "."; the parent is the spelling
  less its last component, "." for a lone relative
  component and the anchor for a lone anchored
  one. The guard's resolve and the cannot-write
  message keep the raw argv spelling, as the
  Python's do.
- RF-36. The class boundary, measured at the
  interpreter and end to end: a trailing separator
  in any count and trailing dot components in any
  mixture diverge before this repair and are the
  class ("out.json/", "out.json//", "out.json/.",
  "./out.json/", "sub/out.json/.", and a missing
  name under a trailing separator, "newdir/"). A
  trailing ".." does not drop: construction keeps
  it whole, so the write still names the parent
  directory and refuses as the raw spelling did;
  "..." and dotted names (".y", "..y", "y.") are
  ordinary components. Internal empty, dot, and
  dotdot components already agreed, the kernel
  absorbing them at the temp creation and the
  replace alike. A spelling of only separators
  and dots collapses as RF-35 states ("." , "..",
  "/", "./") and refuses the write.
- RF-37. The command's other pathlib
  constructions, read from the oracle's cli.py:
  the source directory at the head of _propose,
  the output resolve at the guard, and the tape
  joins. The directory construction changes no
  observable behavior beyond the empty spelling,
  whose "." mapping already stands in
  os_path_from_units and is pinned by the
  standing harness; every other source spelling
  reaches the same directory test and listing
  through the kernel, and the joins carry bare
  tape names with no tail to drop. The guard's
  resolve already agrees on the tailed spellings
  ("a.json/" refuses at the guard on both sides).
  No repair beyond RF-35.
- RF-38. The harness gains the tailed-output
  cells of RF-36: the trailing separator and dot
  forms, an overwrite of an existing proposal
  under a tail, a missing name under a tail, a
  directory under a tail, the collapsed
  spellings, and the boundary pins (a trailing
  dotdot, a dotdot followed by a name through an
  existing directory, "..." as a name, and the
  onto spellings the guard refuses), with control
  runs against the pre-repair binary failing
  exactly the new divergence cells and the pins
  passing as regression coverage.
- RF-39. Round 6 probes the temp placement law
  at this crate's write seam, the family
  secondbill's Round 5 measured and repaired
  under SB-32 and which this crate's seam
  predates. The oracle's write creates its temp
  through tempfile.mkstemp, which absolutizes
  the parent lexically (cwd join, then
  normpath, no link resolution); this crate's
  write creates its temp under the constructed
  parent as spelled. The probe set is the
  popped-link family: an output whose parent
  contains a symlink component that a following
  dotdot pops, across the existence boundary
  (the lexical parent exists as a directory,
  neither parent exists, the popped component
  is a real directory), in plain, tailed,
  absolute, deep, and escape-link spellings,
  each measured against the oracle before any
  repair is written. Repairs follow the
  measurements under new clauses; the harness
  gains the proven cells with controls against
  the pre-repair binary failing exactly the
  convicted cells, boundary pins ordered before
  convicted cells where a one-sided write could
  echo through later snapshots.
- RF-40. The CodeRabbit disposition owed from
  Round 5 discharges here on evidence: the
  Round 5 pull request carries zero inline
  findings on read-back (checked 2026-11-04 in
  the rotation-wide late sweep), so there is
  nothing to fix and the absence is the
  disposition.
- RF-41. README records 6 of 100 only in the
  closure commit written after this round's
  three reviews, fixes, and owner verification.
- RF-42. The temp placement law, measured in
  Round 6 against the oracle: the write's temp
  is created under the lexically absolutized
  parent of the constructed path. The absolutize
  step is mkstemp's own: the parent joined onto
  the working directory when it is relative,
  then normalized lexically in posixpath's
  normpath shape over the spelling's bytes
  (empty and "." components drop, ".." pops the
  component standing before it, whatever that
  component names; a pop at the root drops; a
  leading separator pair stands as the anchor).
  No link is resolved anywhere in that step.
  The rename target remains the constructed
  path itself, and the cannot-write message
  keeps the raw argv spelling, as RF-35 and
  RF-38 already hold. The convicted family is
  the popped link: an output whose constructed
  parent carries a symlink component that a
  following ".." pops, where the lexically
  popped parent does not exist (or is a file)
  while the physically resolved parent does.
  There the oracle's temp creation fails in the
  lexical parent and the write refuses, where
  the pre-repair port created its temp under
  the spelled parent, the kernel resolved the
  link, and the statement of proposals landed.
  RF-39's points about the seam are corrected
  for this family wherever they spoke of the
  spelled parent as the temp's home.
- RF-43. The boundary agreements, measured with
  the family: where the lexical parent exists
  as a directory the behavior is unchanged
  (both sides create the temp there or in the
  physical parent and agree in outcome; where
  the physical parent is then missing, the
  oracle fails at its replace and the port at
  its temp creation, and both refuse with the
  same message); where neither parent exists
  both refuse at temp creation; where the
  popped component is a real directory the
  lexical and physical pops coincide. An
  absolute parent never consults the working
  directory; a failed working-directory read
  under a relative parent fails the temp
  creation into the cannot-write refusal,
  which is mkstemp's own posture (its abspath
  raises there). That arm is unreachable behind
  the guard in this command, whose resolve of a
  relative output from a deleted working
  directory already takes the crash class under
  RF-33; it stands so the placement step is
  total, not as a reachable divergence.
- RF-44. The harness gains the popped-link
  cells: the boundary pins as their own case
  (neither parent, the lexical parent only,
  both parents) with the real-directory pop
  beside them, and one case per convicted shape
  (plain, the trailing separator, the trailing
  dot, the absolute spelling, the deep parent,
  the escape link, the target carrying its own
  dotdot, the lexical parent as a file, two
  links popped in sequence, and the overwrite
  of an existing output at the physical
  target), the absolute cells carried by an
  "abs" step form that substitutes the side's
  own sandbox path and normalizes it back out
  of the compared streams. Controls against the
  pre-repair binary fail exactly the convicted
  cases' write steps, and the pins pass the
  control as regression coverage.
