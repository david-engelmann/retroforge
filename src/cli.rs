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
use crate::emit::dumps_view;
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
    Ambiguous,
    Unknown,
}

fn match_long(name: &[u16]) -> LongMatch {
    // An empty option name (the token "--=...")
    // prefix-matches every long option: ambiguous
    // where a parser has more than one.
    if name.len() == 2 {
        return LongMatch::Ambiguous;
    }
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
                _ => LongMatch::Unknown,
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
                _ => LongMatch::Unknown,
            });
        }
    }
    found.unwrap_or(LongMatch::Unknown)
}

pub fn run_with_args(raw: &[Vec<u16>], stdout_closed: bool, stderr_closed: bool) -> i32 {
    let args = tokens(raw);
    let mut top_extras: Vec<Token> = Vec::new();
    let mut index = 0;
    while index < args.len() {
        let token = &args[index];
        let u = &token.units;
        if units_eq(u, "--") {
            // The subparsers positional takes the
            // separator itself as its value when any
            // token follows it; a trailing separator
            // alone leaves the command missing.
            if index + 1 == args.len() {
                break;
            }
            let mut message = Msg::str("argument command: invalid choice: '--'");
            message.push_str(" (choose from 'propose')");
            return parse_error(top_failure(message), stdout_closed, stderr_closed);
        }
        if let Some(code) = top_help_action(u, stdout_closed, stderr_closed) {
            return code;
        }
        if is_option_like(u) {
            top_extras.push(token.clone());
            index += 1;
            continue;
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

/// The top parser's only long option is --help,
/// so even the empty name (the token "--=...")
/// resolves to it.
fn match_long_top(name: &[u16]) -> LongMatch {
    let full = units("--help");
    if name == full.as_slice() {
        return LongMatch::Help;
    }
    if name.len() == 2 {
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
                LongMatch::Ambiguous => {
                    let mut message = Msg::str("ambiguous option: ");
                    message.push_units(u);
                    message.push_str(" could match --help, --target, --output");
                    return SubOutcome::Done(parse_error(
                        sub_failure(message),
                        stdout_closed,
                        stderr_closed,
                    ));
                }
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
                    // argparse names an option in its
                    // errors by all of its strings.
                    let named = if matches!(match_long(name), LongMatch::Target) {
                        "--target"
                    } else {
                        "-o/--output"
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
        // The Python resolves the output and every
        // loaded tape before comparing; a path whose
        // resolution meets a symlink loop raises
        // there, uncaught (the crash class, RF-25).
        let resolved = match resolve_path(output) {
            Some(resolved) => resolved,
            None => return 1,
        };
        let mut loaded: Vec<Vec<u16>> = Vec::new();
        for (name, _, _) in &tapes {
            // pathlib's join: an empty directory
            // contributes no component at all.
            let mut joined = args.source_dir.clone();
            if !joined.is_empty() && !joined.ends_with(&['/' as u16]) {
                joined.push('/' as u16);
            }
            joined.extend_from_slice(&name.0);
            match resolve_path(&joined) {
                Some(resolved_tape) => loaded.push(resolved_tape),
                None => return 1,
            }
        }
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
    // The Python renders the document text before
    // anything is written anywhere; the temp file
    // below is created only for a text that already
    // exists.
    let bytes = dumps_view(&document);
    match output {
        None => {
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
            match write_atomically(&out_path, &bytes) {
                WriteOutcome::Written => 0,
                WriteOutcome::Refused => {
                    let mut line = Msg::str("error, cannot write the proposal file, ");
                    line.push_msg(&out_display);
                    error_line(&line, stdout_closed, stderr_closed);
                    1
                }
            }
        }
    }
}

enum WriteOutcome {
    Written,
    Refused,
}

/// The Python's _write_atomically over a rendered
/// text: a sibling temp file (prefix the name and
/// a dot, suffix .tmp, mode 0600), written, flushed,
/// and replaced over the target. An OS error
/// unlinks the temp and refuses.
fn write_atomically(path: &PathBuf, bytes: &[u8]) -> WriteOutcome {
    let parent = path.parent().map(|p| p.to_path_buf()).unwrap_or_default();
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let temp = match make_temp(&parent, &name) {
        Ok(temp) => temp,
        Err(_) => return WriteOutcome::Refused,
    };
    let written = (|| -> std::io::Result<()> {
        let mut file = std::fs::File::create(&temp)?;
        file.write_all(bytes)?;
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
    entries.sort_by_cached_key(|entry| scalars_of(&entry.0));
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

/// The walk's recursion budget (RF-32): at the
/// shipped CLI the interpreter's default recursion
/// limit leaves room for 990 nested splices, and
/// the 991st raises RecursionError. The count is
/// of nesting depth, not of splices in total: the
/// top call of a resolution stands at depth 0 and
/// each splice's call stands one deeper, so a
/// spelling carrying two sequential chains nests
/// only as deep as the deeper one.
const MAX_WALK_DEPTH: usize = 990;

/// os.path.join on byte strings: an absolute
/// second part stands alone, an empty or
/// separator-ending first part takes the second
/// as it is, anything else joins on a separator.
fn join_bytes(first: &[u8], second: &[u8]) -> Vec<u8> {
    if second.first() == Some(&0x2F) {
        return second.to_vec();
    }
    let mut out = first.to_vec();
    if !out.is_empty() && !out.ends_with(b"/") {
        out.push(0x2F);
    }
    out.extend_from_slice(second);
    out
}

/// os.path.split on byte strings: the head is
/// everything up to the last separator, trailing
/// separators stripped unless the head is nothing
/// but separators, and the tail is what follows.
fn split_bytes(path: &[u8]) -> (Vec<u8>, Vec<u8>) {
    let index = path
        .iter()
        .rposition(|byte| *byte == 0x2F)
        .map_or(0, |i| i + 1);
    let (mut head, tail) = (path[..index].to_vec(), path[index..].to_vec());
    if !head.is_empty() && !head.iter().all(|byte| *byte == 0x2F) {
        while head.ends_with(b"/") {
            head.pop();
        }
    }
    (head, tail)
}

/// A transient path over raw bytes, for the
/// walk's stats and link reads.
fn path_from_bytes(bytes: &[u8]) -> PathBuf {
    os_path_from_units(&fsdecode_units(bytes))
}

/// The interpreter's realpath walk itself
/// (posixpath._joinrealpath with strict=False):
/// `path` is the resolved prefix, `rest` the
/// spelling still to walk, and `seen` the links
/// already met, an open entry (None) marking a
/// resolution still in progress and a finished
/// one carrying its result. `depth` is the
/// nesting of this call, the top call of a
/// resolution standing at 0; a call entered past
/// MAX_WALK_DEPTH aborts the walk, read here as
/// None, the way the interpreter's RecursionError
/// aborts the resolution (RF-32). Returns the
/// resolved prefix and whether the walk
/// completed; a link whose target bytes cannot
/// be read aborts the walk the way the
/// interpreter's uncaught raise does, likewise
/// read as None.
fn joinrealpath(
    mut path: Vec<u8>,
    rest: Vec<u8>,
    seen: &mut HashMap<Vec<u8>, Option<Vec<u8>>>,
    depth: usize,
) -> Option<(Vec<u8>, bool)> {
    if depth > MAX_WALK_DEPTH {
        return None;
    }
    let mut rest: &[u8] = &rest;
    if rest.first() == Some(&0x2F) {
        rest = &rest[1..];
        path = vec![0x2F];
    }
    while !rest.is_empty() {
        let (name, tail) = match rest.iter().position(|byte| *byte == 0x2F) {
            Some(index) => (&rest[..index], &rest[index + 1..]),
            None => (rest, &rest[rest.len()..]),
        };
        rest = tail;
        if name.is_empty() || name == b"." {
            continue;
        }
        if name == b".." {
            if !path.is_empty() {
                let (head, popped) = split_bytes(&path);
                path = head;
                if popped == b".." {
                    path = join_bytes(&join_bytes(&path, b".."), b"..");
                }
            } else {
                path = b"..".to_vec();
            }
            continue;
        }
        let newpath = join_bytes(&path, name);
        let is_link = match std::fs::symlink_metadata(path_from_bytes(&newpath)) {
            Ok(meta) => meta.file_type().is_symlink(),
            Err(_) => false,
        };
        if !is_link {
            path = newpath;
            continue;
        }
        if let Some(cached) = seen.get(&newpath) {
            match cached {
                Some(resolved) => {
                    path = resolved.clone();
                    continue;
                }
                None => return Some((join_bytes(&newpath, rest), false)),
            }
        }
        seen.insert(newpath.clone(), None);
        let target = match std::fs::read_link(path_from_bytes(&newpath)) {
            Ok(target) => os_bytes(&target.as_os_str().to_owned()),
            Err(_) => return None,
        };
        let (resolved, ok) = joinrealpath(path, target, seen, depth + 1)?;
        if !ok {
            return Some((join_bytes(&resolved, rest), false));
        }
        seen.insert(newpath, Some(resolved.clone()));
        path = resolved;
    }
    Some((path, true))
}

/// pathlib's resolve, in unit form (RF-31): the
/// spelled path is made absolute against the
/// working directory, walked by the interpreter's
/// own algorithm (joinrealpath above), and
/// finished with the abspath normalization, after
/// which pathlib stats the finished path and a
/// loop report there raises, uncaught in the
/// Python; the caller treats a failed resolution
/// as the crash class (RF-25). A relative spelling
/// whose working directory cannot be read fails
/// outright (RF-33): the Python raises
/// FileNotFoundError out of that abspath from a
/// deleted working directory, where no relative
/// component can be stat'ed and no relative walk
/// completes. The loop is judged on the finished
/// path, not on the spellings the walk tries along
/// the way: a loop component a following ".."
/// cancels never survives into the finished path,
/// and the Python resolves those spellings cleanly.
fn resolve_path(units: &[u16]) -> Option<Vec<u16>> {
    const ELOOP: i32 = 40;
    let path = os_path_from_units(units);
    let spelled = if path.is_absolute() {
        path
    } else {
        match std::env::current_dir() {
            Ok(cwd) => cwd.join(&path),
            Err(_) => return None,
        }
    };
    let spelling = os_bytes(&spelled.as_os_str().to_owned());
    let mut seen = HashMap::new();
    let (resolved, _ok) = joinrealpath(Vec::new(), spelling, &mut seen, 0)?;
    // The abspath finish: the walk's result is
    // already absolute, so this is normpath over
    // its bytes, dropping "." components and
    // popping on ".." (a pop at the root drops).
    let mut parts: Vec<&[u8]> = Vec::new();
    for part in resolved.split(|byte| *byte == 0x2F) {
        if part.is_empty() || part == b"." {
            continue;
        }
        if part == b".." {
            parts.pop();
            continue;
        }
        parts.push(part);
    }
    let mut finished: Vec<u8> = vec![0x2F];
    for (index, part) in parts.iter().enumerate() {
        if index > 0 {
            finished.push(0x2F);
        }
        finished.extend_from_slice(part);
    }
    // The closing stat of resolve(): only the
    // kernel's loop report fails the resolution.
    if let Err(error) = std::fs::metadata(path_from_bytes(&finished)) {
        if error.raw_os_error() == Some(ELOOP) {
            return None;
        }
    }
    Some(fsdecode_units(&finished))
}
