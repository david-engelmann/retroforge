//! Proposal files (RF-1, RF-2, RF-5, RF-6, RF-7).
//!
//! A proposal file is the run's only output: a JSON
//! object with a format version, the period its
//! tapes cover, and its proposals. Every proposal
//! carries at least two citations, and every
//! citation must resolve against the tape set it
//! was built from, a tape digest under canonical
//! form 1, a seq, and the kind of the event at that
//! seq. The validator here is the enforcement of
//! obligation one: a proposal with no evidence, or
//! with evidence that does not resolve, is invalid
//! and the validator names each broken citation.

use std::collections::{HashMap, HashSet};

use flighttape::value::{Text, Value};

use crate::digest::{digest_of, Budget, DigestError};
use crate::moments::{redo_moments, rescue_moments};
use crate::pyrepr::repr_value;
use crate::util::{scalars_of, Msg};

pub const PROPOSAL_VERSION: &str = "1.0";
const PROPOSAL_TYPES: [&str; 3] = ["rules-edit", "hook-change", "roster-note"];
const HUNK_ACTIONS: [&str; 3] = ["add", "strike", "rewrite"];
const REDO_THRESHOLD: usize = 2;
const RESCUE_THRESHOLD: usize = 2;

/// The registry payload is shallow by construction;
/// its render cannot reach the interpreter limit.
const PAYLOAD_BUDGET: Budget = (1_000_000, 1_000_000);

fn text(value: &str) -> Value {
    Value::Str(Text::from_str(value))
}

fn obj(entries: Vec<(&str, Value)>) -> Value {
    Value::Object(
        entries
            .into_iter()
            .map(|(key, value)| (Text::from_str(key), value))
            .collect(),
    )
}

/// Build a proposal file from (name, tape, digest)
/// triples in filename order (RF-5 to RF-14). A
/// digest failure on a registry payload is the
/// Python's uncaught DigestError: the crash class,
/// returned to the caller.
pub fn build_proposals(
    tapes: &[(Msg, Value, String)],
    target: &Text,
) -> Result<Value, DigestError> {
    let mut starts: Vec<Text> = Vec::new();
    for (_, tape, _) in tapes {
        if let Some(Value::Str(value)) = tape.get("run").and_then(|run| run.get("started_at")) {
            starts.push(value.clone());
        }
    }
    let period = obj(vec![
        (
            "from",
            match starts.iter().min_by(|a, b| cmp_text(a, b)) {
                Some(value) => Value::Str(value.clone()),
                None => Value::Null,
            },
        ),
        (
            "to",
            match starts.iter().max_by(|a, b| cmp_text(a, b)) {
                Some(value) => Value::Str(value.clone()),
                None => Value::Null,
            },
        ),
    ]);
    let mut by_tool: HashMap<Text, Vec<(String, i64)>> = HashMap::new();
    let mut rescues_by_tool: HashMap<Text, Vec<(String, i64)>> = HashMap::new();
    let mut unattributed_rescues: i64 = 0;
    let mut seen_digests: HashSet<&str> = HashSet::new();
    for (_, tape, digest) in tapes {
        if !seen_digests.insert(digest.as_str()) {
            continue;
        }
        for moment in redo_moments(tape) {
            by_tool
                .entry(moment.tool.clone())
                .or_default()
                .push((digest.clone(), moment.seq));
        }
        for moment in rescue_moments(tape) {
            match &moment.tool {
                None => unattributed_rescues += 1,
                Some(tool) => rescues_by_tool
                    .entry(tool.clone())
                    .or_default()
                    .push((digest.clone(), moment.seq)),
            }
        }
    }
    let mut tools: Vec<Text> = by_tool
        .keys()
        .chain(rescues_by_tool.keys())
        .cloned()
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();
    tools.sort_by(cmp_text);
    let mut proposals: Vec<Value> = Vec::new();
    for tool in &tools {
        let moments = by_tool.get(tool).cloned().unwrap_or_default();
        if moments.len() >= REDO_THRESHOLD {
            let citations = moments
                .iter()
                .map(|(digest, seq)| {
                    obj(vec![
                        ("tape", text(digest)),
                        ("seq", Value::Int(seq.to_string())),
                        ("kind", text("tool_call")),
                    ])
                })
                .collect();
            let mut id = Msg::str("rules-edit-");
            id.push_text(tool);
            let mut hunk = Msg::str("Before calling ");
            hunk.push_text(tool);
            hunk.push_str(" again after a failure, stop and read the error, then change the input or the approach. This run repeated a failed ");
            hunk.push_text(tool);
            hunk.push_str(&format!(" call {} times across the period.", moments.len()));
            let mut rationale = Msg::new();
            rationale.push_text(tool);
            rationale.push_str(&format!(
                " was called again with the same input after an error {} times in the period, at the cited moments.",
                moments.len()
            ));
            let proposal = proposal(id, target, hunk, rationale, citations, &period)?;
            emit(&mut proposals, proposal);
        }
        let found = rescues_by_tool.get(tool).cloned().unwrap_or_default();
        if found.len() >= RESCUE_THRESHOLD {
            let citations = found
                .iter()
                .map(|(digest, seq)| {
                    obj(vec![
                        ("tape", text(digest)),
                        ("seq", Value::Int(seq.to_string())),
                        ("kind", text("message")),
                    ])
                })
                .collect();
            let mut id = Msg::str("rules-edit-rescue-");
            id.push_text(tool);
            let mut hunk = Msg::str("Before acting on ");
            hunk.push_text(tool);
            hunk.push_str(" output, state what it shows and what happens next, so the operator can redirect early. In this period the operator's turn followed ");
            hunk.push_text(tool);
            hunk.push_str(&format!(
                " work {} times, each time after the agent had acted.",
                found.len()
            ));
            let mut rationale = Msg::str("The operator's turn followed ");
            rationale.push_text(tool);
            rationale.push_str(&format!(
                " work {} times in the period, each time after the agent had acted, at the cited moments.",
                found.len()
            ));
            let proposal = proposal(id, target, hunk, rationale, citations, &period)?;
            emit(&mut proposals, proposal);
        }
    }
    let mut document = vec![
        ("proposal_version", text(PROPOSAL_VERSION)),
        ("period", period),
        ("proposals", Value::Array(proposals)),
    ];
    if unattributed_rescues > 0 {
        document.push((
            "unattributed_rescues",
            Value::Int(unattributed_rescues.to_string()),
        ));
    }
    Ok(obj(document))
}

/// RF-11a: a proposal whose id is already emitted
/// in the same file carries a suffix of a dash and
/// the first eight characters of its own registry
/// experiment id's digest.
fn emit(proposals: &mut Vec<Value>, mut proposal: Value) {
    let id = match proposal.get("id") {
        Some(Value::Str(id)) => id.clone(),
        _ => return,
    };
    let collides = proposals
        .iter()
        .any(|other| matches!(other.get("id"), Some(Value::Str(other_id)) if *other_id == id));
    if collides {
        if let Some(Value::Str(experiment)) = proposal
            .get("registry")
            .and_then(|registry| registry.get("experiment_id"))
        {
            let experiment = experiment.as_str().unwrap_or_default();
            let suffix: String = experiment
                .strip_prefix("exp-")
                .unwrap_or(&experiment)
                .chars()
                .take(8)
                .collect();
            let mut units = id.units().to_vec();
            units.extend(format!("-{suffix}").encode_utf16());
            if let Some(Value::Str(slot)) = proposal.get_mut("id") {
                *slot = Text::from_units(units);
            }
        }
    }
    proposals.push(proposal);
}

fn proposal(
    pid: Msg,
    target: &Text,
    hunk_text: Msg,
    rationale: Msg,
    citations: Vec<Value>,
    period: &Value,
) -> Result<Value, DigestError> {
    let hunk = obj(vec![
        ("action", text("add")),
        ("text", Value::Str(Text::from_units(hunk_text.0.clone()))),
    ]);
    let rationale = Value::Str(Text::from_units(rationale.0.clone()));
    let citations = Value::Array(citations);
    let payload = obj(vec![
        ("type", text("rules-edit")),
        ("target", Value::Str(target.clone())),
        ("hunk", hunk.clone()),
        ("rationale", rationale.clone()),
        ("citations", citations.clone()),
        ("baseline_period", period.clone()),
    ]);
    let experiment = format!("exp-{}", &digest_of(&payload, PAYLOAD_BUDGET)?[..16]);
    Ok(obj(vec![
        ("id", Value::Str(Text::from_units(pid.0.clone()))),
        ("type", text("rules-edit")),
        ("target", Value::Str(target.clone())),
        ("hunk", hunk),
        ("rationale", rationale),
        ("citations", citations),
        (
            "registry",
            obj(vec![
                ("experiment_id", text(&experiment)),
                ("baseline_period", period.clone()),
            ]),
        ),
    ]))
}

/// Name every defect in a proposal file (RF-1,
/// RF-2). tapes_by_digest maps each input tape's
/// canonical form 1 digest to the tape itself.
pub fn validate_proposals(document: &Value, tapes_by_digest: &HashMap<String, &Value>) -> Vec<Msg> {
    let mut defects: Vec<Msg> = Vec::new();
    if document.as_object().is_none() {
        return vec![Msg::str("the proposal file is not an object")];
    }
    if !matches!(document.get("proposal_version"), Some(v) if str_is(v, PROPOSAL_VERSION)) {
        defects.push(Msg::str("proposal_version is not 1.0"));
    }
    if let Some(count) = document.get("unattributed_rescues") {
        let valid = match count {
            Value::Int(digits) => !digits.starts_with('-'),
            _ => false,
        };
        if !valid {
            defects.push(Msg::str(
                "unattributed_rescues is not a non-negative integer",
            ));
        }
    }
    match document.get("period") {
        Some(period) if period.as_object().is_some() => {
            for key in ["from", "to"] {
                let valid = match period.get(key) {
                    Some(Value::Null) => true,
                    Some(Value::Str(_)) => true,
                    _ => false,
                };
                if !valid {
                    defects.push(Msg::str(&format!("period.{key} must be a string or null")));
                }
            }
        }
        _ => defects.push(Msg::str("period is not an object")),
    }
    let proposals = match document.get("proposals") {
        Some(Value::Array(items)) => items,
        _ => {
            defects.push(Msg::str("proposals is not an array"));
            return defects;
        }
    };
    let mut seen_ids: HashSet<Text> = HashSet::new();
    for (index, proposal) in proposals.iter().enumerate() {
        let where_ = format!("proposals[{index}]");
        if proposal.as_object().is_none() {
            defects.push(Msg::str(&format!("{where_} is not an object")));
            continue;
        }
        for field in [
            "id",
            "type",
            "target",
            "hunk",
            "rationale",
            "citations",
            "registry",
        ] {
            if proposal.get(field).is_none() {
                defects.push(Msg::str(&format!("{where_} is missing {field}")));
            }
        }
        for field in ["id", "target", "rationale"] {
            if let Some(value) = proposal.get(field) {
                if !matches!(value, Value::Str(t) if !t.units().is_empty()) {
                    defects.push(Msg::str(&format!(
                        "{where_}.{field} must be a non-empty string"
                    )));
                }
            }
        }
        if let Some(Value::Str(id)) = proposal.get("id") {
            if !seen_ids.insert(id.clone()) {
                let mut m = Msg::str(&format!("{where_}.id "));
                m.push_str(&repr_value(&Value::Str(id.clone())));
                m.push_str(" is used twice");
                defects.push(m);
            }
        }
        match proposal.get("registry") {
            Some(registry) if registry.as_object().is_some() => {
                let experiment_ok = matches!(
                    registry.get("experiment_id"),
                    Some(Value::Str(t)) if t.as_str().is_some_and(|s| s.starts_with("exp-"))
                );
                if !experiment_ok {
                    defects.push(Msg::str(&format!(
                        "{where_}.registry.experiment_id is not an experiment id"
                    )));
                }
                if registry
                    .get("baseline_period")
                    .and_then(Value::as_object)
                    .is_none()
                {
                    defects.push(Msg::str(&format!(
                        "{where_}.registry.baseline_period is not an object"
                    )));
                }
            }
            Some(_) => {
                defects.push(Msg::str(&format!("{where_}.registry is not an object")));
            }
            None => {}
        }
        if !matches!(proposal.get("type"), Some(v) if PROPOSAL_TYPES.iter().any(|t| str_is(v, t))) {
            defects.push(Msg::str(&format!("{where_}.type is not a proposal type")));
        }
        match proposal.get("hunk") {
            Some(hunk) if hunk.as_object().is_some() => {
                if !matches!(hunk.get("action"), Some(v) if HUNK_ACTIONS.iter().any(|t| str_is(v, t)))
                {
                    defects.push(Msg::str(&format!(
                        "{where_}.hunk.action is not a hunk action"
                    )));
                }
                if !matches!(hunk.get("text"), Some(Value::Str(t)) if !t.units().is_empty()) {
                    defects.push(Msg::str(&format!("{where_}.hunk.text is empty")));
                }
            }
            Some(_) => {
                defects.push(Msg::str(&format!("{where_}.hunk is not an object")));
            }
            None => {}
        }
        match proposal.get("citations") {
            Some(Value::Array(citations)) => {
                if citations.len() < 2 {
                    defects.push(Msg::str(&format!(
                        "{where_} carries fewer than two citations"
                    )));
                }
                for (cindex, citation) in citations.iter().enumerate() {
                    defects.extend(citation_defects(
                        &format!("{where_}.citations[{cindex}]"),
                        citation,
                        tapes_by_digest,
                    ));
                }
            }
            Some(_) => {
                defects.push(Msg::str(&format!("{where_}.citations is not an array")));
            }
            None => {}
        }
    }
    defects
}

fn citation_defects(
    where_: &str,
    citation: &Value,
    tapes_by_digest: &HashMap<String, &Value>,
) -> Vec<Msg> {
    if citation.as_object().is_none() {
        return vec![Msg::str(&format!("{where_} is not an object"))];
    }
    let tape = match citation.get("tape") {
        Some(Value::Str(digest)) => digest
            .as_str()
            .and_then(|name| tapes_by_digest.get(&name).copied()),
        _ => None,
    };
    let tape = match tape {
        Some(tape) => tape,
        None => {
            return vec![Msg::str(&format!(
                "{where_} names a tape outside the input set"
            ))]
        }
    };
    let events = tape
        .get("events")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let seq = match citation.get("seq") {
        Some(Value::Int(digits)) => int_in_range(digits, events.len()),
        _ => None,
    };
    let seq = match seq {
        Some(seq) => seq,
        None => return vec![Msg::str(&format!("{where_} names no event in its tape"))],
    };
    let event = &events[seq];
    let kind_ok = match (event.get("kind"), citation.get("kind")) {
        (Some(Value::Str(a)), Some(Value::Str(b))) => a == b,
        _ => false,
    };
    if !kind_ok {
        return vec![Msg::str(&format!(
            "{where_} states the wrong kind for its event"
        ))];
    }
    if matches!(citation.get("kind"), Some(v) if str_is(v, "message")) {
        let main_chain_user = matches!(event.get("role"), Some(v) if str_is(v, "user"))
            && matches!(event.get("sidechain"), Some(Value::Bool(false)));
        if !main_chain_user {
            return vec![Msg::str(&format!(
                "{where_} cites a message that is not a main-chain user turn"
            ))];
        }
    }
    Vec::new()
}

/// A non-negative integer literal below `len`, in
/// the Python's range check shape.
fn int_in_range(digits: &str, len: usize) -> Option<usize> {
    if digits.starts_with('-') || digits.len() > 19 {
        return None;
    }
    let value: usize = digits.parse().ok()?;
    if value < len {
        Some(value)
    } else {
        None
    }
}

fn str_is(value: &Value, name: &str) -> bool {
    matches!(value, Value::Str(text) if text.as_str().is_some_and(|s| s == name))
}

/// Python string order (code points) over the
/// crate's text.
fn cmp_text(a: &Text, b: &Text) -> std::cmp::Ordering {
    scalars_of(a.units()).cmp(&scalars_of(b.units()))
}
