use super::*;
use crate::contexts::agent_runtime::application::{
    RunnerKind, RunnerPermissionContext, RunnerPermissionPort, RunnerSelection,
};
use crate::contexts::permissions::api::test_permissions_api;
use crate::contexts::permissions::domain::PolicyTemplateName;

fn adapter(template: PolicyTemplateName) -> PermissionsPortAdapter {
    PermissionsPortAdapter::new(test_permissions_api(template))
}

fn context(selection: RunnerSelection) -> RunnerPermissionContext {
    RunnerPermissionContext {
        agent_id: "codex-cli".into(),
        session_id: "session-1".into(),
        generation_id: "generation-1".into(),
        project_key: "project-1".into(),
        action: "shell.exec".into(),
        selection,
    }
}

#[test]
fn local_compatibility_is_admitted_but_does_not_authorize_ssh() {
    let permissions = adapter(PolicyTemplateName::Standard);
    assert!(permissions
        .authorize(&context(RunnerSelection::local()))
        .is_ok());
    let ssh = RunnerSelection::ssh("connection-1".into(), 3).expect("selection");
    assert_eq!(
        permissions.authorize(&context(ssh)).expect_err("ssh asks"),
        RunnerError::new(RunnerErrorKind::PermissionDenied)
    );
}

#[test]
fn witness_binds_action_runner_target_revision_and_policy() {
    let permissions = adapter(PolicyTemplateName::Trusted);
    let first = context(RunnerSelection::ssh("connection-1".into(), 3).expect("selection"));
    let changed = context(RunnerSelection::ssh("connection-1".into(), 4).expect("selection"));
    let first_witness = permissions.authorize(&first).expect("allowed");
    let changed_witness = permissions.authorize(&changed).expect("allowed");
    assert_ne!(first_witness, changed_witness);
    assert!(first_witness.fingerprint.starts_with("sha256:"));
}

#[test]
fn revalidation_fails_closed_after_policy_change() {
    let permissions = adapter(PolicyTemplateName::Trusted);
    let request = context(RunnerSelection::ssh("connection-1".into(), 3).expect("selection"));
    let witness = permissions.authorize(&request).expect("allowed");
    permissions
        .api
        .assign_template("codex-cli", PolicyTemplateName::Readonly)
        .expect("reassign");
    assert_eq!(
        permissions
            .revalidate(&request, &witness)
            .expect_err("stale"),
        RunnerError::new(RunnerErrorKind::AuthorityStale)
    );
}

#[test]
fn unknown_execution_action_fails_closed() {
    let permissions = adapter(PolicyTemplateName::Trusted);
    let mut request = context(RunnerSelection::local());
    request.action = "agent.unknown".into();
    assert_eq!(
        permissions.authorize(&request).expect_err("unknown"),
        RunnerError::new(RunnerErrorKind::PermissionDenied)
    );
    assert_eq!(RunnerKind::Local.as_str(), "local");
}
