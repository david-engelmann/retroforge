//! The retroforge command line (RF-19).
//!
//! One command, propose: read every tape file in a
//! directory, find the redo and rescue moments, and
//! write the proposal file. Retroforge never writes
//! to the setup tree it reads; the proposal file,
//! to the path named with -o or to stdout, is its
//! only output (RF-8). Read and write failures are
//! named errors with exit 1, never tracebacks.
//!
//! The argument parser reproduces argparse's
//! observable behavior: its help and usage texts,
//! its error phrases, its exit code 2, prefix
//! matching on long options, and its stream
//! behavior. Arguments are carried as UTF-16 unit
//! strings (the Python's str under surrogateescape),
//! so a non-UTF-8 argument reaches the same code
//! points the Python would see.

use std::collections::HashMap;
use std::io::Write;
use std::path::{Path, PathBuf};

use flighttape::value::{Text, Value};

use crate::digest::{digest_of, Budget};
use crate::emit::{dumps_view, Mode};
use crate::helptext::{PROPOSE_HELP, PROPOSE_USAGE, TOP_HELP, TOP_USAGE};
use crate::proposals::{build_proposals, validate_proposals};
use crate::tapes::{load_tape_file, TapeLoad};
use crate::util::{closed_at_launch, fsdecode_units, os_path_from_units, scalars_of, Msg};

/// The tape digest render budget, probed at the
/// oracle for the load path (RF-17): an array
/// chain under run is digested at nesting 990
/// (deepest node 993) and refused at 991; an
/// object chain is digested at nesting 989
/// (deepest object 991) and refused at 990.
const TAPE_DIGEST_BUDGET: Budget = (993, 991);

pub fn run_cli() -> i32 {
    let stdout_closed = closed_at_launch(1);
    let stderr_closed = closed_at_launch(2);
    let raw: Vec<Vec<u16>> = std::env::args_os()
        .skip(1)
        .map(|arg| fsdecode_units(&os_bytes(&arg)))
        .collect();
    let code = run_with_args(&raw, stdout_closed, stderr_closed);
    let _ = std::io::stdout().flush();
    let _ = std::io::stderr().flush();
    code
}

#[cfg(unix)]
fn os_bytes(arg: &std::ffi::OsString) -> Vec<u8> {
    use std::os::unix::ffi::OsStrExt;
    arg.as_os_str().as_bytes().to_vec()
}

#[cfg(not(unix))]
fn os_bytes(arg: &std::ffi::OsString) -> Vec<u8> {
    arg.to_string_lossy().into_owned().into_bytes()
}

// The streams. A report the Python prints to a
// stream that was closed at launch migrates or
// fails as the closed-stream matrix the family
// ports established; the writers below carry it.

fn write_stdout(bytes: &[u8], stdout_closed: bool) -> bool {
    if stdout_closed {
        return false;
    }
    let mut out = std::io::stdout().lock();
    out.write_all(bytes).is_ok() && out.flush().is_ok()
}

fn write_stderr(bytes: &[u8], stderr_closed: bool, stdout_closed: bool) -> bool {
    if stderr_closed {
        // The Python's error reports go to the
        // surviving stream; in this port the caller
        // renders them to stdout instead.
        return write_stdout(bytes, stdout_closed);
    }
    let mut out = std::io::stderr().lock();
    out.write_all(bytes).is_ok() && out.flush().is_ok()
}

/// Print a line to stderr (the error channel), in
/// the Python's rendering there (backslashreplace).
fn error_line(msg: &Msg, stdout_closed: bool, stderr_closed: bool) {
    let mut text = msg.render_stderr();
    text.push('\n');
    if stderr_closed {
        // Migrated whole to stdout, rendered for
        // stdout there: the bytes the Python's
        // fallback print would write.
        let bytes = stdout_bytes(msg);
        let mut bytes = bytes;
        bytes.push(b'\n');
        write_stdout(&bytes, stdout_closed);
    } else {
        write_stderr(text.as_bytes(), false, stdout_closed);
    }
}

/// A Msg's stdout bytes: UTF-8 with lone units in
/// DC80..DCFF as their raw byte; any other lone
/// surrogate fails the whole render, as a Python
/// stdout write of the same text fails.
fn stdout_bytes(msg: &Msg) -> Vec<u8> {
    let units = &msg.0;
    let mut out = Vec::new();
    let mut index = 0;
    while index < units.len() {
        let unit = units[index];
        if (0xDC80..=0xDCFF).contains(&unit) {
            out.push((unit - 0xDC00) as u8);
            index += 1;
        } else if (0xD800..=0xDBFF).contains(&unit)
            && index + 1 < units.len()
            && (0xDC00..=0xDFFF).contains(&units[index + 1])
        {
            let scalar = crate::util::decode_pair(unit, units[index + 1]);
            let mut buf = [0u8; 4];
            out.extend_from_slice(
                char::from_u32(scalar)
                    .expect("a valid pair")
                    .encode_utf8(&mut buf)
                    .as_bytes(),
            );
            index += 2;
        } else if (0xD800..=0xDFFF).contains(&unit) {
            return Vec::new();
        } else {
            let mut buf = [0u8; 4];
            out.extend_from_slice(
                char::from_u32(unit as u32)
                    .expect("a BMP scalar")
                    .encode_utf8(&mut buf)
                    .as_bytes(),
            );
            index += 1;
        }
    }
    out
}

fn print_out(msg: &Msg, stdout_closed: bool, stderr_closed: bool) -> bool {
    let bytes = stdout_bytes(msg);
    if stdout_closed {
        return false;
    }
    let _ = stderr_closed;
    write_stdout(&bytes, false)
}

// The parser.

struct ParseFailure {
    prog: &'static str,
    usage: &'static str,
    message: Msg,
}

fn top_failure(message: Msg) -> ParseFailure {
    ParseFailure {
        prog: "retroforge",
        usage: TOP_USAGE,
        message,
    }
}

fn sub_failure(message: Msg) -> ParseFailure {
    ParseFailure {
        prog: "retroforge propose",
        usage: PROPOSE_USAGE,
        message,
    }
}

/// argparse's error exit: the usage and the error
/// line to stderr at exit 2, the error line naming
/// the parser that raised it (the subcommand's
/// errors carry its full name). When the usage
/// write itself fails the Python's error line is
/// lost in its fallback stream; the usage-collapse
/// shape is reproduced by exiting 1 in that case.
fn parse_error(failure: ParseFailure, stdout_closed: bool, stderr_closed: bool) -> i32 {
    let mut usage = failure.usage.to_string();
    usage.push('\n');
    if stderr_closed {
        // The usage migrates to stdout alone: the
        // Python's error line is lost in its fallback
        // stream, and the migrated write succeeds
        // into the runtime's substituted stream even
        // when stdout was itself closed at launch.
        // A genuine write failure collapses the exit
        // to 1 (the family's usage-collapse shape).
        let mut out = std::io::stdout().lock();
        let ok = out.write_all(usage.as_bytes()).is_ok() && out.flush().is_ok();
        return if ok { 2 } else { 1 };
    }
    let usage_ok = write_stderr(usage.as_bytes(), false, stdout_closed);
    if !usage_ok {
        return 1;
    }
    let mut line = Msg::str(&format!("{}: error: ", failure.prog));
    line.push_msg(&failure.message);
    error_line(&line, stdout_closed, stderr_closed);
    2
}

/// Print a help text at exit 0: to stdout, or to
/// stderr when stdout was closed at launch.
fn print_help(help: &'static str, stdout_closed: bool, stderr_closed: bool) -> i32 {
    let help = Msg::str(help);
    if stdout_closed {
        let text = help.render_stderr();
        write_stderr(text.as_bytes(), stderr_closed, true);
    } else {
        print_out(&help, false, stderr_closed);
    }
    0
}

fn units(text: &str) -> Vec<u16> {
    text.encode_utf16().collect()
}

fn units_eq(token: &[u16], text: &str) -> bool {
    token == units(text).as_slice()
}

/// argparse's negative number pattern over the
/// token: ^-\d+$ or ^-\d*\.\d+$ with Unicode decimal
/// digits. A token matching it is a value, not an
/// option.
fn looks_like_negative_number(token: &[u16]) -> bool {
    let scalars = scalars_of(token);
    if scalars.len() < 2 || scalars[0] != '-' as u32 {
        return false;
    }
    let rest = &scalars[1..];
    let all_digits =
        |s: &[u32]| !s.is_empty() && s.iter().all(|c| crate::util::decimal_digit(*c).is_some());
    if all_digits(rest) {
        return true;
    }
    if let Some(dot) = rest.iter().position(|c| *c == '.' as u32) {
        let (before, after) = (&rest[..dot], &rest[dot + 1..]);
        return (before.is_empty()
            || before
                .iter()
                .all(|c| crate::util::decimal_digit(*c).is_some()))
            && all_digits(after);
    }
    false
}

fn is_option_like(token: &[u16]) -> bool {
    token.len() > 1
        && token[0] == '-' as u16
        && !looks_like_negative_number(token)
        && token != units("--")
}

#[derive(Clone)]
struct Token {
    units: Vec<u16>,
}

fn tokens(raw: &[Vec<u16>]) -> Vec<Token> {
    raw.iter()
        .map(|units| Token {
            units: units.clone(),
        })
        .collect()
}

/// The one long option of the propose command that
/// takes a value, with argparse's prefix matching.
enum LongMatch {
    Target,
    Output,
    Help,
    Unknown,
}

fn match_long(name: &[u16]) -> LongMatch {
    let candidates: [(&str, LongMatch); 3] = [
        ("--target", LongMatch::Target),
        ("--output", LongMatch::Output),
        ("--help", LongMatch::Help),
    ];
    let mut found: Option<LongMatch> = None;
    for (text, which) in &candidates {
        let full = units(text);
        if name == full.as_slice() {
            return match which {
                LongMatch::Target => LongMatch::Target,
                LongMatch::Output => LongMatch::Output,
                LongMatch::Help => LongMatch::Help,
                LongMatch::Unknown => LongMatch::Unknown,
            };
        }
        if name.len() > 2 && full.starts_with(name) {
            if found.is_some() {
                return LongMatch::Unknown;
            }
            found = Some(match which {
                LongMatch::Target => LongMatch::Target,
                LongMatch::Output => LongMatch::Output,
                LongMatch::Help => LongMatch::Help,
                LongMatch::Unknown => LongMatch::Unknown,
            });
        }
    }
    found.unwrap_or(LongMatch::Unknown)
}

pub fn run_with_args(raw: &[Vec<u16>], stdout_closed: bool, stderr_closed: bool) -> i32 {
    let args = tokens(raw);
    let mut top_extras: Vec<Token> = Vec::new();
    let mut seen_separator = false;
    let mut index = 0;
    while index < args.len() {
        let token = &args[index];
        let u = &token.units;
        if !seen_separator && units_eq(u, "--") {
            seen_separator = true;
            index += 1;
            continue;
        }
        if !seen_separator {
            if let Some(code) = top_help_action(u, stdout_closed, stderr_closed) {
                return code;
            }
            if is_option_like(u) {
                top_extras.push(token.clone());
                index += 1;
                continue;
            }
        }
        // The first value token is the command.
        if units_eq(u, "propose") {
            return match parse_propose(&args[index + 1..], stdout_closed, stderr_closed) {
                SubOutcome::Done(code) => code,
                SubOutcome::Parsed(parsed, sub_extras) => {
                    let mut all = top_extras;
                    all.extend(sub_extras);
                    if !all.is_empty() {
                        return parse_error(
                            top_failure(unrecognized(&all)),
                            stdout_closed,
                            stderr_closed,
                        );
                    }
                    propose(&parsed, stdout_closed, stderr_closed)
                }
            };
        }
        let mut message = Msg::str("argument command: invalid choice: ");
        message.push_str(&crate::pyrepr::repr_text(&Text::from_units(u.clone())));
        message.push_str(" (choose from 'propose')");
        return parse_error(top_failure(message), stdout_closed, stderr_closed);
    }
    parse_error(
        top_failure(Msg::str("the following arguments are required: command")),
        stdout_closed,
        stderr_closed,
    )
}

/// A help action at the top level, when the token
/// is one: bare or clustered -h, --help in full or
/// by prefix. A glued value is the explicit
/// argument error. Help acts as its token is
/// reached, ahead of any later token.
fn top_help_action(token: &[u16], stdout_closed: bool, stderr_closed: bool) -> Option<i32> {
    if units_eq(token, "-h") {
        return Some(print_help(TOP_HELP, stdout_closed, stderr_closed));
    }
    if token.len() > 2 && token[0] == '-' as u16 && token[1] == '-' as u16 {
        let (name, glued) = split_glued(token);
        if matches!(match_long_top(name), LongMatch::Help) {
            return Some(match glued {
                Some(value) => help_explicit(
                    top_failure(Msg::new()),
                    "-h/--help",
                    &value,
                    stdout_closed,
                    stderr_closed,
                ),
                None => print_help(TOP_HELP, stdout_closed, stderr_closed),
            });
        }
        return None;
    }
    if token.len() > 1
        && token[0] == '-' as u16
        && token[1] != '-' as u16
        && !looks_like_negative_number(token)
    {
        // A short cluster: help only when h is its
        // first character; an unknown character
        // ahead of h makes the whole token an extra.
        if token[1] == 'h' as u16 {
            return Some(print_help(TOP_HELP, stdout_closed, stderr_closed));
        }
    }
    None
}

/// The top parser's only long option is --help.
fn match_long_top(name: &[u16]) -> LongMatch {
    let full = units("--help");
    if name == full.as_slice() {
        return LongMatch::Help;
    }
    if name.len() > 2 && full.starts_with(name) {
        return LongMatch::Help;
    }
    LongMatch::Unknown
}

fn split_glued(token: &[u16]) -> (&[u16], Option<Vec<u16>>) {
    match token.iter().position(|c| *c == '=' as u16) {
        Some(at) => (&token[..at], Some(token[at + 1..].to_vec())),
        None => (token, None),
    }
}

fn unrecognized(args: &[Token]) -> Msg {
    let mut message = Msg::str("unrecognized arguments: ");
    for (index, token) in args.iter().enumerate() {
        if index > 0 {
            message.push_str(" ");
        }
        message.push_units(&token.units);
    }
    message
}

struct ProposeArgs {
    source_dir: Vec<u16>,
    target: Option<Vec<u16>>,
    output: Option<Vec<u16>>,
}

enum SubOutcome {
    Done(i32),
    Parsed(ProposeArgs, Vec<Token>),
}

/// The subparser's scan. argparse matches and acts
/// token by token: a token's own matching (an
/// option's value, a cluster's whole shape) settles
/// before its actions run, so a cluster whose -o
/// lacks a value errors even when its -h comes
/// first, while a help token ahead of a later
/// erroring token prints help and exits. Extras
/// and the missing-source refusal settle after the
/// whole scan, the refusal first; extras bubble to
/// the top parser's report.
fn parse_propose(args: &[Token], stdout_closed: bool, stderr_closed: bool) -> SubOutcome {
    let mut source_dir: Option<Vec<u16>> = None;
    let mut target: Option<Vec<u16>> = None;
    let mut output: Option<Vec<u16>> = None;
    let mut extras: Vec<Token> = Vec::new();
    let mut positional_only = false;
    let mut index = 0;
    while index < args.len() {
        let token = &args[index];
        let u = &token.units;
        if !positional_only && units_eq(u, "--") {
            positional_only = true;
            index += 1;
            continue;
        }
        if !positional_only && u.len() > 2 && u[0] == '-' as u16 && u[1] == '-' as u16 {
            let (name, glued) = split_glued(u);
            match match_long(name) {
                LongMatch::Help => {
                    return SubOutcome::Done(match glued {
                        Some(value) => help_explicit(
                            sub_failure(Msg::new()),
                            "-h/--help",
                            &value,
                            stdout_closed,
                            stderr_closed,
                        ),
                        None => print_help(PROPOSE_HELP, stdout_closed, stderr_closed),
                    });
                }
                LongMatch::Target | LongMatch::Output => {
                    let named = if matches!(match_long(name), LongMatch::Target) {
                        "--target"
                    } else {
                        "--output"
                    };
                    let value = match glued {
                        Some(value) => value,
                        None => match next_value(args, index) {
                            Some(value) => {
                                index += 1;
                                value
                            }
                            None => {
                                return SubOutcome::Done(parse_error(
                                    sub_failure(Msg::str(&format!(
                                        "argument {named}: expected one argument"
                                    ))),
                                    stdout_closed,
                                    stderr_closed,
                                ));
                            }
                        },
                    };
                    if named == "--target" {
                        target = Some(value);
                    } else {
                        output = Some(value);
                    }
                    index += 1;
                    continue;
                }
                LongMatch::Unknown => {
                    extras.push(token.clone());
                    index += 1;
                    continue;
                }
            }
        }
        if !positional_only
            && u.len() > 1
            && u[0] == '-' as u16
            && u[1] != '-' as u16
            && !looks_like_negative_number(u)
        {
            // A short cluster, matched whole first:
            // h is help, o takes the rest of the
            // token or the next value token, and any
            // other character makes the whole token
            // an extra.
            let cluster = &u[1..];
            let mut help = false;
            let mut unknown = false;
            let mut missing = false;
            let mut cluster_output: Option<Vec<u16>> = None;
            let mut consumed_next = false;
            let mut at = 0;
            while at < cluster.len() {
                match cluster[at] {
                    0x68 => {
                        help = true;
                        at += 1;
                    }
                    0x6F => {
                        let rest = &cluster[at + 1..];
                        if !rest.is_empty() {
                            cluster_output = Some(if rest[0] == '=' as u16 {
                                rest[1..].to_vec()
                            } else {
                                rest.to_vec()
                            });
                        } else {
                            match next_value(args, index) {
                                Some(value) => {
                                    cluster_output = Some(value);
                                    consumed_next = true;
                                }
                                None => missing = true,
                            }
                        }
                        at = cluster.len();
                    }
                    _ => {
                        unknown = true;
                        at = cluster.len();
                    }
                }
            }
            if missing {
                return SubOutcome::Done(parse_error(
                    sub_failure(Msg::str("argument -o/--output: expected one argument")),
                    stdout_closed,
                    stderr_closed,
                ));
            }
            if unknown {
                extras.push(token.clone());
                index += 1;
                continue;
            }
            if help {
                return SubOutcome::Done(print_help(PROPOSE_HELP, stdout_closed, stderr_closed));
            }
            if let Some(value) = cluster_output {
                output = Some(value);
            }
            if consumed_next {
                index += 1;
            }
            index += 1;
            continue;
        }
        if source_dir.is_none() {
            source_dir = Some(u.clone());
        } else {
            extras.push(token.clone());
        }
        index += 1;
    }
    match source_dir {
        None => SubOutcome::Done(parse_error(
            sub_failure(Msg::str("the following arguments are required: source_dir")),
            stdout_closed,
            stderr_closed,
        )),
        Some(source_dir) => SubOutcome::Parsed(
            ProposeArgs {
                source_dir,
                target,
                output,
            },
            extras,
        ),
    }
}

/// The token after `index` as an option's value,
/// when it is a value: not an option-like token
/// and not the separator.
fn next_value(args: &[Token], index: usize) -> Option<Vec<u16>> {
    let next = args.get(index + 1)?;
    if is_option_like(&next.units) || units_eq(&next.units, "--") {
        return None;
    }
    Some(next.units.clone())
}

fn help_explicit(
    failure: ParseFailure,
    named: &str,
    value: &[u16],
    stdout_closed: bool,
    stderr_closed: bool,
) -> i32 {
    let mut message = Msg::str(&format!("argument {named}: ignored explicit argument "));
    message.push_str(&crate::pyrepr::repr_text(&Text::from_units(value.to_vec())));
    parse_error(
        ParseFailure {
            prog: failure.prog,
            usage: failure.usage,
            message,
        },
        stdout_closed,
        stderr_closed,
    )
}

// The propose run.

fn propose(args: &ProposeArgs, stdout_closed: bool, stderr_closed: bool) -> i32 {
    let source_display = Msg(args.source_dir.clone());
    let directory = os_path_from_units(&args.source_dir);
    let is_dir = std::fs::metadata(&directory).is_ok_and(|meta| meta.is_dir());
    if !is_dir {
        let mut line = Msg::str("error, not a directory, ");
        line.push_msg(&source_display);
        error_line(&line, stdout_closed, stderr_closed);
        return 1;
    }
    let entries = match read_sorted_entries(&directory) {
        Ok(entries) => entries,
        Err(_) => {
            let mut line = Msg::str("error, cannot list the directory, ");
            line.push_msg(&source_display);
            error_line(&line, stdout_closed, stderr_closed);
            return 1;
        }
    };
    let mut tapes: Vec<(Msg, Value, String)> = Vec::new();
    let mut by_digest: HashMap<String, &Value> = HashMap::new();
    for (name_units, path) in &entries {
        if !has_json_suffix(name_units) {
            continue;
        }
        let name = Msg(name_units.clone());
        match load_tape_file(path) {
            TapeLoad::Excluded(reason, defects) => {
                let mut line = Msg::str("error, ");
                line.push_msg(&name);
                line.push_str(&format!(" is excluded, {reason}"));
                error_line(&line, stdout_closed, stderr_closed);
                for defect in &defects {
                    let mut line = Msg::str("defect, ");
                    line.push_msg(defect);
                    error_line(&line, stdout_closed, stderr_closed);
                }
            }
            TapeLoad::Loaded(tape) => match digest_of(&tape, TAPE_DIGEST_BUDGET) {
                Ok(digest) => {
                    tapes.push((name, tape, digest));
                }
                Err(error) => {
                    let mut line = Msg::str("error, ");
                    line.push_msg(&name);
                    line.push_str(&format!(
                        " is excluded, the tape cannot be digested, {}",
                        error.0
                    ));
                    error_line(&line, stdout_closed, stderr_closed);
                }
            },
        }
    }
    if tapes.is_empty() {
        error_line(
            &Msg::str("error, no valid tapes in the directory"),
            stdout_closed,
            stderr_closed,
        );
        return 1;
    }
    for (_, tape, digest) in &tapes {
        by_digest.insert(digest.clone(), tape);
    }
    // An empty output string is no output at all:
    // the Python's `if output:` guards both the
    // overwrite check and the write.
    let output = args.output.as_ref().filter(|units| !units.is_empty());
    if let Some(output) = output {
        let resolved = resolve_path(output);
        let loaded: Vec<Vec<u16>> = tapes
            .iter()
            .map(|(name, _, _)| {
                let mut joined = args.source_dir.clone();
                if !joined.ends_with(&['/' as u16]) {
                    joined.push('/' as u16);
                }
                joined.extend_from_slice(&name.0);
                resolve_path(&joined)
            })
            .collect();
        if loaded.contains(&resolved) {
            error_line(
                &Msg::str("error, the output path is one of the input tapes, the run never writes over its own evidence"),
                stdout_closed,
                stderr_closed,
            );
            return 1;
        }
    }
    let target = match &args.target {
        Some(value) => Text::from_units(value.clone()),
        None => Text::from_str("AGENTS.md"),
    };
    let document = match build_proposals(&tapes, &target) {
        Ok(document) => document,
        Err(_) => return 1, // the crash class: the Python's uncaught DigestError
    };
    let defects = validate_proposals(&document, &by_digest);
    if !defects.is_empty() {
        error_line(
            &Msg::str("error, the proposal file does not validate"),
            stdout_closed,
            stderr_closed,
        );
        for defect in &defects {
            let mut line = Msg::str("defect, ");
            line.push_msg(defect);
            error_line(&line, stdout_closed, stderr_closed);
        }
        return 1;
    }
    match output {
        None => {
            let bytes = match dumps_view(&document, Mode::Stdout) {
                Ok(bytes) => bytes,
                Err(_) => return 1, // the crash class at the stdout write
            };
            if write_stdout(&bytes, stdout_closed) {
                0
            } else if stdout_closed {
                1 // the crash class: the stream was never there
            } else {
                error_line(
                    &Msg::str("error, cannot write the proposal file to stdout"),
                    stdout_closed,
                    stderr_closed,
                );
                1
            }
        }
        Some(output) => {
            let out_display = Msg(output.clone());
            let out_path = os_path_from_units(output.as_slice());
            match write_atomically(&out_path, &document) {
                WriteOutcome::Written => 0,
                WriteOutcome::Refused => {
                    let mut line = Msg::str("error, cannot write the proposal file, ");
                    line.push_msg(&out_display);
                    error_line(&line, stdout_closed, stderr_closed);
                    1
                }
                WriteOutcome::Crash => 1,
            }
        }
    }
}

enum WriteOutcome {
    Written,
    Refused,
    Crash,
}

/// The Python's _write_atomically: a sibling temp
/// file (prefix the name and a dot, suffix .tmp,
/// mode 0600), written, flushed, and replaced over
/// the target. An OS error unlinks the temp and
/// refuses; a render failure is not an OS error in
/// the Python and leaves the temp standing (the
/// crash class).
fn write_atomically(path: &PathBuf, document: &Value) -> WriteOutcome {
    let parent = path.parent().map(|p| p.to_path_buf()).unwrap_or_default();
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let temp = match make_temp(&parent, &name) {
        Ok(temp) => temp,
        Err(_) => return WriteOutcome::Refused,
    };
    let bytes = match dumps_view(document, Mode::Strict) {
        Ok(bytes) => bytes,
        Err(_) => return WriteOutcome::Crash,
    };
    let written = (|| -> std::io::Result<()> {
        let mut file = std::fs::File::create(&temp)?;
        file.write_all(&bytes)?;
        file.flush()?;
        Ok(())
    })();
    match written {
        Ok(()) => match std::fs::rename(&temp, path) {
            Ok(()) => WriteOutcome::Written,
            Err(_) => {
                let _ = std::fs::remove_file(&temp);
                WriteOutcome::Refused
            }
        },
        Err(_) => {
            let _ = std::fs::remove_file(&temp);
            WriteOutcome::Refused
        }
    }
}

fn make_temp(parent: &Path, name: &str) -> std::io::Result<PathBuf> {
    use std::os::unix::fs::OpenOptionsExt;
    const ALPHABET: &[u8] = b"abcdefghijklmnopqrstuvwxyz0123456789_";
    let mut rng = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.subsec_nanos() as u64 ^ (std::process::id() as u64) << 17)
        .unwrap_or(0x9e3779b9);
    for _ in 0..64 {
        let mut random = String::new();
        for _ in 0..8 {
            rng ^= rng << 13;
            rng ^= rng >> 7;
            rng ^= rng << 17;
            random.push(ALPHABET[(rng % 36) as usize] as char);
        }
        let candidate = parent.join(format!("{name}.{random}.tmp"));
        match std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&candidate)
        {
            Ok(_) => return Ok(candidate),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error),
        }
    }
    Err(std::io::Error::from(std::io::ErrorKind::AlreadyExists))
}

/// The directory's entries sorted by name in the
/// Python's string order, each with its joined OS
/// path.
fn read_sorted_entries(directory: &PathBuf) -> std::io::Result<Vec<(Vec<u16>, PathBuf)>> {
    let mut entries: Vec<(Vec<u16>, PathBuf)> = Vec::new();
    for entry in std::fs::read_dir(directory)? {
        let entry = entry?;
        let name = fsdecode_units(&os_bytes(&entry.file_name()));
        entries.push((name, entry.path()));
    }
    entries.sort_by(|a, b| scalars_of(&a.0).cmp(&scalars_of(&b.0)));
    Ok(entries)
}

/// pathlib's suffix rule over a name: the part
/// after the last dot, when the dot is neither the
/// first nor the last character.
fn has_json_suffix(name: &[u16]) -> bool {
    let scalars = scalars_of(name);
    let dot = scalars.iter().rposition(|c| *c == '.' as u32);
    match dot {
        Some(at) if at > 0 && at + 1 < scalars.len() => {
            scalars[at..] == ['.' as u32, 'j' as u32, 's' as u32, 'o' as u32, 'n' as u32]
        }
        _ => false,
    }
}

/// pathlib's resolve, in unit form: absolute
/// against the working directory, symlinks
/// resolved where the path exists, the rest
/// normalized lexically.
fn resolve_path(units: &[u16]) -> Vec<u16> {
    let path = os_path_from_units(units);
    let absolute = if path.is_absolute() {
        path
    } else {
        std::env::current_dir().unwrap_or_default().join(&path)
    };
    let resolved = match std::fs::canonicalize(&absolute) {
        Ok(found) => found,
        Err(_) => {
            // Resolve the longest existing prefix,
            // then append the rest normalized.
            let mut prefix = absolute.clone();
            let mut rest: Vec<std::ffi::OsString> = Vec::new();
            loop {
                match std::fs::canonicalize(&prefix) {
                    Ok(found) => {
                        let mut out = found;
                        for part in rest.iter().rev() {
                            out.push(part);
                        }
                        break normalize_lexical(&out);
                    }
                    Err(_) => match prefix.file_name() {
                        Some(name) => {
                            rest.push(name.to_os_string());
                            prefix = prefix.parent().map(|p| p.to_path_buf()).unwrap_or_default();
                            if prefix.as_os_str().is_empty() {
                                break normalize_lexical(&absolute);
                            }
                        }
                        None => break normalize_lexical(&absolute),
                    },
                }
            }
        }
    };
    fsdecode_units(&os_bytes(&resolved.as_os_str().to_owned()))
}

fn normalize_lexical(path: &Path) -> PathBuf {
    use std::path::Component;
    let mut out = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                out.pop();
            }
            other => out.push(other.as_os_str()),
        }
    }
    if out.as_os_str().is_empty() {
        out.push("/");
    }
    out
}
