//! RF-9 to RF-13, rescue moments. Ported file for
//! file from the Python suite.

use std::collections::HashMap;

use flighttape::value::{Text, Value};
use retroforge::digest::digest_of;
use retroforge::moments::rescue_moments;
use retroforge::proposals::{build_proposals, validate_proposals};
use retroforge::util::Msg;

const T0: &str = "2026-01-05T09:00:00.000Z";

fn j(text: &str) -> Value {
    flighttape::json::parse(text).expect("test JSON parses")
}

fn digest(value: &Value) -> String {
    digest_of(value, (1_000_000, 1_000_000)).expect("test value digests")
}

fn msg(seq: i64, role: &str, sidechain: bool) -> String {
    format!(
        "{{\"seq\":{seq},\"kind\":\"message\",\"sidechain\":{sidechain},\"timestamp\":\"{T0}\",\"role\":\"{role}\",\"text\":\"turn {seq}\"}}"
    )
}

fn call(seq: i64, tool: &str) -> String {
    format!(
        "{{\"seq\":{seq},\"kind\":\"tool_call\",\"sidechain\":false,\"timestamp\":\"{T0}\",\"call_id\":\"c{seq}\",\"tool\":\"{tool}\",\"input\":{{\"path\":\"a.py\"}}}}"
    )
}

fn result(seq: i64, call_id: &str, is_error: bool) -> String {
    format!(
        "{{\"seq\":{seq},\"kind\":\"tool_result\",\"sidechain\":false,\"timestamp\":\"{T0}\",\"call_id\":\"{call_id}\",\"is_error\":{is_error},\"text\":\"{}\"}}",
        if is_error { "boom" } else { "ok" }
    )
}

fn tape(events: &[String], started: &str) -> Value {
    j(&format!(
        "{{\"tape_version\":\"1.0\",\"run\":{{\"harness\":\"claude-code\",\"session_id\":\"s1\",\"started_at\":\"{started}\"}},\"events\":[{}],\"outcome\":{{\"status\":\"completed\",\"basis\":\"final-assistant-message\"}}}}",
        events.join(",")
    ))
}

fn rescue_tape(tool: &str, rescues: usize) -> Value {
    let mut events = vec![msg(0, "user", false)];
    let mut seq = 1;
    for _ in 0..rescues {
        events.push(call(seq, tool));
        seq += 1;
        events.push(msg(seq, "assistant", false));
        seq += 1;
        events.push(msg(seq, "user", false));
        seq += 1;
    }
    tape(&events, T0)
}

fn triples(tapes: &[&Value]) -> Vec<(Msg, Value, String)> {
    tapes
        .iter()
        .map(|tape| (Msg::str("a.json"), (*tape).clone(), digest(tape)))
        .collect()
}

fn proposal_ids(document: &Value) -> Vec<String> {
    match document.get("proposals") {
        Some(Value::Array(items)) => items
            .iter()
            .map(|p| match p.get("id") {
                Some(Value::Str(t)) => t.as_str().unwrap(),
                _ => panic!("id"),
            })
            .collect(),
        _ => panic!("proposals"),
    }
}

#[test]
fn a_rescue_needs_an_agent_action_since_the_last_user_turn() {
    let events = [
        msg(0, "user", false),
        msg(1, "user", false),
        call(2, "Edit"),
        msg(3, "user", false),
        msg(4, "assistant", false),
        msg(5, "user", false),
    ];
    let found = rescue_moments(&tape(&events, T0));
    let seqs: Vec<i64> = found.iter().map(|m| m.seq).collect();
    assert_eq!(seqs, [3, 5]);
    assert!(found
        .iter()
        .all(|m| m.tool.as_ref().and_then(|t| t.as_str()).as_deref() == Some("Edit")));
}

#[test]
fn the_first_user_message_is_never_a_rescue() {
    let events = [
        call(0, "Edit"),
        msg(1, "user", false),
        call(2, "Edit"),
        msg(3, "user", false),
    ];
    let found = rescue_moments(&tape(&events, T0));
    let seqs: Vec<i64> = found.iter().map(|m| m.seq).collect();
    assert_eq!(seqs, [3]);
}

#[test]
fn sidechain_turns_neither_rescue_nor_attribute() {
    let sidechain_call = format!(
        "{{\"seq\":2,\"kind\":\"tool_call\",\"sidechain\":true,\"timestamp\":\"{T0}\",\"call_id\":\"cx\",\"tool\":\"Bash\",\"input\":{{}}}}"
    );
    let events = [
        msg(0, "user", false),
        call(1, "Edit"),
        sidechain_call,
        msg(3, "user", true),
        msg(4, "user", false),
    ];
    let found = rescue_moments(&tape(&events, T0));
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].seq, 4);
    assert_eq!(
        found[0].tool.as_ref().and_then(|t| t.as_str()).as_deref(),
        Some("Edit")
    );
}

#[test]
fn an_assistant_message_alone_is_an_agent_action() {
    let events = [
        msg(0, "user", false),
        msg(1, "assistant", false),
        msg(2, "user", false),
        msg(3, "assistant", false),
        msg(4, "user", false),
    ];
    let found = rescue_moments(&tape(&events, T0));
    let seqs: Vec<i64> = found.iter().map(|m| m.seq).collect();
    assert_eq!(seqs, [2, 4]);
    assert!(found.iter().all(|m| m.tool.is_none()));
}

#[test]
fn two_attributed_rescues_emit_one_rescue_proposal() {
    let one = rescue_tape("Edit", 2);
    let digest = digest(&one);
    let document = build_proposals(&triples(&[&one]), &Text::from_str("AGENTS.md")).unwrap();
    assert_eq!(proposal_ids(&document), ["rules-edit-rescue-Edit"]);
    let proposal = match document.get("proposals") {
        Some(Value::Array(items)) => items[0].clone(),
        _ => panic!("proposals"),
    };
    let hunk_text = match proposal.get("hunk").and_then(|h| h.get("text")) {
        Some(Value::Str(t)) => t.as_str().unwrap(),
        _ => panic!("hunk text"),
    };
    assert!(
        hunk_text.contains("followed Edit work 2 times"),
        "{hunk_text}"
    );
    let citations = match proposal.get("citations") {
        Some(Value::Array(items)) => items.clone(),
        _ => panic!("citations"),
    };
    assert!(citations.iter().all(
        |c| matches!(c.get("kind"), Some(Value::Str(t)) if t.as_str().as_deref() == Some("message"))
    ));
    let mut by_digest: HashMap<String, &Value> = HashMap::new();
    by_digest.insert(digest, &one);
    assert!(validate_proposals(&document, &by_digest).is_empty());
    assert!(document.get("unattributed_rescues").is_none());
}

#[test]
fn below_threshold_and_unattributed_accounting() {
    let events = [
        msg(0, "user", false),
        msg(1, "assistant", false),
        msg(2, "user", false),
        call(3, "Edit"),
        msg(4, "user", false),
    ];
    let one = tape(&events, T0);
    let digest = digest(&one);
    let document = build_proposals(&triples(&[&one]), &Text::from_str("AGENTS.md")).unwrap();
    assert_eq!(document.get("proposals"), Some(&Value::Array(Vec::new())));
    assert_eq!(
        document.get("unattributed_rescues"),
        Some(&Value::Int("1".to_string()))
    );
    let mut by_digest: HashMap<String, &Value> = HashMap::new();
    by_digest.insert(digest, &one);
    assert!(validate_proposals(&document, &by_digest).is_empty());
    let mut bad = document.clone();
    *bad.get_mut("unattributed_rescues").unwrap() = Value::Int("-1".to_string());
    assert!(!validate_proposals(&bad, &by_digest).is_empty());
    let mut bad = document.clone();
    *bad.get_mut("unattributed_rescues").unwrap() = Value::Bool(true);
    assert!(!validate_proposals(&bad, &by_digest).is_empty());
}

#[test]
fn rescues_accumulate_across_tapes_and_follow_redo_in_order() {
    let first = rescue_tape("Edit", 1);
    let mut second = rescue_tape("Edit", 1);
    let run = second.get_mut("run").unwrap();
    *run.get_mut("session_id").unwrap() = Value::Str(Text::from_str("s2"));
    let tapes = triples(&[&first, &second]);
    let document = build_proposals(&tapes, &Text::from_str("AGENTS.md")).unwrap();
    assert_eq!(proposal_ids(&document), ["rules-edit-rescue-Edit"]);
    let citations = match document.get("proposals") {
        Some(Value::Array(items)) => match items[0].get("citations") {
            Some(Value::Array(citations)) => citations.len(),
            _ => panic!("citations"),
        },
        _ => panic!("proposals"),
    };
    assert_eq!(citations, 2);
}

#[test]
fn the_same_tape_twice_is_one_tape() {
    let one = rescue_tape("Edit", 1);
    let d = digest(&one);
    let entries = vec![
        (Msg::str("a.json"), one.clone(), d.clone()),
        (Msg::str("copy.json"), one.clone(), d),
    ];
    let document = build_proposals(&entries, &Text::from_str("AGENTS.md")).unwrap();
    assert_eq!(document.get("proposals"), Some(&Value::Array(Vec::new())));
    let two = rescue_tape("Edit", 2);
    let d2 = digest(&two);
    let entries = vec![
        (Msg::str("a.json"), two.clone(), d2.clone()),
        (Msg::str("copy.json"), two.clone(), d2),
    ];
    let document = build_proposals(&entries, &Text::from_str("AGENTS.md")).unwrap();
    let proposals = match document.get("proposals") {
        Some(Value::Array(items)) => items,
        _ => panic!("proposals"),
    };
    assert_eq!(proposals.len(), 1);
    match proposals[0].get("citations") {
        Some(Value::Array(citations)) => assert_eq!(citations.len(), 2),
        _ => panic!("citations"),
    }
}

#[test]
fn attribution_is_the_most_recent_tool_call() {
    let events = [
        msg(0, "user", false),
        call(1, "Edit"),
        call(2, "Bash"),
        msg(3, "user", false),
        call(4, "Edit"),
        msg(5, "user", false),
    ];
    let found = rescue_moments(&tape(&events, T0));
    assert_eq!(found.len(), 2);
    assert_eq!(found[0].seq, 3);
    assert_eq!(
        found[0].tool.as_ref().and_then(|t| t.as_str()).as_deref(),
        Some("Bash")
    );
    assert_eq!(found[1].seq, 5);
    assert_eq!(
        found[1].tool.as_ref().and_then(|t| t.as_str()).as_deref(),
        Some("Edit")
    );
}

#[test]
fn redo_precedes_rescue_for_one_tool_in_one_period() {
    let mut events = vec![msg(0, "user", false)];
    let mut seq = 1;
    for _ in 0..2 {
        events.push(call(seq, "Edit"));
        events.push(result(seq + 1, &format!("c{seq}"), true));
        seq += 2;
    }
    events.push(call(seq, "Edit"));
    seq += 1;
    for _ in 0..2 {
        events.push(msg(seq, "assistant", false));
        seq += 1;
        events.push(msg(seq, "user", false));
        seq += 1;
    }
    let one = tape(&events, T0);
    let digest = digest(&one);
    let document = build_proposals(&triples(&[&one]), &Text::from_str("AGENTS.md")).unwrap();
    assert_eq!(
        proposal_ids(&document),
        ["rules-edit-Edit", "rules-edit-rescue-Edit"]
    );
    let mut by_digest: HashMap<String, &Value> = HashMap::new();
    by_digest.insert(digest, &one);
    assert!(validate_proposals(&document, &by_digest).is_empty());
}

#[test]
fn a_tool_named_to_collide_still_yields_a_valid_file() {
    let mut events = vec![msg(0, "user", false)];
    let mut seq = 1;
    for tool in ["Edit", "rescue-Edit"] {
        for _ in 0..2 {
            events.push(call(seq, tool));
            seq += 1;
            events.push(msg(seq, "assistant", false));
            seq += 1;
            events.push(msg(seq, "user", false));
            seq += 1;
        }
    }
    for _ in 0..2 {
        events.push(call(seq, "rescue-Edit"));
        events.push(result(seq + 1, &format!("c{seq}"), true));
        seq += 2;
    }
    events.push(call(seq, "rescue-Edit"));
    let one = tape(&events, T0);
    let digest = digest(&one);
    let document = build_proposals(&triples(&[&one]), &Text::from_str("AGENTS.md")).unwrap();
    let ids = proposal_ids(&document);
    let unique: std::collections::HashSet<&String> = ids.iter().collect();
    assert_eq!(ids.len(), unique.len(), "{ids:?}");
    let mut by_digest: HashMap<String, &Value> = HashMap::new();
    by_digest.insert(digest, &one);
    assert!(validate_proposals(&document, &by_digest).is_empty());
}

#[test]
fn a_message_citation_must_name_an_operator_turn() {
    let one = rescue_tape("Edit", 2);
    let digest = digest(&one);
    let document = build_proposals(&triples(&[&one]), &Text::from_str("AGENTS.md")).unwrap();
    let mut forged = document.clone();
    let assistant_seq = match one.get("events") {
        Some(Value::Array(items)) => items
            .iter()
            .find(|e| {
                matches!(e.get("kind"), Some(Value::Str(t)) if t.as_str().as_deref() == Some("message"))
                    && matches!(e.get("role"), Some(Value::Str(t)) if t.as_str().as_deref() == Some("assistant"))
            })
            .and_then(|e| e.get("seq"))
            .cloned()
            .unwrap(),
        _ => panic!("events"),
    };
    let first = match forged.get_mut("proposals") {
        Some(Value::Array(items)) => &mut items[0],
        _ => panic!("proposals"),
    };
    let citation = match first.get_mut("citations") {
        Some(Value::Array(items)) => &mut items[0],
        _ => panic!("citations"),
    };
    *citation.get_mut("seq").unwrap() = assistant_seq;
    let mut by_digest: HashMap<String, &Value> = HashMap::new();
    by_digest.insert(digest, &one);
    let defects = validate_proposals(&forged, &by_digest);
    assert!(
        defects
            .iter()
            .any(|d| d.render_stderr().contains("not a main-chain user turn")),
        "{defects:?}"
    );
}
