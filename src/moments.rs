//! Moment passes over one tape: redo moments
//! (RF-4) and rescue moments (RF-9, RF-10).
//!
//! A redo moment is a tool call whose tool and
//! input match an earlier tool call in the same
//! tape, with an error tool result answering the
//! earlier call in between. Inputs are compared by
//! their canonical form 1 bytes, so two inputs that
//! differ only in key order are the same call. The
//! tape has already passed the format checks in
//! tapes.rs, so a result's call id answers an
//! earlier call and arrives after it.

use std::collections::HashMap;

use flighttape::value::{Text, Value};

use crate::digest::{canonical_bytes, Budget};

/// The render budget for a call input's canonical
/// bytes. Probing showed this render unreachable
/// as a refusal through the CLI (RF-17): a moment
/// still fires for an input whose subtree reaches
/// (990 nodes, 0 objects) and (989 nodes, 988
/// objects) from its own root, the whole of what
/// the tape digest's gate lets through, so the
/// Python's renderer never refuses an input the
/// digest accepted. The budget is generous.
pub const INPUT_RENDER_BUDGET: Budget = (1_000_000, 1_000_000);

/// A redo moment: the later call's seq, the
/// earliest errored matching call's seq, and the
/// tool name.
pub struct RedoMoment {
    pub seq: i64,
    pub earlier_seq: i64,
    pub tool: Text,
}

/// Every redo moment in one valid tape, in seq
/// order. A call whose input cannot be
/// canonicalized is skipped: it can never be shown
/// to match another call, and it is not indexed
/// for results either.
pub fn redo_moments(tape: &Value) -> Vec<RedoMoment> {
    let mut moments = Vec::new();
    let mut calls: HashMap<Text, ((Text, Vec<u8>), i64)> = HashMap::new();
    let mut first_errored: HashMap<(Text, Vec<u8>), i64> = HashMap::new();
    let events = match tape.get("events").and_then(Value::as_array) {
        Some(events) => events,
        None => return moments,
    };
    for event in events {
        let kind = str_field(event, "kind");
        if kind.as_deref() == Some("tool_result") {
            if matches!(event.get("is_error"), Some(Value::Bool(true))) {
                if let Some(Value::Str(call_id)) = event.get("call_id") {
                    if let Some((key, seq)) = calls.get(call_id) {
                        let entry = first_errored.entry(key.clone()).or_insert(*seq);
                        if *seq < *entry {
                            *entry = *seq;
                        }
                    }
                }
            }
            continue;
        }
        if kind.as_deref() != Some("tool_call") {
            continue;
        }
        let tool = match event.get("tool") {
            Some(Value::Str(tool)) => tool.clone(),
            _ => continue,
        };
        let input = event.get("input").cloned().unwrap_or(Value::Null);
        let bytes = match canonical_bytes(&input, INPUT_RENDER_BUDGET) {
            Ok(bytes) => bytes,
            Err(_) => continue,
        };
        let key = (tool, bytes);
        if let Some(earlier) = first_errored.get(&key) {
            moments.push(RedoMoment {
                seq: int_field(event, "seq"),
                earlier_seq: *earlier,
                tool: key.0.clone(),
            });
        }
        if let Some(Value::Str(call_id)) = event.get("call_id") {
            calls.insert(call_id.clone(), (key, int_field(event, "seq")));
        }
    }
    moments
}

/// A rescue moment: the message's seq and the tool
/// it is attributed to, or None when no main-chain
/// tool call precedes it.
pub struct RescueMoment {
    pub seq: i64,
    pub tool: Option<Text>,
}

/// Every rescue moment in one valid tape, in seq
/// order (RF-9, RF-10). Only main-chain events
/// count: an event whose sidechain is not exactly
/// false is invisible to this pass.
pub fn rescue_moments(tape: &Value) -> Vec<RescueMoment> {
    let mut moments = Vec::new();
    let events = match tape.get("events").and_then(Value::as_array) {
        Some(events) => events,
        None => return moments,
    };
    let mut seen_user = false;
    let mut agent_acted = false;
    let mut last_tool: Option<Text> = None;
    for event in events {
        if !matches!(event.get("sidechain"), Some(Value::Bool(false))) {
            continue;
        }
        let kind = str_field(event, "kind");
        if kind.as_deref() == Some("tool_call") {
            agent_acted = true;
            if let Some(Value::Str(tool)) = event.get("tool") {
                last_tool = Some(tool.clone());
            }
        } else if kind.as_deref() == Some("message")
            && str_field(event, "role").as_deref() == Some("assistant")
        {
            agent_acted = true;
        } else if kind.as_deref() == Some("message")
            && str_field(event, "role").as_deref() == Some("user")
        {
            if seen_user && agent_acted {
                moments.push(RescueMoment {
                    seq: int_field(event, "seq"),
                    tool: last_tool.clone(),
                });
            }
            seen_user = true;
            agent_acted = false;
        }
    }
    moments
}

fn str_field(value: &Value, name: &str) -> Option<String> {
    match value.get(name) {
        Some(Value::Str(text)) => text.as_str(),
        _ => None,
    }
}

fn int_field(value: &Value, name: &str) -> i64 {
    match value.get(name) {
        Some(Value::Int(digits)) => digits.parse::<i64>().unwrap_or(0),
        _ => 0,
    }
}
