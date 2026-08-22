// Included through `#[path]` from dto.rs.
//
// These assert the wire shape itself: field names, casing, and that a `None` crosses as `null`
// rather than disappearing. A field that silently vanishes reads on the other side as "the backend
// does not know", which is a different statement from "there is nothing here".
use super::*;

fn snapshot() -> CliEnvironmentSnapshotDto {
    CliEnvironmentSnapshotDto {
        schema_version: 1,
        agent_id: "claude-code".to_string(),
        scope: "local-desktop".to_string(),
        overall_state: "ready".to_string(),
        freshness: "fresh".to_string(),
        environment_fingerprint: "fingerprint-a".to_string(),
        installations: vec![CliInstallationDto {
            id: "a".to_string(),
            executable_path: "/usr/local/bin/claude".to_string(),
            canonical_path: None,
            alias_paths: Vec::new(),
            target_missing: false,
            reported_version: Some("1.2.0".to_string()),
            source_id: Some("npm".to_string()),
            source_kind: "npm".to_string(),
            source_confidence: "inferred".to_string(),
            path_priority: Some(0),
            environment_origin: "path".to_string(),
            executable_status: "healthy".to_string(),
        }],
        path_selected_installation_id: Some("a".to_string()),
        recommended_installation_id: Some("a".to_string()),
        discovery: "found-one".to_string(),
        executable: "healthy".to_string(),
        authentication: "unknown".to_string(),
        readiness: "unknown".to_string(),
        compatibility: "unknown".to_string(),
        update: "up-to-date".to_string(),
        conflicts: Vec::new(),
        sources: Vec::new(),
        allowed_actions: Vec::new(),
        last_mutation: None,
        last_operation_id: None,
        checked_at: None,
    }
}

#[test]
fn a_snapshot_serializes_with_camel_case_field_names() {
    let value = serde_json::to_value(snapshot()).expect("serializes");
    let object = value.as_object().expect("object");

    assert!(object.contains_key("agentId"));
    assert!(object.contains_key("environmentFingerprint"));
    assert!(object.contains_key("allowedActions"));
    // The two installation identities are distinct fields; collapsing them would hide exactly the
    // case where PATH runs something other than what the backend would act on.
    assert!(object.contains_key("pathSelectedInstallationId"));
    assert!(object.contains_key("recommendedInstallationId"));
    assert!(!object.contains_key("activeInstallationId"));
}

#[test]
fn an_absent_value_crosses_as_null_rather_than_being_omitted() {
    let value = serde_json::to_value(snapshot()).expect("serializes");
    let object = value.as_object().expect("object");

    // "There is no last mutation" and "the backend did not tell us" must not look the same.
    assert_eq!(object.get("lastMutation"), Some(&serde_json::Value::Null));
    assert_eq!(object.get("checkedAt"), Some(&serde_json::Value::Null));
    assert_eq!(
        object.get("lastOperationId"),
        Some(&serde_json::Value::Null)
    );
}

#[test]
fn enums_cross_as_their_stable_wire_strings() {
    let value = serde_json::to_value(snapshot()).expect("serializes");
    assert_eq!(value["freshness"], "fresh");
    assert_eq!(value["update"], "up-to-date");
    assert_eq!(value["installations"][0]["sourceConfidence"], "inferred");
    assert_eq!(value["installations"][0]["executableStatus"], "healthy");
}

#[test]
fn a_conflict_carries_everything_the_ui_needs_to_decide_without_parsing() {
    let conflict = CliConflictDto {
        kind: "path-shadowing".to_string(),
        severity: "blocking".to_string(),
        installation_ids: vec!["a".to_string(), "b".to_string()],
        blocks_mutation: true,
        blocks_launch: false,
        reason_code: "cli.conflict.path-shadowing".to_string(),
    };
    let value = serde_json::to_value(conflict).expect("serializes");

    assert_eq!(value["installationIds"].as_array().expect("array").len(), 2);
    assert_eq!(value["blocksMutation"], true);
    assert_eq!(value["blocksLaunch"], false);
    // The frontend localizes from the code; it never reads `kind` or a message string.
    assert_eq!(value["reasonCode"], "cli.conflict.path-shadowing");
}

#[test]
fn a_plan_exposes_the_exact_argv_that_will_run() {
    let plan = CliActionPlanDto {
        id: "plan-1".to_string(),
        revision: 1,
        agent_id: "claude-code".to_string(),
        action: "upgrade".to_string(),
        source_id: "npm".to_string(),
        installation_id: None,
        current_version: Some("1.2.0".to_string()),
        target_version: Some("1.3.0".to_string()),
        channel: Some("stable".to_string()),
        command_preview: CliCommandPreviewDto {
            program: "npm".to_string(),
            args: vec![
                "install".to_string(),
                "--global".to_string(),
                "@anthropic-ai/claude-code@1.3.0".to_string(),
            ],
        },
        preconditions: vec!["source-executable-available".to_string()],
        warnings: Vec::new(),
        requires_elevation: false,
        requires_network: true,
        state: "draft".to_string(),
        created_at: "1970-01-01T00:16:40+00:00".to_string(),
        expires_at: "1970-01-01T00:26:40+00:00".to_string(),
    };
    let value = serde_json::to_value(plan).expect("serializes");

    // Argv, never a shell string: each argument is one value the user can read.
    let args = value["commandPreview"]["args"]
        .as_array()
        .expect("args array");
    assert_eq!(args.len(), 3);
    assert_eq!(args[2], "@anthropic-ai/claude-code@1.3.0");
    // The revision the user saw is on the plan, because execution submits it back.
    assert_eq!(value["revision"], 1);
}

#[test]
fn a_bulk_plan_lists_what_was_left_out_and_why() {
    let plan = CliBulkActionPlanDto {
        id: "bulk-1".to_string(),
        revision: 1,
        items: vec![CliBulkActionItemDto {
            agent_id: "claude-code".to_string(),
            plan_id: "plan-1".to_string(),
            source_id: "npm".to_string(),
            current_version: Some("1.2.0".to_string()),
            target_version: Some("1.3.0".to_string()),
        }],
        skipped: vec![CliBulkSkipDto {
            agent_id: "codex-cli".to_string(),
            reason: "installation-conflict".to_string(),
        }],
        created_at: "1970-01-01T00:16:40+00:00".to_string(),
        expires_at: "1970-01-01T00:26:40+00:00".to_string(),
    };
    let value = serde_json::to_value(plan).expect("serializes");

    assert_eq!(value["items"].as_array().expect("items").len(), 1);
    // A shorter item list with no skip entries would read as "everything else is up to date".
    assert_eq!(value["skipped"][0]["agentId"], "codex-cli");
    assert_eq!(value["skipped"][0]["reason"], "installation-conflict");
}

#[test]
fn an_operation_handle_carries_only_an_id() {
    let value = serde_json::to_value(CliOperationHandleDto {
        operation_id: "op-1".to_string(),
    })
    .expect("serializes");

    assert_eq!(value["operationId"], "op-1");
    // Nothing else: the operation itself is the channel for progress, phases, and the result.
    assert_eq!(value.as_object().expect("object").len(), 1);
}
