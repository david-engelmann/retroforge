//! Reading flighttape tapes (RF-3).
//!
//! Retroforge reads tapes at flighttape format
//! version 1.0 only, never raw harness session
//! files. The checks here are the format rules the
//! rest of the tool stands on: the value is an object
//! carrying tape_version 1.0, a run object, and an
//! events array whose seq values run consecutively
//! from 0 and whose kinds are in the format's enum,
//! and the tool call shapes the flighttape validator
//! enforces under its FT-4 pairing rule, a tool call
//! carrying a non-empty tool name and a unique
//! non-empty call id, a tool result answering an
//! earlier call, an is_error that is a boolean when
//! present. A file that fails any check is excluded
//! with its defects named, in the rollup discipline,
//! and the run proceeds on the tapes that remain.

use std::collections::HashSet;
use std::path::Path;

use flighttape::value::{Text, Value};

use crate::pyrepr::repr_value;
use crate::util::Msg;

pub const EVENT_KINDS: [&str; 8] = [
    "session_start",
    "message",
    "tool_call",
    "tool_result",
    "diff",
    "spend",
    "note",
    "session_end",
];

/// One tape file's outcome: the parsed tape, or the
/// exclusion the CLI prints (a reason, and the
/// format defects when the file parsed but is not a
/// valid tape).
pub enum TapeLoad {
    Loaded(Value),
    Excluded(&'static str, Vec<Msg>),
}

/// Read one tape file. The JSON reader is the
/// flighttape crate's; its two refusal shapes map
/// onto the Python's (RF-17).
pub fn load_tape_file(path: &Path) -> TapeLoad {
    let raw = match std::fs::read(path) {
        Ok(raw) => raw,
        Err(_) => return TapeLoad::Excluded("cannot read the tape file", Vec::new()),
    };
    let text = match String::from_utf8(raw) {
        Ok(text) => text,
        Err(_) => return TapeLoad::Excluded("the file is not valid UTF-8", Vec::new()),
    };
    let tape = match flighttape::json::parse(&text) {
        Ok(tape) => tape,
        Err(flighttape::json::ParseError::TooLarge) => {
            return TapeLoad::Excluded("the file holds a number too large to read", Vec::new());
        }
        Err(flighttape::json::ParseError::Invalid) => {
            return TapeLoad::Excluded("the file is not valid JSON", Vec::new());
        }
    };
    // The Python's reader is recursive and refuses a
    // file nested past its limit as not valid JSON;
    // the crate's reader is iterative and accepts it.
    // The probed cap stands in for the recursion.
    if value_depth(&tape) > JSON_DEPTH_CAP {
        return TapeLoad::Excluded("the file is not valid JSON", Vec::new());
    }
    let defects = tape_defects(&tape);
    if !defects.is_empty() {
        return TapeLoad::Excluded("not a valid tape", defects);
    }
    TapeLoad::Loaded(tape)
}

/// The deepest container nesting in a value,
/// counted iteratively (a scalar is depth 0, an
/// empty container depth 1).
pub fn value_depth(value: &Value) -> usize {
    let mut deepest = 0usize;
    let mut stack: Vec<(&Value, usize)> = vec![(value, 1)];
    while let Some((node, depth)) = stack.pop() {
        match node {
            Value::Array(items) => {
                deepest = deepest.max(depth);
                for item in items {
                    stack.push((item, depth + 1));
                }
            }
            Value::Object(entries) => {
                deepest = deepest.max(depth);
                for (_, item) in entries {
                    stack.push((item, depth + 1));
                }
            }
            _ => {}
        }
    }
    deepest
}

/// The parse depth cap, probed at the oracle CLI
/// for the shipped reader path (RF-17): an array
/// chain nested 9,992 deep under run still parses
/// (and is then refused by the digest), at 9,993
/// the reader refuses it as not valid JSON. In
/// this module's counting (the tape object is
/// depth 1) the last parseable file has depth
/// 9,994.
pub const JSON_DEPTH_CAP: usize = 9994;

/// The format checks the tool's evidence stands on.
pub fn tape_defects(tape: &Value) -> Vec<Msg> {
    let mut defects: Vec<Msg> = Vec::new();
    if tape.as_object().is_none() {
        return vec![Msg::str("the tape is not an object")];
    }
    if !matches!(tape.get("tape_version"), Some(v) if text_is_str(v, "1.0")) {
        defects.push(Msg::str("tape_version is not 1.0"));
    }
    if tape.get("run").and_then(Value::as_object).is_none() {
        defects.push(Msg::str("run is not an object"));
    }
    let events = match tape.get("events").and_then(Value::as_array) {
        Some(events) if !events.is_empty() => events,
        _ => {
            defects.push(Msg::str("events must hold at least one event"));
            return defects;
        }
    };
    let mut seen_calls: HashSet<Text> = HashSet::new();
    for (index, event) in events.iter().enumerate() {
        let where_ = format!("events[{index}]");
        if event.as_object().is_none() {
            defects.push(Msg::str(&format!("{where_} is not an object")));
            continue;
        }
        let seq_ok = match event.get("seq") {
            Some(Value::Int(digits)) => digits == &index.to_string(),
            _ => false,
        };
        if !seq_ok {
            defects.push(Msg::str(&format!("{where_}.seq is not {index}")));
        }
        let kind_str = match event.get("kind") {
            Some(Value::Str(text)) => match text.as_str() {
                Some(name) if EVENT_KINDS.contains(&name.as_str()) => name,
                _ => {
                    defects.push(Msg::str(&format!(
                        "{where_}.kind is not in the format enum"
                    )));
                    continue;
                }
            },
            _ => {
                defects.push(Msg::str(&format!(
                    "{where_}.kind is not in the format enum"
                )));
                continue;
            }
        };
        let call_id = event.get("call_id");
        if kind_str == "tool_call" {
            let tool_ok = matches!(event.get("tool"), Some(Value::Str(t)) if !t.units().is_empty());
            if !tool_ok {
                defects.push(Msg::str(&format!(
                    "{where_}.tool must be a non-empty string"
                )));
            }
            match call_id {
                Some(Value::Str(id)) if !id.units().is_empty() => {
                    if !seen_calls.insert(id.clone()) {
                        let mut m = Msg::str(&format!("{where_}.call_id "));
                        m.push_str(&repr_value(&Value::Str(id.clone())));
                        m.push_str(" is used twice");
                        defects.push(m);
                    }
                }
                _ => {
                    defects.push(Msg::str(&format!(
                        "{where_}.call_id must be a non-empty string"
                    )));
                }
            }
        }
        if kind_str == "tool_result" {
            let answers = matches!(call_id, Some(Value::Str(id)) if seen_calls.contains(id));
            if !answers {
                let mut m = Msg::str(&format!("{where_}.call_id "));
                m.push_str(&match call_id {
                    Some(v) => repr_value(v),
                    None => "None".to_string(),
                });
                m.push_str(" answers no earlier tool_call");
                defects.push(m);
            }
            if let Some(is_error) = event.get("is_error") {
                if !matches!(is_error, Value::Bool(_)) {
                    defects.push(Msg::str(&format!("{where_}.is_error must be a boolean")));
                }
            }
        }
    }
    defects
}

fn text_is_str(value: &Value, name: &str) -> bool {
    matches!(value, Value::Str(text) if text.as_str().is_some_and(|s| s == name))
}
