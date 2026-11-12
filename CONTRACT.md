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
  repair and tetlock's WW-25 do, and the
  harness gains the interleave cells on both
  guard sides (the owed file's item 2 shape).
- RF-25. The round probes the resolver's loop
  posture against the oracle and takes the
  outcome the oracle dictates: if the Python
  counts an unresolvable output as naming an
  input, the resolver becomes ELOOP-fallible in
  tetlock's WW-25a shape (the loop judged on
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
- RF-45. The CodeRabbit disposition owed from
  Round 6 discharges here on evidence: the
  Round 6 pull request carries zero inline
  findings and zero reviews on read-back, and
  its summary shell stands finalized as
  "Review failed, the pull request is closed"
  over a findings-free walkthrough, the
  failed-because-closed shape. There is
  nothing to fix and the absence is the
  disposition, as RF-40 held for Round 5.
- RF-46. The temp suffix measured, the width
  arm. This crate's generator is measured on
  its own facts, never inherited from a
  sibling's. The oracle's candidate is the
  name, a dot, the eight characters mkstemp
  draws, and ".tmp"; the port's candidate is
  the same shape, eight characters drawn from
  the same 36-symbol alphabet. The flip is
  the byte count 242 writes and 243 refuses
  on both sides, and it does not move with
  the pid: the same flip stands at ambient
  pids, at pid 1 in a pid namespace, and at a
  three-digit pid reached by burning pids in
  the namespace. The width is not a function
  of the pid in either implementation. A
  traced run shows the port's candidate
  opened with O_EXCL at mode 0600, the
  mkstemp posture. This arm is agreement,
  not a defect, and no repair is made.
- RF-47. The temp suffix measured, the byte
  counting arm. The candidate is assembled
  from raw bytes end to end on both sides,
  the oracle's name reaching its temp through
  its string's surrogateescape round trip and
  the port's through its byte helpers, with
  no lossy conversion on either path. A name
  carrying one raw byte flips at the same
  byte counts as a plain name, 242 writes and
  243 refuses, at ambient pids and at pid 1.
  A parent directory whose name carries a
  raw byte lands the write at the exactly
  spelled directory, and no sibling named
  with the replacement character appears.
  This arm is agreement, not a defect, and
  no repair is made.
- RF-48. The temp suffix measured, the
  determinism arm. The port's draw is a
  stream seeded from the sub-second wall
  clock crossed with the process id, so it
  is not a public function of the pid, and
  nothing about the next draw is enumerable
  from the pid alone. At a fixed pid, twenty
  runs drew twenty distinct first candidates
  on the port, as on the oracle; planting
  every observed candidate in the output
  directory did not block the next write on
  either side, each drawing fresh. The plant
  family that convicted the sibling
  generators whose draws were public
  functions of the pid does not arise here.
  This arm is agreement, not a defect, and
  no repair is made.
- RF-49. The README is brought to the fleet
  README standard in this round: the first
  screen in the standard's order, the lede
  identical to the repo description, one
  badge whose rendered state is verified
  against the workflow it reports, install
  and quickstart blocks that run verbatim,
  and usage carrying captured output. The
  description identity is recorded exactly:
  the repo description and the README lede
  are one line, and Cargo.toml carries no
  description field at all, so the package
  metadata side of the identity holds no
  third variant and nothing to align. The
  absence is recorded here, not repaired;
  this round's bound is the README only.
- RF-50. The harness gains the boundary
  battery as its own cases: plain names at
  241, 242, and 243 and a name carrying one
  raw byte at 242 and 243 at ambient pids,
  plain names at 242 and 243 under a pidns
  step form that runs each side as pid 1 in
  a pid namespace, and the raw-byte parent
  cell. The control against the pre-round
  binary passes the battery whole, as it
  must in a round that repairs nothing: the
  battery pins a measured agreement, as
  RF-43's pins did. A mutant whose draw is
  widened from eight characters to eighteen
  fails exactly the battery's write cells,
  proving the battery convicts the width it
  pins and is not vacuous.
- RF-51. README records 7 of 100 only in the
  closure commit written after this round's
  three reviews, fixes, and owner
  verification, as RF-41 held for Round 6.
- RF-50a. Amendment to RF-50, recorded after
  the runs it describes. The widened-draw
  mutant's failures number fourteen in all:
  five are stream mismatches at write steps,
  and nine are state mismatches at the state
  steps of the same cases, the echoes the
  differing write outcomes leave behind
  them, including at the state steps that
  follow refusal steps. All fourteen sit
  inside the four boundary cases the clause
  pins, and none sits outside them. RF-50's
  closing claim stands narrowed to this
  count: the battery convicts the width it
  pins at its write cells and at the state
  those cells leave, and nowhere else.
- RF-52. Amendment to RF-45 and RF-46,
  recorded after the spec review of this
  round. Two sentences are corrected by
  reference; no measured number changes.
  First, RF-46's alphabet sentence. The
  oracle's name sequence draws over 37
  symbols: the letters, the digits, and
  the underscore. The port's alphabet
  constant is the same 37-byte string,
  but the draw indexes it modulo 36
  (src/cli.rs), so the port never draws
  the underscore the oracle can. The red
  team's 200 draws at a fixed pid drew
  only [0-9a-z] on the port against
  [0-9a-z_] on the oracle. The pinned
  width law, exactly eight drawn
  characters and the flip at 242 writes
  and 243 refuses, is unaffected, and no
  battery cell observes alphabet
  membership. RF-46's identity sentence
  stands corrected to the effective
  alphabets stated here. Second, RF-45's
  quoted shell. The shell of record keeps
  its two parts distinct, the heading
  "Review failed" over the notice "The
  pull request is closed", above a
  findings-free walkthrough. RF-45's
  disposition stands in that two-part
  form, and the comma-joined quotation
  is replaced by this restatement.
- RF-53. The seed's pre-epoch fallback is
  named here as a standing deviation, not
  a repair. make_temp seeds its draw from
  the sub-second wall clock crossed with
  the process id, and when the clock
  cannot be read, as on a pre-epoch clock,
  the seed falls back to the constant
  0x9e3779b9 (src/cli.rs). Under that
  fallback every process draws the
  identical first candidate at any pid,
  the shape convicted as WW-57 in tetlock
  and cured as BP-47a in bluepencil. It is
  named rather than repaired here because
  this round touches no source, being a
  measured-agreement round; because the
  branch is unreachable on any post-1970
  clock; and because no measured cell in
  this round touches it. The BP-47a
  treatment, refusing the write when the
  clock cannot be read, is filed for the
  next source-touching round of this
  crate.
- RF-54. Two dispositions from the step 0
  review of this round's pull request,
  recorded after the three reviews closed.
  First, the boundary qualification,
  repairing a CodeRabbit finding. RF-46's
  flip, 242 writes and 243 refuses, is the
  value at a 255-byte component limit of a
  general form, the output filesystem's
  component limit minus the temp name's
  13-byte overhead, one byte for the dot,
  eight for the drawn characters, and four
  for ".tmp". On a filesystem whose
  component limit is smaller, the flip
  sits lower by the same arithmetic.
  RF-46's numbers stand as measured on the
  filesystems in play, and this clause
  states the form they instantiate.
  Second, the zero seed, named here with
  RF-53's filing and not repaired in this
  round, on a second CodeRabbit finding.
  The seed is the sub-second nanoseconds
  crossed with the process id shifted left
  17 (src/cli.rs), so for any given pid
  exactly one nanoseconds reading zeroes
  it, the pid 1 case being a reading of
  exactly 131072. A zero xorshift state
  never changes, so every one of the 64
  attempts draws the same candidate, and
  if that one name is planted in the
  output directory the write refuses
  though other names are free. The
  reachability is stated plainly, one
  exact nanoseconds value for a given pid,
  and no measured cell in this round has
  hit it. The oracle's draw carries no
  degenerate state. The cure joins RF-53's
  pre-epoch fallback in the same filing,
  the next source-touching round of this
  crate gives the draw a nonzero-state
  guard alongside the
  refuse-on-unreadable-clock treatment.
- RF-55. The dispositions of the last two
  pull requests, read back whole for this
  round, reviews and review bodies and
  issue comments included. The CI shaping
  pull request 10 merged first, and the
  Round 7 pull request 11 merged on top of
  it. On pull request 10, the step 0 shell
  stands in its completed walkthrough
  form, created about five minutes before
  the merge and last updated about a
  minute before it, carrying one findings
  review with one inline finding and one
  nitpick in the review body. The finding
  asks that merge-queue branches be
  excluded from the push trigger, since a
  merge group also fires a push run, and
  the nitpick asks for a pull-request
  gated check that the merge_group
  trigger stays. Both are disposed here
  and not repaired. The workflow is the
  fleet's uniform shaping step, merge
  queues cannot be enabled on user-owned
  repositories because the ruleset API
  rejects the merge_queue rule there, so
  no merge group can form and no duplicate
  run can occur, and the exclusion belongs
  to the fleet queue pass if queues ever
  open. The finding stands unanswered on
  its thread, and the answer is this item.
  On pull request 11, the step 0 shell
  stands in its final form as the
  rate-limit banner over a complete
  walkthrough of the round's commits,
  created about twenty-eight minutes
  before the merge and last updated about
  eight minutes before it. Its findings
  review carried seven inline findings,
  six valid and repaired in Round 7 and
  one dismissed, with six owner replies
  and six bot acknowledgments standing on
  the threads, and the landing comment
  after the merge counts the seven in
  that form. An eighth item stood in the
  findings review's body, under
  additional comments, and nothing in
  Round 7 disposed of it. The item, a
  minor claiming a test gap, holds that
  the name battery of plain ASCII names
  and a one-raw-byte name does not
  exercise conversion of a supplementary
  Unicode character, a UTF-16 surrogate
  pair, and that a regression encoding
  each half of a valid pair as a
  replacement character would pass the
  battery. This round measured the cell.
  A tape whose tool call input path
  carries U+1F600 and an output name
  carrying U+1F600 both ran on the port
  and on the oracle, and the two sides
  agree byte for byte, the proposal
  files identical at 1161 bytes and the
  created name the character's raw UTF-8
  on both sides, with no replacement
  character anywhere. The disposition is
  that the gap is closed by measurement,
  no defect stands in the conversion, and
  the harness gains a case pinning the
  pair conversion so the battery itself
  covers it from this round on.
- RF-56. RF-53 is adjudicated and
  repaired. The measurements, taken under
  a clock shim on this machine against
  the pre-round binary and the Python
  oracle, are these. The oracle lands on
  the pre-epoch cell and on the
  erroring-clock cell, exit 0 with the
  proposal file written, because its draw
  is mkstemp and carries no clock seed.
  The port on the pre-epoch cell draws
  the constant's stream whole, shown by
  planting all 64 names of the simulated
  stream from the constant 0x9e3779b9, at
  two ambient pids the write refused
  against the full plant, and unplanted
  the same cell lands. On the
  erroring-clock cell the port dies in
  the standard library's panic inside
  SystemTime::now, exit 1 with nothing
  written, where the oracle lands, and
  the pair is stated unsmoothed. The
  repair gives the seed a read that
  yields no seed when the duration since
  the epoch cannot be read, and make_temp
  then fails, which the standing write
  arm maps to the standing refusal, exit
  1 with the standing message. The
  refusal is a hardening past the oracle,
  stated openly and not as fidelity. The
  oracle lands on the cell and the port
  now refuses it, because the port's draw
  is clock-seeded by design and a
  computable constant fallback is the
  defect RF-53 filed. The erroring-clock
  cell is unchanged by the repair, since
  the panic fires inside the standard
  library before any crate code can
  refuse. The entropy shape settled at
  tetlock in WW-69 was weighed for this
  seam and not imported. This seam's
  filed cure is the refusal, the draw's
  ordinary stream stands on the measured
  agreement of RF-52 and is unchanged,
  and the degenerate states are closed by
  this item and RF-57 together.
- RF-57. RF-54's zero seed is adjudicated
  and repaired. The measurements, taken
  under the clock shim at pid 1 in a pid
  namespace with the nanoseconds reading
  pinned at 131072, are these. With the
  zero state's one candidate planted, the
  pre-round binary refuses the write with
  the standing message and the plant
  stands untouched, and with nothing
  planted the same cell lands, so every
  attempt draws the planted name and the
  state never moves. The oracle at the
  same cell lands, its draw carrying no
  such state. The repair guards the
  crossed seed. When the nanoseconds
  crossed with the shifted pid come to
  zero, the constant 0x9e3779b9 is
  substituted as the state, so the draw
  moves off the single candidate and the
  write lands on a drawn name. The
  substitution is not RF-53's fallback
  returning. The zero point is one exact
  reading for one pid. The substituted
  stream is the constant's stream, drawn
  identically by any process at its own
  zero point, and pids repeat across
  namespaces, so no privacy is claimed
  for the stream and the bound is the
  crossing pair alone, while the refusal
  of RF-56 is checked
  on the read before any crossing, so an
  unreadable clock refuses and only a
  readable clock can reach the guard. The
  seed factoring is pinned by exact
  values in the crate's tests. A missing
  reading yields no seed, the pid 1
  reading of 131072 yields the constant,
  and the reading one above it yields the
  crossed value 1.
- RF-58. Amendment to RF-56 and RF-57,
  recorded after the runs they describe.
  The standing differential harness gains
  the three clock cases and the two astral
  cases, five cases in all, and stands at
  113 cases and 239 steps. Against the
  repaired binary the harness passes
  whole, no failures. Against the
  pre-round binary it fails in exactly
  five places, three in the pre-epoch
  clock case, where the old binary lands
  and the repaired one refuses, and two
  in the zero seed case, where the old
  binary refuses against the planted sole
  candidate and the repaired one lands.
  The erroring clock case passes on both
  binaries, as RF-56 states it unchanged.
  No failure sits outside the two repaired
  cells, so the new cases convict exactly
  the states this round repaired and are
  not vacuous. The gate on the repaired
  tree runs 42 tests, the 39 of the
  pre-round tree and the three seed pins,
  with clippy at -D warnings clean and
  the formatter check clean.
- RF-59. README records 8 of 100 only in
  the closure commit written after this
  round's three reviews, fixes, and owner
  verification, as RF-51 held for Round 7.
- RF-60. The CodeRabbit review of this
  round's pull request (review 5817730249,
  posted after the three reviews) carried
  two nitpicks in its body and no inline
  findings. The first is repaired here.
  RF-57's rationale and the matching
  comment on the seed function claimed
  the substituted stream is a single
  process's stream because two live
  processes never share a pid. That claim
  does not hold. Pids repeat across
  namespaces, and every process at its
  own zero point draws the same constant
  stream. Both texts now state the stream
  is the constant's, shared by any
  process at its zero point, with no
  privacy claimed; the behavior is
  unchanged, the amendment touching prose
  and a comment only, and CI runs the
  full suite on the amended head. The
  second nitpick asks for a caller-level
  test that a missing seed refuses at a
  writable output path, inside cargo
  test. It is dismissed. The refusal at
  a writable path under a missing reading
  is covered where the clock can be
  governed, in the standing differential's
  clock cases and in this round's red
  team rig, both run at the gate; a crate
  test cannot set the process clock
  without a preload shim this suite does
  not carry, and the seed factoring's
  pins plus the one line flow through
  the standing refusal arm are the
  crate-level coverage.
- RF-61. The standing read-back of the
  last pull request, read whole for this
  round, reviews and review bodies and
  issue comments included. Pull request
  12 carries one review, no review
  comments, and four issue comments. The
  review (5817730249) carried two
  nitpicks, both disposed in RF-60, and
  no inline findings. One item stood
  undisposed. The review tool's Docstring
  Coverage check failed the pull request
  at 50.00 percent against its own 80.00
  percent threshold, over functions of
  cyclomatic complexity at least 2 that
  the diff touched, naming no function,
  and it stands in the summary comment's
  failed checks table and in the review's
  own failed list, while RF-60 and the
  landing comment count only the two
  nitpicks. It is dismissed here. The
  check names no function, this
  repository carries no docstring
  configuration or threshold of its own,
  and the check applies the review tool's
  default profile, so there is no rule of
  this repository for it to enforce. The
  remaining comments are the review
  trigger, the bot's auto-reply that the
  last commit was already reviewed, and
  the landing comment. Nothing else was
  owed.
- RF-62. The flighttape pin aligns to
  the fleet. The crate stood at eebce0be,
  flighttape's Round 12 merge, one rev
  behind the fleet's 74a83298, its Round
  13 merge, and the dependency stood
  twice, under [dependencies] and again
  under [dev-dependencies]. Cargo.toml
  and Cargo.lock now carry 74a83298 and
  the duplicate is removed, so the
  dependency stands once. The revs differ
  in the timestamp grammar (FT-60,
  FT-60a, FT-63, FT-64, FT-65, FT-68), the
  surface flighttape's own contract names
  for consumers to re-pin against, so the
  alignment was controlled before it was
  believed, on the method of RF-23. The
  standing differential harness ran
  against the re-pinned binary before any
  harness edit, over the same corpus the
  old pin passes: 113 cases, 239 steps,
  0 failures, the old pin's own numbers,
  and no harness edit followed. Nothing
  the older rev carried silently is lost,
  and no deviation step was needed,
  because no case this port exercises
  reaches the moved grammar shapes. The
  alignment and the dedup are contracted
  as hygiene, with the control run as the
  evidence. One run note: the box's file
  quota refused the harness's standing
  sandbox root through the whole round,
  so the harness ran whole from a copy
  whose only change relocates that root;
  the diff is one line, and the cases,
  steps, and comparisons are the standing
  ones.
- RF-63. The seed surface, measured
  fresh at this round's binary against
  the oracle, a battery of 13 cells. The
  RF-56 and RF-57 cells stand as
  contracted. Under the clock shim at
  pid 1, the zero reading's cell lands
  with the sole candidate planted and
  lands unplanted. At a reading one
  nanosecond before the epoch the port
  refuses with the standing message,
  planted with the constant's full
  64-name stream and unplanted alike,
  where the oracle lands. On the erroring
  clock the port dies in the standard
  library's panic inside SystemTime::now,
  exit 1 with nothing written, where the
  oracle lands, the pair stated unsmoothed
  as before. The band past the crossings
  is now measured at the command line,
  not only derived. The crossing reading
  for a pid is the pid shifted left 17,
  and a sub-second reading is at most
  999,999,999, so pid 7629, whose crossing
  reading is 999,948,288, is the last pid
  with a crossing and pid 7630 the first
  with none. Under a shim pinning the
  reading at 999,948,288, with the
  constant's full stream planted, the
  binary run at exactly pid 7629 refuses
  and unplanted lands, while at exactly
  pid 7630 and at pid 8000 the planted
  cell lands, each run's pid proven by
  the launcher's own report of the child
  it exec'd. The factoring gains two
  crate pins at the same reading, their
  expected values computed outside the
  crate: pid 7629 yields the constant
  0x9e3779b9, and pid 7630 yields the
  plain crossed value 393216.
- RF-64. The suffix surface, measured
  fresh at this round's binary against
  the oracle, a battery of 20 cells and
  400 draws. The width flip stands where
  RF-46 and RF-47 put it. Output names of
  241 and 242 bytes land and 243 refuses
  on both sides, at ambient pids and at
  pid 1 alike, and a name carrying one
  raw byte flips at the same counts on
  both sides. The alphabet law of RF-52
  stands re-measured. Of 200 first
  candidates drawn by the port under
  strace, none carries an underscore and
  the drawn set is exactly the 36 symbols
  0 to 9 and a to z; of 200 drawn by the
  oracle, 34 carry an underscore and the
  drawn set is all 37 symbols. The port's
  agreement with the oracle is agreement
  of shape, width, and refusal, with the
  one contracted alphabet difference
  standing as measured.
- RF-65. README records 9 of 100 only in
  the closure commit written after this
  round's three reviews, fixes, and owner
  verification, as RF-59 held for Round 8.
- RF-66. Round 10 first read PR 13
  whole, the Round 9 pull request, as
  RF-61 read PR 12. The census is 0
  reviews, 0 review comments, and 5
  issue comments, every body read to
  its end. The one review shell
  (comment 6756095036, opened
  2026-11-10T01:17:25Z) stands in an
  edited final form (updated
  2026-11-10T01:22:56Z), and this
  census states the shell as it
  finally stands. In that form it
  carries no actionable comments and
  no inline findings. Its walkthrough
  stands under the head 23a20ffe. Its
  pre-merge checks read 5 passed,
  among them Docstring Coverage in a
  Passed row of the skipped form, no
  eligible functions found in the
  changed files, 0 functions analyzed
  across 1 file, 3 skipped as
  unsupported. It records Merge Risk
  Minimal on that head, run
  configuration CHILL, and that no
  configuration file from this
  repository was applied. A path
  filter of the tool's own excludes
  Cargo.lock from its walkthrough.
  The remaining comments are the
  owner's trigger (6756100741), the
  bot's auto-reply (6756215562, the
  review action not completed because
  the last commit was already
  reviewed), the lane's disposition
  (6756236465, the shell read whole,
  nothing to fix and nothing to
  dismiss), and the owner's landing
  record (6757553671, merged as
  c9f95fdb, the merge tree equal to
  the reviewed head's tree, the three
  reviews clean, the standalone
  audit's fix 6 discharged). No Round
  9 disposition is falsified by
  anything in the record, and nothing
  in it stands undisposed.
- RF-67. The seed band edge was
  decided at the exact points, not
  only bracketed, at the shipped
  binary against the oracle's stream
  simulator, in one battery cycle of
  25 cells (the battery's parts A to
  D, shared with RF-68 and RF-69).
  The crossing reading for a pid is
  the pid shifted left 17, computed
  outside the crate. Pid 7628's own
  crossing is 999,817,216. Seated at
  pid 7628 by the burn launcher, its
  pid proven from the child's own
  report, the port refuses at that
  reading with the constant's full
  64-name stream planted and lands
  unplanted. At 999,948,288, pid
  7629's crossing, pid 7628 draws the
  stream of the crossed seed 131,072.
  It refuses with that stream planted
  and lands with the constant's
  stream planted. One nanosecond
  below the last crossing, at
  999,948,287, pid 7629 draws the
  crossed seed 262,143's stream. It
  refuses with that stream planted
  and lands with the constant's
  stream planted. One nanosecond
  above, at 999,948,289, pid 7629
  draws seed 1's stream, refusing
  with it planted and landing with
  the constant's stream planted. At
  the last crossing itself the port
  refuses planted and lands free, as
  RF-63 held. Pid 7630 at 999,948,288
  lands with the constant's stream
  planted, as RF-63 held, and at the
  maximum sub-second reading
  999,999,999 pid 7630 draws the
  crossed seed 444,927's stream,
  refusing with it planted and
  landing with the constant's stream
  planted. Four pins join the crate's
  seed tests on these points, in
  RF-57's form. The reading and pid
  pairs are (999,817,216, 7628) to the
  constant, (999,948,287, 7629) to
  262,143, (999,948,289, 7629) to 1,
  and (999,999,999, 7630) to 444,927.
- RF-68. The zero-crossing guard and
  the clock refusals were re-verified
  at their boundaries in the same
  battery cycle (parts A and B). At
  pid 1, the binary run as the
  namespace init, reading 131,072
  refuses with the constant's full
  stream planted, so the stream the
  guard substitutes is the constant's
  own stream, and lands with only the
  zero state's sole candidate
  planted, so the draw has moved off
  the zero state. Reading 131,073
  lands with the constant's stream
  planted and refuses with seed 1's
  stream planted, so the guard holds
  at the crossing and releases one
  nanosecond above it. Under a
  pre-epoch clock the port refuses
  planted and unplanted, exit 1, no
  output file, no temp residue beyond
  the plants, with the refusal line
  "error, cannot write the proposal
  file, out.json" on stderr. The
  oracle under the same pre-epoch
  clock lands. Under an erroring
  clock the port dies in the standard
  library's panic inside
  SystemTime::now, the panic naming
  library/std/src/sys/pal/unix/
  time.rs and an unwrap on an Err
  value, Os code 22, kind
  InvalidInput, its thread identifier
  per-run, exit 1, nothing written,
  and the oracle lands. RF-56 and
  RF-57 stand as contracted.
- RF-69. The flighttape pin and the
  evidence adapter were re-verified.
  The pinned commit
  74a83298aeb31c82e62b9fb0c613f5bc8600799e
  has tree
  afd4d7d13520411e003762548fc79082a1b9cf6e
  by the API, 50 blobs, and the local
  cargo checkout at that revision
  matches all 50 blob for blob.
  Cargo.toml's rev and Cargo.lock's
  single flighttape entry both carry
  the full pinned revision. Propose
  over tests/fixtures/
  tape_synthetic.json reproduces the
  README's captured output byte for
  byte on the port and on the oracle
  (battery part D). The adapter's
  gate_citations refuses by name on
  both implementations with the
  message "the evidence pack format
  is not pinned yet, no gate evidence
  can be cited", the port's called
  directly through a throwaway
  integration test run once and
  removed before this commit, the
  oracle's called directly in Python.
  A proposal document rendered over a
  tape with two redo moments carries
  tape citations only. The word gate
  occurs zero times in it. The
  oracle's identity was verified
  before any cell was trusted. Its
  src tree matches commit 09b4686's
  src tree blob for blob, 7 files,
  read from the local clone's history
  (RF-15's record). The API answers
  422 for that short identifier. It
  is a local-history identifier.
- RF-70. A realistic pass exercised
  the forge over synthetic tapes at
  flighttape's volume shapes, the
  whole period compared against the
  oracle on every cell. The period
  holds 29 tape files and 3,651,298
  bytes. Its 26 valid tapes carry
  21,482 events, counting the
  duplicate file's 2,400 events as
  they stand in that file. A
  long-haul tape of 7,500 events. A
  redo storm of 2,400 events and its
  byte-identical duplicate under a
  second name (RF-14's one tape). A
  wide and deep tape with a 400-key
  input object, an input nested 120
  deep, a result text of 180,000
  characters, and float edge values
  (5e-324, 1.7976931348623157e308,
  -0.0) in an input. Three
  rescue-heavy tapes, sixteen
  ordinary tapes, an empty tape, a
  failed-outcome tape, and a
  sidechain-only tape. Three
  defective tapes stand mixed into
  the directory (invalid JSON, a
  tape_version of 2.0, and an input
  nested 10,005 deep, past the
  format's 9,994 depth cap), each
  excluded with its named reason on
  stderr, identically on both sides.
  Nine command cells ran on each
  side in a fresh copy of the
  period, outputs compared byte for
  byte or exit for exit, 18 runs in
  all. Propose to stdout and propose
  -o render the same 560,736-byte
  proposal document. Propose with
  --target TEAM-RULES.md renders
  560,764 bytes. Propose onto an
  input tape is refused at the guard
  (exit 1), a nonexistent directory
  is refused with the
  no-such-directory line (exit 1),
  an empty directory refuses with
  the no-valid-tapes line (exit 1),
  an output under a missing parent
  refuses (exit 1), an output name
  of 242 bytes lands with bytes
  identical to the -o cell, and an
  output name of 243 bytes refuses
  (exit 1). Every cell agreed.
- RF-71. The gate on this round's
  trees, all runs offline with an
  isolated target directory and two
  build jobs. On the base tree
  c9f95fdb, cargo test passed 44 (5
  library, 27 proposals, 12 rescue),
  clippy with warnings denied was
  clean, fmt was clean, and the
  codepoint scan read 0 and 0. The
  differential harness over the base
  binary reproduced 113 cases, 239
  steps, 0 failures, its sandbox
  root relocated to /tmp by the
  one-line in-memory substitution
  disclosed in RF-62. The battery of
  RF-67 to RF-69 ran 25 cells in one
  cycle with every cell as expected.
  The realistic pass of RF-70 ran 9
  cells, 18 runs, in one cycle with
  every cell equal. On the slice
  tree, cargo test passed 48 (the
  four new seed pins among the 9
  library tests), clippy with
  warnings denied was clean, fmt was
  clean, and the codepoint scan read
  0 and 0. The closure commit
  carrying this item changes no
  code. It carries RF-71, RF-72, and
  the README Status counts. The
  lane's worktree and instruments
  were staged under /tmp this round
  because the workspace file-count
  cap refused new workspace files
  and the box write fault refused
  process writes in long waves. The
  staging is disclosed on the lane
  report's face, and workspace
  copies are owed when a window
  opens. Nothing in the wait is
  evidence about the code.
- RF-72. Closure. The README records
  10 of 100 in the commit carrying
  this item. This round's read-back,
  measurements, pins, and realistic
  pass are the whole of the change.
- RF-73. Round 11 first read PR 14
  whole, the Round 10 pull request, as
  RF-66 read PR 13. The census is 0
  reviews, 0 review comments, and 4
  issue comments, every body read to
  its end. The one review shell
  (comment 6784269135, opened
  2026-11-11T01:28:16Z) stands in an
  edited final form (updated
  2026-11-11T04:47:25Z), and this
  census states the shell as it
  finally stands. In that form it
  carries no actionable comments and
  no inline findings. Its walkthrough
  stands under the head 76430df5. Its
  pre-merge checks read 5 passed,
  among them Docstring Coverage in a
  Passed row of the skipped form, no
  eligible functions found in the
  changed files, 0 functions analyzed
  across 1 file, 2 skipped as
  unsupported. It records Merge Risk
  Minimal on that head, run
  configuration CHILL, run
  99201502-bd7a-40ed-8987-bd8d691ce0bb,
  and that no configuration file from
  this repository was applied. A path
  filter of the tool's own excludes
  Cargo.lock from its walkthrough.
  The remaining comments are the
  owner's trigger (6784271405, at
  01:28:24Z), the bot's auto-reply
  (6784373532, at 01:34:39Z, the
  review action not completed because
  the last commit was already
  reviewed), and the owner's landing
  record (6788791953, at 06:26:50Z,
  merged as 40233bec, the merge tree
  equal to the reviewed head's tree).
  Every Round 10 disposition is
  graded against this record. The
  step 0 shell generated no
  actionable comments, so nothing
  stood to fix or dismiss from it,
  and its dispositions were carried
  to this read-back under the line's
  standing rhythm, as the landing
  record states. The standards
  review stands APPROVE with zero
  findings. The spec fidelity review
  returned NOT PASS with one Minor,
  against the pull request body's
  verified block (it claimed the
  cargo gate on the closure tree and
  the differential repeated against
  the closure-built binary, where
  RF-71 records the cargo gate on the
  base and slice trees and the
  differential over the base binary);
  the owner narrowed the two phrases
  in the body to exactly what RF-71
  records, body only, no tree
  changed, and the repaired body
  stands in the record in that form.
  The red team's first pass returned
  NOT APPROVE on coverage grounds
  with zero findings, and a finisher
  lane discharged the three unrun
  surfaces by execution and returned
  APPROVE with zero findings. Round
  10's spec and standards report
  files were lost to a /tmp wipe;
  their verdicts stand in the round's
  dossier and the landing record, and
  this read-back grades what the
  record shows and claims nothing for
  the lost files. No Round 10
  disposition is falsified by
  anything in the record, and nothing
  in it stands undisposed.
- RF-74. The draw stream, the seed
  band edge, and the clock refusals
  were re-measured fresh at the
  shipped binary against the oracle's
  stream simulator, in one battery
  cycle of 22 cells (this round's
  battery part A). The seed is the
  sub-second reading crossed with the
  pid shifted left 17, a crossed
  value of zero substituting the
  constant 0x9e3779b9, with refusal
  at a pre-epoch clock and the
  erroring-clock panic in
  SystemTime::now (RF-56, RF-57); no
  privacy is claimed for the draw.
  The oracle's identity was verified
  before any cell was trusted: its
  copy matches commit
  09b4686518c643fad55398cd6d42ddac299393dc
  blob for blob across all 14 of its
  files, verified against the local
  clone that holds that commit (the
  API answers 422 for the short
  identifier, as RF-69 records).
  Seated at exact pids by the burn
  launcher, each run's pid proven
  from the child's own report, with
  the clock shim pinning the reading:
  pid 7628 at its own crossing
  999,817,216 refuses with the
  constant's full 64-name stream
  planted and lands unplanted; pid
  7628 at 999,948,288 draws crossed
  seed 131,072, refusing with that
  stream planted and landing with
  the constant's stream planted; pid
  7629 one nanosecond below the last
  crossing, at 999,948,287, draws
  262,143 and refuses with its stream
  planted, landing with the
  constant's; pid 7629 at the last
  crossing 999,948,288 refuses with
  the constant's stream planted and
  lands unplanted; pid 7629 one
  nanosecond above, at 999,948,289,
  draws seed 1 and refuses with its
  stream planted, landing with the
  constant's; pid 7630 at 999,948,288
  lands with the constant's stream
  planted (its crossed seed is
  393,216, not the constant); pid
  7630 at the maximum reading
  999,999,999 draws 444,927 and
  refuses with its stream planted,
  landing with the constant's. At pid
  1, the binary run as the namespace
  init, reading 131,072 refuses with
  the constant's full stream planted
  and lands with only the zero
  state's sole candidate aaaaaaaa
  planted, and reading 131,073 lands
  with the constant's stream planted
  and refuses with seed 1's stream
  planted, so the guard holds at the
  crossing and releases one
  nanosecond above it. Under a
  pre-epoch clock the port refuses
  planted and unplanted, exit 1, no
  output file, with the line "error,
  cannot write the proposal file,
  out.json" on stderr, and the oracle
  lands. Under an erroring clock the
  port dies in the standard library's
  panic inside SystemTime::now, the
  panic naming library/std/src/sys/
  pal/unix/time.rs and an unwrap on
  an Err value, Os code 22, kind
  InvalidInput, its thread identifier
  per-run, exit 1, nothing written,
  and the oracle lands. RF-67 and
  RF-68 stand as contracted at every
  point re-measured.
- RF-75. The output guard's resolver
  and walk were re-measured fresh at
  the shipped binary against the
  oracle, at this CLI's own measured
  constants (the walk depth 990 of
  RF-32, never inherited from another
  repo), in one battery of 32 cells
  (this round's battery part B). A
  chain of N links plus its final
  spelling makes N plus 1 splices.
  In every chain cell the pre-existing
  output link stands in the source
  directory and is excluded as an
  unreadable tape, with the same
  exclusion line on both sides. The
  dangling onto chain (the output
  name links through c1 to cN, whose
  target spelling missing/../a.json
  resolves onto the tape) refuses at
  the guard on both sides at 2 links,
  at 41 links, and at 989 links (990
  splices, the deepest resolution the
  walk completes). At 990 links (991
  splices) and at 991 links the cell
  is the crash class on both sides:
  the oracle dies uncaught with
  RecursionError, and the port exits
  1 silently with nothing written,
  its failed resolution returning no
  path. The dangling away chain (the
  final spelling missing/../dest.json,
  dest.json not a tape) lands on both
  sides at 42 links and at 989 links,
  the output link replaced by the
  1,161-byte proposal document and
  dest.json never created, and is the
  crash class on both sides at 990
  links. The existing-final onto
  chain (cN naming a.json directly)
  refuses at the guard on both sides
  at 41 links. The prefix chain (the
  output spelling p1/<name>, p1
  through p41 resolving to the
  sandbox directory itself) refuses
  at the guard on both sides at 41
  links when the name is a.json, and
  when the name is out.json both
  sides pass the guard and refuse at
  the write with the cannot-write
  line, the kernel's own 40-link
  bound met in the write's path
  resolution. An output name that is
  a symlink to itself is the crash
  class on both sides: the oracle
  dies uncaught with a RuntimeError
  naming the symlink loop, and the
  port exits 1 silently. From a
  deleted working directory, with an
  absolute source, a relative output
  is the crash class on both sides
  onto a tape and away alike (the
  oracle dies uncaught with
  FileNotFoundError, the port exits
  1 silently), an absolute output
  onto a tape refuses at the guard
  on both sides, and an absolute
  output away lands on both sides
  with the 1,161-byte document.
  RF-31, RF-32, and RF-33 stand as
  contracted at every point
  re-measured.
- RF-76. The flighttape pin and the
  evidence adapter were re-verified.
  The pinned commit
  74a83298aeb31c82e62b9fb0c613f5bc8600799e
  has tree
  afd4d7d13520411e003762548fc79082a1b9cf6e
  by the API, exactly 50 blobs, and
  the local cargo checkout at that
  revision matches all 50 blob for
  blob. Cargo.toml's rev and
  Cargo.lock's single flighttape
  entry both carry the full pinned
  revision (RF-62's single entry
  stands). In a scratch copy of the
  pinned tree the tree's own
  digest_cli suite passes 5 of 5,
  executed this round, including
  cli_digest_prints_exactly_the_digest:
  the built flighttape binary prints
  exactly
  cc9c4e01fca418dcc33114caf75fae1f6bf0d401f297b5d61f6776903fa955a3
  for the golden fixture, and
  cli_verify_accepts_and_rejects
  passes with it. The adapter's
  gate_citations refuses by name on
  both implementations with the
  message "the evidence pack format
  is not pinned yet, no gate evidence
  can be cited", the port's called
  directly through a throwaway
  integration test run once and
  removed before this commit, the
  oracle's called directly in Python,
  on a real-looking path and on an
  empty path alike on both sides.
  Propose over tests/fixtures/
  tape_synthetic.json reproduces the
  README's captured output byte for
  byte on the port and on the oracle
  (the 147-byte document, in the
  realistic pass of RF-77). The
  README Status sentence stands as
  contracted: the evidence pack
  format is not pinned for this line
  and the adapter refuses by name
  until it is.
