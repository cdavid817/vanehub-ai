use super::memory_extraction::{
    parse_memory_actions, MemoryActionKind, MAX_MEMORY_ACTIONS, MEMORY_ACTIONS_INSTRUCTION,
};
use super::{AgentRuntimeDomainError, MemoryType};

#[test]
fn a_well_formed_response_yields_every_action_kind() {
    let parsed = parse_memory_actions(
        r#"[
          {"action":"create","name":"user-role","description":"The user is a data scientist","type":"user","body":"Frame answers around observability."},
          {"action":"update","name":"npm-only","description":"npm, never pnpm","body":"pnpm layout breaks the katex chunk split."},
          {"action":"delete","name":"stale-fact"}
        ]"#,
    )
    .expect("parse");

    assert!(parsed.rejections.is_empty());
    assert_eq!(parsed.actions.len(), 3);
    assert_eq!(parsed.actions[0].kind, MemoryActionKind::Create);
    assert_eq!(parsed.actions[0].memory_type, Some(MemoryType::User));
    assert_eq!(parsed.actions[1].kind, MemoryActionKind::Update);
    // An untyped action is legal; the taxonomy degrades rather than rejecting.
    assert_eq!(parsed.actions[1].memory_type, None);
    assert_eq!(parsed.actions[2].kind, MemoryActionKind::Delete);
    // A delete needs nothing but the name of the memory it retracts.
    assert_eq!(parsed.actions[2].body, None);
    assert_eq!(parsed.actions[2].description, None);
}

#[test]
fn an_empty_array_is_a_successful_extraction_that_found_nothing() {
    // Distinct from a malfunction: the model deciding nothing is worth remembering is the expected
    // outcome most of the time, and must not be logged as a failure.
    let parsed = parse_memory_actions("[]").expect("parse");

    assert!(parsed.actions.is_empty());
    assert!(parsed.rejections.is_empty());
}

#[test]
fn one_bad_action_does_not_discard_the_good_ones() {
    let parsed = parse_memory_actions(
        r#"[
          {"action":"create","name":"good-one","description":"Kept","body":"Body."},
          {"action":"summarize","name":"unknown-verb","description":"d","body":"b"},
          {"action":"create","name":"../escape","description":"d","body":"b"},
          {"action":"create","description":"no name","body":"b"},
          {"action":"create","name":"no-body","description":"d"},
          {"action":"create","name":"no-description","body":"b"},
          {"action":"create","name":"good-two","description":"Also kept","body":"Body."}
        ]"#,
    )
    .expect("parse");

    let kept = parsed
        .actions
        .iter()
        .map(|action| action.name.as_str())
        .collect::<Vec<_>>();
    assert_eq!(kept, vec!["good-one", "good-two"]);
    let reasons = parsed
        .rejections
        .iter()
        .map(|rejection| (rejection.index, rejection.reason))
        .collect::<Vec<_>>();
    assert_eq!(
        reasons,
        vec![
            (1, "unknown-action"),
            (2, "invalid-name"),
            (3, "missing-name"),
            (4, "missing-body"),
            (5, "missing-description"),
        ]
    );
}

#[test]
fn a_name_that_escapes_the_memory_directory_is_rejected_not_written() {
    // Built through `json!` rather than string interpolation: a hand-escaped backslash produces
    // invalid JSON, which fails the whole response and would exercise the malfunction path by
    // accident instead of the per-action rejection path this test is about.
    for escaping in [
        "../escape",
        "nested/inner",
        "with\\slash",
        "con",
        "trailing.",
    ] {
        let response = serde_json::json!([{
            "action": "create",
            "name": escaping,
            "description": "d",
            "body": "b",
        }])
        .to_string();
        let parsed = parse_memory_actions(&response).expect("parse");

        assert!(
            parsed.actions.is_empty(),
            "expected {escaping:?} to be rejected"
        );
        assert_eq!(parsed.rejections.len(), 1);
    }
}

#[test]
fn a_response_that_is_not_an_array_is_a_malfunction() {
    for malformed in [
        "",
        "I could not find anything worth remembering.",
        "{\"action\":\"create\"}",
        "[{\"action\": ",
    ] {
        assert_eq!(
            parse_memory_actions(malformed),
            Err(AgentRuntimeDomainError::InvalidMemoryValue(
                "action response"
            )),
            "expected {malformed:?} to be a malfunction"
        );
    }
}

#[test]
fn prose_and_code_fences_around_the_array_are_tolerated() {
    // Models wrap structured output often enough that failing on the wrapper would misreport a
    // perfectly usable response as a malfunction.
    let parsed = parse_memory_actions(
        "Here is what I found:\n```json\n[{\"action\":\"delete\",\"name\":\"stale\"}]\n```\nDone.",
    )
    .expect("parse");

    assert_eq!(parsed.actions.len(), 1);
    assert_eq!(parsed.actions[0].name, "stale");
}

#[test]
fn a_bracket_inside_a_string_does_not_end_the_array_early() {
    let parsed = parse_memory_actions(
        r#"[{"action":"create","name":"brackets","description":"Uses vec![1] syntax","body":"Body with ] and \" inside."}]"#,
    )
    .expect("parse");

    assert_eq!(parsed.actions.len(), 1);
    assert_eq!(
        parsed.actions[0].description.as_deref(),
        Some("Uses vec![1] syntax")
    );
}

#[test]
fn the_action_count_is_capped() {
    let elements = (0..MAX_MEMORY_ACTIONS + 3)
        .map(|index| {
            format!(r#"{{"action":"create","name":"memory-{index}","description":"d","body":"b"}}"#)
        })
        .collect::<Vec<_>>()
        .join(",");
    let parsed = parse_memory_actions(&format!("[{elements}]")).expect("parse");

    assert_eq!(parsed.actions.len(), MAX_MEMORY_ACTIONS);
    assert_eq!(parsed.rejections.len(), 3);
    assert!(parsed
        .rejections
        .iter()
        .all(|rejection| rejection.reason == "action-limit"));
}

#[test]
fn the_instruction_names_every_shape_the_parser_accepts() {
    // The prompt and the parser drift apart silently otherwise: the model would be told to send a
    // field the parser ignores, or never told about one it requires.
    for expected in [
        "\"create\"",
        "\"update\"",
        "\"delete\"",
        "\"name\"",
        "\"description\"",
        "\"type\"",
        "\"body\"",
        "user",
        "feedback",
        "project",
        "reference",
    ] {
        assert!(
            MEMORY_ACTIONS_INSTRUCTION.contains(expected),
            "instruction must mention {expected}"
        );
    }
}
