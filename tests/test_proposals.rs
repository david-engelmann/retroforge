//! RF-1 to RF-8, evidence cited proposals. Ported
//! file for file from the Python suite; the CLI
//! cases run the compiled binary, as the family's
//! ports do.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;

use flighttape::value::{Text, Value};
use retroforge::digest::{canonical_bytes, digest_of, Budget};
use retroforge::evidence::gate_citations;
use retroforge::moments::redo_moments;
use retroforge::proposals::{build_proposals, validate_proposals};
use retroforge::tapes::{tape_defects, EVENT_KINDS};

const GEN: Budget = (1_000_000, 1_000_000);
const T0: &str = "2026-01-05T09:00:00.000Z";
const T1: &str = "2026-01-06T09:00:00.000Z";

fn j(text: &str) -> Value {
    flighttape::json::parse(text).expect("test JSON parses")
}

fn digest(value: &Value) -> String {
    digest_of(value, GEN).expect("test value digests")
}

fn call(seq: i64, call_id: &str, tool: &str, path: &str, ts: &str) -> String {
    format!(
        "{{\"seq\":{seq},\"kind\":\"tool_call\",\"sidechain\":false,\"timestamp\":\"{ts}\",\"call_id\":\"{call_id}\",\"tool\":\"{tool}\",\"input\":{{\"path\":\"{path}\"}}}}"
    )
}

fn result(seq: i64, call_id: &str, is_error: bool, ts: &str) -> String {
    format!(
        "{{\"seq\":{seq},\"kind\":\"tool_result\",\"sidechain\":false,\"timestamp\":\"{ts}\",\"call_id\":\"{call_id}\",\"is_error\":{},\"text\":\"{}\"}}",
        is_error,
        if is_error { "boom" } else { "ok" }
    )
}

fn redo_tape(started: &str, failures: usize) -> Value {
    let mut events = Vec::new();
    for i in 0..failures {
        events.push(call(
            2 * i as i64,
            &format!("c{i}"),
            "Read",
            "a.py",
            started,
        ));
        events.push(result(2 * i as i64 + 1, &format!("c{i}"), true, started));
    }
    events.push(call(2 * failures as i64, "cx", "Read", "a.py", started));
    j(&format!(
        "{{\"tape_version\":\"1.0\",\"run\":{{\"harness\":\"claude-code\",\"session_id\":\"s1\",\"started_at\":\"{started}\"}},\"events\":[{}],\"outcome\":{{\"status\":\"completed\",\"basis\":\"final-assistant-message\"}}}}",
        events.join(",")
    ))
}

fn set(value: &mut Value, key: &str, new: Value) {
    *value.get_mut(key).expect("key exists") = new;
}

fn event_at(tape: &mut Value, index: usize) -> &mut Value {
    match tape.get_mut("events") {
        Some(Value::Array(items)) => &mut items[index],
        _ => panic!("events is an array"),
    }
}

fn defects_text(defects: &[retroforge::util::Msg]) -> Vec<String> {
    defects.iter().map(|d| d.render_stderr()).collect()
}

fn any_defect(defects: &[retroforge::util::Msg], needle: &str) -> bool {
    defects_text(defects).iter().any(|d| d.contains(needle))
}

#[test]
fn the_pinned_digest_matches_the_flighttape_golden() {
    let tape = j(include_str!("fixtures/tape_synthetic.json"));
    assert_eq!(
        digest(&tape),
        "cc9c4e01fca418dcc33114caf75fae1f6bf0d401f297b5d61f6776903fa955a3"
    );
}

#[test]
fn a_redo_needs_the_same_input_and_an_error_between() {
    let tape = redo_tape(T0, 1);
    let moments = redo_moments(&tape);
    assert_eq!(moments.len(), 1);
    assert_eq!(moments[0].seq, 2);
    assert_eq!(moments[0].earlier_seq, 0);
    assert_eq!(moments[0].tool.as_str().as_deref(), Some("Read"));
    let mut changed = redo_tape(T0, 1);
    set(
        event_at(&mut changed, 2),
        "input",
        j("{\"path\":\"other.py\"}"),
    );
    assert!(redo_moments(&changed).is_empty());
    let mut succeeded = redo_tape(T0, 1);
    set(event_at(&mut succeeded, 1), "is_error", Value::Bool(false));
    assert!(redo_moments(&succeeded).is_empty());
    let mut reordered = redo_tape(T0, 1);
    set(
        event_at(&mut reordered, 0),
        "input",
        j("{\"path\":\"a.py\",\"x\":1}"),
    );
    set(
        event_at(&mut reordered, 2),
        "input",
        j("{\"x\":1,\"path\":\"a.py\"}"),
    );
    assert_eq!(redo_moments(&reordered).len(), 1);
}

fn triple(tape: &Value) -> (retroforge::util::Msg, Value, String) {
    (
        retroforge::util::Msg::str("a.json"),
        tape.clone(),
        digest(tape),
    )
}

#[test]
fn two_redo_moments_emit_one_proposal_with_resolving_citations() {
    let tape = redo_tape(T0, 2);
    let (_, _, digest) = triple(&tape);
    let document = build_proposals(&[triple(&tape)], &Text::from_str("AGENTS.md")).unwrap();
    assert_eq!(
        document.get("proposal_version").and_then(|v| match v {
            Value::Str(t) => t.as_str(),
            _ => None,
        }),
        Some("1.0".to_string())
    );
    let period = document.get("period").unwrap().clone();
    assert_eq!(period, j(&format!("{{\"from\":\"{T0}\",\"to\":\"{T0}\"}}")));
    let proposals = document.get("proposals").unwrap().clone();
    let proposals = match proposals {
        Value::Array(items) => items,
        _ => panic!("proposals is an array"),
    };
    assert_eq!(proposals.len(), 1);
    let proposal = &proposals[0];
    assert!(
        matches!(proposal.get("type"), Some(Value::Str(t)) if t.as_str().as_deref() == Some("rules-edit"))
    );
    assert!(
        matches!(proposal.get("hunk").and_then(|h| h.get("action")), Some(Value::Str(t)) if t.as_str().as_deref() == Some("add"))
    );
    let hunk_text = match proposal.get("hunk").and_then(|h| h.get("text")) {
        Some(Value::Str(t)) => t.as_str().unwrap(),
        _ => panic!("hunk text"),
    };
    assert!(hunk_text.contains("Read"));
    let citations = match proposal.get("citations") {
        Some(Value::Array(items)) => items.clone(),
        _ => panic!("citations"),
    };
    assert_eq!(citations.len(), 2);
    let mut by_digest: HashMap<String, &Value> = HashMap::new();
    by_digest.insert(digest.clone(), &tape);
    assert!(validate_proposals(&document, &by_digest).is_empty());
    let stub = proposal.get("registry").unwrap();
    let experiment = match stub.get("experiment_id") {
        Some(Value::Str(t)) => t.as_str().unwrap(),
        _ => panic!("experiment id"),
    };
    assert!(experiment.starts_with("exp-"));
    assert_eq!(stub.get("baseline_period").unwrap(), &period);
    let again = build_proposals(&[triple(&tape)], &Text::from_str("AGENTS.md")).unwrap();
    assert_eq!(
        retroforge::emit::dumps_view(&again, retroforge::emit::Mode::Strict).unwrap(),
        retroforge::emit::dumps_view(&document, retroforge::emit::Mode::Strict).unwrap()
    );
}

#[test]
fn below_the_threshold_no_proposal_is_emitted() {
    let tape = redo_tape(T0, 1);
    let document = build_proposals(&[triple(&tape)], &Text::from_str("AGENTS.md")).unwrap();
    assert_eq!(document.get("proposals"), Some(&Value::Array(Vec::new())));
}

#[test]
fn the_validator_names_every_broken_citation() {
    let tape = redo_tape(T0, 2);
    let digest = digest(&tape);
    let mut by_digest: HashMap<String, &Value> = HashMap::new();
    by_digest.insert(digest.clone(), &tape);

    let mut document = build_proposals(&[triple(&tape)], &Text::from_str("AGENTS.md")).unwrap();
    let first = match document.get_mut("proposals") {
        Some(Value::Array(items)) => &mut items[0],
        _ => panic!("proposals"),
    };
    let citations = match first.get_mut("citations") {
        Some(Value::Array(items)) => items,
        _ => panic!("citations"),
    };
    citations.truncate(1);
    let defects = validate_proposals(&document, &by_digest);
    assert!(any_defect(&defects, "fewer than two"), "{defects:?}");

    let mut document = build_proposals(&[triple(&tape)], &Text::from_str("AGENTS.md")).unwrap();
    let first = match document.get_mut("proposals") {
        Some(Value::Array(items)) => &mut items[0],
        _ => panic!("proposals"),
    };
    let citation = match first.get_mut("citations") {
        Some(Value::Array(items)) => &mut items[0],
        _ => panic!("citations"),
    };
    set(
        citation,
        "tape",
        Value::Str(Text::from_str(&"0".repeat(64))),
    );
    let defects = validate_proposals(&document, &by_digest);
    assert!(any_defect(&defects, "outside the input set"));

    let mut document = build_proposals(&[triple(&tape)], &Text::from_str("AGENTS.md")).unwrap();
    let first = match document.get_mut("proposals") {
        Some(Value::Array(items)) => &mut items[0],
        _ => panic!("proposals"),
    };
    let citation = match first.get_mut("citations") {
        Some(Value::Array(items)) => &mut items[0],
        _ => panic!("citations"),
    };
    set(citation, "kind", Value::Str(Text::from_str("note")));
    let defects = validate_proposals(&document, &by_digest);
    assert!(any_defect(&defects, "wrong kind"));

    let mut document = build_proposals(&[triple(&tape)], &Text::from_str("AGENTS.md")).unwrap();
    let first = match document.get_mut("proposals") {
        Some(Value::Array(items)) => &mut items[0],
        _ => panic!("proposals"),
    };
    let citation = match first.get_mut("citations") {
        Some(Value::Array(items)) => &mut items[0],
        _ => panic!("citations"),
    };
    set(citation, "seq", Value::Int("99".to_string()));
    let defects = validate_proposals(&document, &by_digest);
    assert!(any_defect(&defects, "names no event"));
}

// The CLI cases, through the compiled binary.

fn tempdir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("rf-proposals-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn write(dir: &Path, name: &str, value: &Value) {
    std::fs::write(dir.join(name), tape_text(value)).unwrap();
}

fn write_text(dir: &Path, name: &str, text: &str) {
    std::fs::write(dir.join(name), text).unwrap();
}

fn tape_text(value: &Value) -> String {
    // The Python tests write tapes with json.dumps:
    // the compact form with ", " and ": " separators.
    let bytes = retroforge::emit::dumps_line(value, retroforge::emit::Mode::Strict).unwrap();
    String::from_utf8(bytes).unwrap()
}

fn run(args: &[String]) -> (i32, String, String) {
    let output = Command::new(env!("CARGO_BIN_EXE_retroforge"))
        .args(args)
        .output()
        .unwrap();
    (
        output.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&output.stdout).into_owned(),
        String::from_utf8_lossy(&output.stderr).into_owned(),
    )
}

fn s(text: &str) -> String {
    text.to_string()
}

#[test]
fn cli_propose_over_a_mixed_directory() {
    let dir = tempdir("mixed");
    write(&dir, "a.json", &redo_tape(T0, 2));
    write_text(&dir, "b.json", "{not json");
    write_text(&dir, "notes.txt", "ignored");
    let out = dir.join("proposal.json");
    let (code, _, err) = run(&[
        s("propose"),
        s(dir.to_str().unwrap()),
        s("-o"),
        s(out.to_str().unwrap()),
    ]);
    assert_eq!(code, 0, "{err}");
    assert!(err.contains("b.json is excluded"), "{err}");
    let document = j(&std::fs::read_to_string(&out).unwrap());
    match document.get("proposals") {
        Some(Value::Array(items)) => assert_eq!(items.len(), 1),
        _ => panic!("proposals"),
    }
}

#[test]
fn cli_no_valid_tapes_writes_nothing() {
    let dir = tempdir("novalid");
    write_text(&dir, "b.json", "{not json");
    let out = dir.join("proposal.json");
    let (code, _, err) = run(&[
        s("propose"),
        s(dir.to_str().unwrap()),
        s("-o"),
        s(out.to_str().unwrap()),
    ]);
    assert_eq!(code, 1);
    assert!(err.contains("no valid tapes"), "{err}");
    assert!(!out.exists());
    let missing = dir.join("missing");
    let (code, _, _) = run(&[s("propose"), s(missing.to_str().unwrap())]);
    assert_eq!(code, 1);
}

#[test]
fn cli_propose_to_stdout_is_byte_deterministic() {
    let dir = tempdir("deterministic");
    write(&dir, "a.json", &redo_tape(T0, 2));
    write(&dir, "z.json", &redo_tape(T1, 2));
    let (code, first, _) = run(&[s("propose"), s(dir.to_str().unwrap())]);
    assert_eq!(code, 0);
    let (code, second, _) = run(&[s("propose"), s(dir.to_str().unwrap())]);
    assert_eq!(code, 0);
    assert_eq!(first, second);
    let document = j(&first);
    assert_eq!(
        document.get("period").unwrap(),
        &j(&format!("{{\"from\":\"{T0}\",\"to\":\"{T1}\"}}"))
    );
    let proposals = match document.get("proposals") {
        Some(Value::Array(items)) => items,
        _ => panic!("proposals"),
    };
    match proposals[0].get("citations") {
        Some(Value::Array(items)) => assert_eq!(items.len(), 4),
        _ => panic!("citations"),
    }
}

#[test]
fn canonical_form_1_battery_against_the_reference_values() {
    let cases: Vec<(Value, &[u8])> = vec![
        (j("42"), b"42"),
        (j("150.0"), b"150"),
        (j("0.000001"), b"0.000001"),
        (j("1e-7"), b"1e-7"),
        (j("1e21"), b"1e+21"),
        (j("123456789.123456789"), b"123456789.12345679"),
        (j("5e-324"), b"5e-324"),
        (j("-0.0"), b"0"),
        (
            j("{\"b\":1e21,\"a\":[0.000001,2.5]}"),
            b"{\"a\":[0.000001,2.5],\"b\":1e+21}",
        ),
        (
            j("{\"\\uffff\":1,\"\\ud800\\udc00\":2}"),
            "{\"𐀀\":2,\"￿\":1}".as_bytes(),
        ),
    ];
    for (value, expected) in cases {
        assert_eq!(
            canonical_bytes(&value, GEN).unwrap(),
            expected,
            "value {value:?}"
        );
    }
}

#[test]
fn the_kind_enum_is_the_formats_eight() {
    let mut sorted: Vec<&str> = EVENT_KINDS.to_vec();
    sorted.sort();
    let mut expected = vec![
        "session_start",
        "message",
        "tool_call",
        "tool_result",
        "diff",
        "spend",
        "note",
        "session_end",
    ];
    expected.sort();
    assert_eq!(sorted, expected);
}

#[test]
fn pairing_and_shape_defects_exclude_a_tape() {
    let mut dangling = redo_tape(T0, 1);
    let ghost = result(0, "ghost", true, T0);
    if let Some(Value::Array(items)) = dangling.get_mut("events") {
        items.insert(0, j(&ghost));
        for (index, event) in items.iter_mut().enumerate() {
            set(event, "seq", Value::Int(index.to_string()));
        }
    }
    assert!(any_defect(&tape_defects(&dangling), "answers no earlier"));
    let mut listed = redo_tape(T0, 1);
    set(event_at(&mut listed, 0), "tool", j("[\"Read\"]"));
    assert!(any_defect(&tape_defects(&listed), "tool must be"));
    let mut missing = redo_tape(T0, 1);
    if let Value::Object(entries) = event_at(&mut missing, 0) {
        entries.retain(|(key, _)| key.as_str().as_deref() != Some("tool"));
    }
    assert!(any_defect(&tape_defects(&missing), "tool must be"));
    let mut dup = redo_tape(T0, 1);
    set(
        event_at(&mut dup, 2),
        "call_id",
        Value::Str(Text::from_str("c0")),
    );
    assert!(any_defect(&tape_defects(&dup), "used twice"));
    let mut bad_seq = redo_tape(T0, 1);
    set(event_at(&mut bad_seq, 0), "seq", Value::Bool(true));
    assert!(any_defect(&tape_defects(&bad_seq), "seq is not 0"));
    let mut bad_error = redo_tape(T0, 1);
    set(
        event_at(&mut bad_error, 1),
        "is_error",
        Value::Int("1".to_string()),
    );
    assert!(any_defect(&tape_defects(&bad_error), "is_error must be"));
    let future = j(&format!(
        "{{\"tape_version\":\"2.0\",\"run\":{{}},\"events\":[{}]}}",
        call(0, "c0", "Read", "a.py", T0)
    ));
    assert!(any_defect(&tape_defects(&future), "tape_version"));
}

#[test]
fn an_error_must_answer_the_earlier_call_before_the_repeat() {
    let tape = j(&format!(
        "{{\"tape_version\":\"1.0\",\"run\":{{\"started_at\":\"{T0}\"}},\"events\":[{}],\"outcome\":{{\"status\":\"completed\",\"basis\":\"final-assistant-message\"}}}}",
        [
            call(0, "a", "Read", "a.py", T0),
            result(1, "a", false, T0),
            call(2, "b", "Read", "a.py", T0),
            result(3, "b", true, T0),
            call(4, "c", "Read", "a.py", T0),
        ]
        .join(",")
    ));
    let moments = redo_moments(&tape);
    assert_eq!(moments.len(), 1);
    assert_eq!(moments[0].seq, 4);
    assert_eq!(moments[0].earlier_seq, 2);
    let late = j(&format!(
        "{{\"tape_version\":\"1.0\",\"run\":{{\"started_at\":\"{T0}\"}},\"events\":[{}],\"outcome\":{{\"status\":\"completed\",\"basis\":\"final-assistant-message\"}}}}",
        [
            call(0, "a", "Read", "a.py", T0),
            call(1, "b", "Read", "a.py", T0),
            result(2, "a", true, T0),
        ]
        .join(",")
    ));
    assert!(redo_moments(&late).is_empty());
}

#[test]
fn the_experiment_id_moves_with_the_baseline_period() {
    let redo = redo_tape(T0, 2);
    let quiet = j(&format!(
        "{{\"tape_version\":\"1.0\",\"run\":{{\"harness\":\"claude-code\",\"session_id\":\"q\",\"started_at\":\"{T1}\"}},\"events\":[{}],\"outcome\":{{\"status\":\"completed\",\"basis\":\"final-assistant-message\"}}}}",
        call(0, "only", "Write", "a.py", T1)
    ));
    let base = build_proposals(&[triple(&redo)], &Text::from_str("AGENTS.md")).unwrap();
    let mut second_triple = (
        retroforge::util::Msg::str("z.json"),
        quiet.clone(),
        digest(&quiet),
    );
    second_triple.0 = retroforge::util::Msg::str("z.json");
    let wider = build_proposals(
        &[triple(&redo), second_triple],
        &Text::from_str("AGENTS.md"),
    )
    .unwrap();
    let first = match base.get("proposals") {
        Some(Value::Array(items)) => items[0].clone(),
        _ => panic!("proposals"),
    };
    let second = match wider.get("proposals") {
        Some(Value::Array(items)) => items[0].clone(),
        _ => panic!("proposals"),
    };
    assert_eq!(first.get("citations"), second.get("citations"));
    assert_eq!(
        second
            .get("registry")
            .and_then(|r| r.get("baseline_period"))
            .unwrap(),
        &j(&format!("{{\"from\":\"{T0}\",\"to\":\"{T1}\"}}"))
    );
    assert_ne!(
        first.get("registry").and_then(|r| r.get("experiment_id")),
        second.get("registry").and_then(|r| r.get("experiment_id"))
    );
}

#[test]
fn the_validator_checks_period_registry_and_duplicate_ids() {
    let tape = redo_tape(T0, 2);
    let digest = digest(&tape);
    let mut by_digest: HashMap<String, &Value> = HashMap::new();
    by_digest.insert(digest.clone(), &tape);

    let mut document = build_proposals(&[triple(&tape)], &Text::from_str("AGENTS.md")).unwrap();
    let first = match document.get_mut("proposals") {
        Some(Value::Array(items)) => &mut items[0],
        _ => panic!("proposals"),
    };
    let registry = first.get_mut("registry").unwrap();
    set(registry, "experiment_id", Value::Str(Text::from_str("x")));
    let defects = validate_proposals(&document, &by_digest);
    assert!(any_defect(&defects, "experiment_id"));

    let mut document = build_proposals(&[triple(&tape)], &Text::from_str("AGENTS.md")).unwrap();
    if let Value::Object(entries) = &mut document {
        entries.retain(|(key, _)| key.as_str().as_deref() != Some("period"));
    }
    let defects = validate_proposals(&document, &by_digest);
    assert!(any_defect(&defects, "period"));

    let mut document = build_proposals(&[triple(&tape)], &Text::from_str("AGENTS.md")).unwrap();
    let clone = match document.get("proposals") {
        Some(Value::Array(items)) => items[0].clone(),
        _ => panic!("proposals"),
    };
    if let Some(Value::Array(items)) = document.get_mut("proposals") {
        items.push(clone);
    }
    let defects = validate_proposals(&document, &by_digest);
    assert!(any_defect(&defects, "used twice"));
}

#[test]
fn gate_citations_refuse_until_the_format_is_pinned() {
    let error = gate_citations("pack.json").unwrap_err();
    assert!(error.0.contains("not pinned yet"), "{}", error.0);
}

#[test]
fn cli_the_output_never_overwrites_an_input_tape() {
    let dir = tempdir("overwrite");
    let tape = redo_tape(T0, 2);
    write(&dir, "a.json", &tape);
    let tape_path = dir.join("a.json");
    let before = std::fs::read_to_string(&tape_path).unwrap();
    let (code, _, err) = run(&[
        s("propose"),
        s(dir.to_str().unwrap()),
        s("-o"),
        s(tape_path.to_str().unwrap()),
    ]);
    assert_eq!(code, 1);
    assert!(err.contains("never writes over"), "{err}");
    assert_eq!(std::fs::read_to_string(&tape_path).unwrap(), before);
}

#[test]
fn cli_a_write_failure_is_a_named_error() {
    let dir = tempdir("writefail");
    write(&dir, "a.json", &redo_tape(T0, 2));
    let (code, _, err) = run(&[
        s("propose"),
        s(dir.to_str().unwrap()),
        s("-o"),
        s(dir.to_str().unwrap()),
    ]);
    assert_eq!(code, 1);
    assert!(err.contains("cannot write the proposal file"), "{err}");
}

#[test]
fn cli_orderings_are_filename_then_tool() {
    let dir = tempdir("orderings");
    let mut zeta = redo_tape(T0, 0);
    let events = [
        call(0, "z0", "Zeta", "a.py", T0),
        result(1, "z0", true, T0),
        call(2, "z1", "Zeta", "a.py", T0),
        result(3, "z1", true, T0),
        call(4, "z2", "Zeta", "a.py", T0),
    ];
    set(&mut zeta, "events", j(&format!("[{}]", events.join(","))));
    write(&dir, "a.json", &zeta);
    let mut alpha = redo_tape(T0, 0);
    let events = [
        call(0, "a0", "Alpha", "a.py", T0),
        result(1, "a0", true, T0),
        call(2, "a1", "Alpha", "a.py", T0),
        result(3, "a1", true, T0),
        call(4, "a2", "Alpha", "a.py", T0),
    ];
    set(&mut alpha, "events", j(&format!("[{}]", events.join(","))));
    write(&dir, "z.json", &alpha);
    let (code, out, _) = run(&[s("propose"), s(dir.to_str().unwrap())]);
    assert_eq!(code, 0);
    let document = j(&out);
    let ids: Vec<String> = match document.get("proposals") {
        Some(Value::Array(items)) => items
            .iter()
            .map(|p| match p.get("id") {
                Some(Value::Str(t)) => t.as_str().unwrap(),
                _ => panic!("id"),
            })
            .collect(),
        _ => panic!("proposals"),
    };
    assert_eq!(ids, ["rules-edit-Alpha", "rules-edit-Zeta"]);
}

#[test]
fn cli_citations_follow_filename_order() {
    let dir = tempdir("citeorder");
    let first = redo_tape(T0, 1);
    let mut second = redo_tape(T0, 1);
    let run_obj = second.get_mut("run").unwrap();
    set(run_obj, "session_id", Value::Str(Text::from_str("other")));
    write(&dir, "a.json", &first);
    write(&dir, "z.json", &second);
    let (code, out, _) = run(&[s("propose"), s(dir.to_str().unwrap())]);
    assert_eq!(code, 0);
    let document = j(&out);
    let cited: Vec<String> = match document.get("proposals") {
        Some(Value::Array(items)) => match items[0].get("citations") {
            Some(Value::Array(citations)) => citations
                .iter()
                .map(|c| match c.get("tape") {
                    Some(Value::Str(t)) => t.as_str().unwrap(),
                    _ => panic!("tape"),
                })
                .collect(),
            _ => panic!("citations"),
        },
        _ => panic!("proposals"),
    };
    assert_eq!(cited, [digest(&first), digest(&second)]);
}

#[test]
fn cli_a_rerun_may_overwrite_its_own_proposal() {
    let dir = tempdir("rerun");
    write(&dir, "a.json", &redo_tape(T0, 2));
    let out = dir.join("proposal.json");
    let args = [
        s("propose"),
        s(dir.to_str().unwrap()),
        s("-o"),
        s(out.to_str().unwrap()),
    ];
    assert_eq!(run(&args).0, 0);
    let (code, _, err) = run(&args);
    assert_eq!(code, 0);
    assert!(err.contains("proposal.json is excluded"), "{err}");
}

#[test]
fn an_unhashable_kind_is_a_defect_not_a_crash() {
    let mut tape = redo_tape(T0, 1);
    set(event_at(&mut tape, 0), "kind", j("[\"tool_call\"]"));
    let defects = tape_defects(&tape);
    assert!(any_defect(&defects, "kind is not in the format enum"));
}

#[test]
fn canonical_form_renders_big_integers_exactly() {
    assert_eq!(
        canonical_bytes(&j("1152921504606846977"), GEN).unwrap(),
        b"1152921504606846977"
    );
}

#[test]
fn identical_calls_cite_the_earliest_errored_call() {
    let tape = redo_tape(T0, 3);
    let moments = redo_moments(&tape);
    let seqs: Vec<i64> = moments.iter().map(|m| m.seq).collect();
    assert_eq!(seqs, [2, 4, 6]);
    assert!(moments.iter().all(|m| m.earlier_seq == 0));
}

#[test]
fn a_failed_write_leaves_the_earlier_proposal_intact() {
    // The Python patches os.replace to fail; here the
    // replace is failed for real by turning the
    // target into a non-empty directory between runs,
    // which refuses the same replace step.
    let dir = tempdir("atomic");
    write(&dir, "a.json", &redo_tape(T0, 2));
    let out = dir.join("proposal.json");
    let args = [
        s("propose"),
        s(dir.to_str().unwrap()),
        s("-o"),
        s(out.to_str().unwrap()),
    ];
    assert_eq!(run(&args).0, 0);
    let before = std::fs::read_to_string(&out).unwrap();
    std::fs::remove_file(&out).unwrap();
    std::fs::create_dir(&out).unwrap();
    std::fs::write(out.join("marker"), &before).unwrap();
    let (code, _, _) = run(&args);
    assert_eq!(code, 1);
    assert_eq!(std::fs::read_to_string(out.join("marker")).unwrap(), before);
    let leftovers: Vec<String> = std::fs::read_dir(&dir)
        .unwrap()
        .filter_map(|entry| {
            let name = entry.unwrap().file_name().to_string_lossy().into_owned();
            if name.starts_with("proposal.json.") && name.ends_with(".tmp") {
                Some(name)
            } else {
                None
            }
        })
        .collect();
    assert!(leftovers.is_empty(), "{leftovers:?}");
}
