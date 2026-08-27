// Declared directly under `application`, so `super` is that module.
//
// Readiness derivation (task group 6). The backend derives readiness from the executable,
// authentication, compatibility, and Doctor results together; nothing here is assembled by the
// frontend from separate fields. Every probe answer is a fixture -- no provider is contacted and
// no credential store is read.
use super::environment_service_fixtures::{healthy_npm_installation, Harness};
use crate::contexts::tooling::cli::domain::status::{
    CliAuthenticationStatus, CliExecutableStatus, CliOverallState, CliReadinessStatus,
};

fn refresh_one(harness: &Harness, agent_id: &str) {
    let prepared = harness
        .service
        .prepare_refresh(vec![agent_id.to_string()], false)
        .expect("prepare");
    harness.service.execute_refresh(prepared).expect("execute");
}

fn install(harness: &Harness, agent_id: &str, path: &str, version: &str) {
    harness
        .discovery
        .set(agent_id, vec![healthy_npm_installation("a", path)]);
    harness.probes.set_version(path, version);
}

#[test]
fn a_tool_that_reports_not_logged_in_becomes_needs_auth() {
    let harness = Harness::new();
    install(&harness, "codex-cli", "/path/codex", "1.0.0");
    harness.probes.set_command_output(
        "/path/codex",
        &["login", "status"],
        Some(1),
        "Not logged in.",
    );

    refresh_one(&harness, "codex-cli");

    let snapshot = harness.repository.snapshot("codex-cli").expect("snapshot");
    assert_eq!(snapshot.authentication, CliAuthenticationStatus::Required);
    assert_eq!(snapshot.readiness, CliReadinessStatus::NeedsAuth);
    assert_eq!(snapshot.overall_state, CliOverallState::NeedsAuth);
}

#[test]
fn an_expired_session_is_reported_as_expired_not_merely_required() {
    let harness = Harness::new();
    install(&harness, "codex-cli", "/path/codex", "1.0.0");
    harness.probes.set_command_output(
        "/path/codex",
        &["login", "status"],
        Some(1),
        "Session expired. Not logged in.",
    );

    refresh_one(&harness, "codex-cli");

    let snapshot = harness.repository.snapshot("codex-cli").expect("snapshot");
    assert_eq!(snapshot.authentication, CliAuthenticationStatus::Expired);
    assert_eq!(snapshot.readiness, CliReadinessStatus::NeedsAuth);
}

#[test]
fn a_tool_that_reports_a_session_becomes_ready() {
    let harness = Harness::new();
    install(&harness, "codex-cli", "/path/codex", "1.0.0");
    harness.probes.set_command_output(
        "/path/codex",
        &["login", "status"],
        Some(0),
        "Logged in as dev@example.test",
    );

    refresh_one(&harness, "codex-cli");

    let snapshot = harness.repository.snapshot("codex-cli").expect("snapshot");
    assert_eq!(
        snapshot.authentication,
        CliAuthenticationStatus::Authenticated
    );
    assert_eq!(snapshot.readiness, CliReadinessStatus::Ready);
}

#[test]
fn opencode_credentials_are_reduced_to_a_state_without_the_account_list() {
    let harness = Harness::new();
    install(&harness, "opencode", "/path/opencode", "1.0.0");
    harness.probes.set_command_output(
        "/path/opencode",
        &["auth", "list"],
        Some(0),
        "Provider   Account\n---------  -------\nanthropic  dev@example.test\n",
    );

    refresh_one(&harness, "opencode");

    let snapshot = harness.repository.snapshot("opencode").expect("snapshot");
    assert_eq!(
        snapshot.authentication,
        CliAuthenticationStatus::Authenticated
    );
    // The account never reaches the snapshot. The only thing persisted is the enum.
    let serialized = format!("{snapshot:?}");
    assert!(!serialized.contains("dev@example.test"));
    assert!(!serialized.contains("anthropic  "));
}

#[test]
fn a_tool_with_no_documented_auth_probe_stays_unknown_and_is_not_blocked() {
    // Gemini CLI declares no authentication probe. Unknown must not become NeedsAuth, or the page
    // would tell the user to log in to something VaneHub cannot check.
    let harness = Harness::new();
    install(&harness, "gemini-cli", "/path/gemini", "1.0.0");

    refresh_one(&harness, "gemini-cli");

    let snapshot = harness.repository.snapshot("gemini-cli").expect("snapshot");
    assert_eq!(snapshot.authentication, CliAuthenticationStatus::Unknown);
    assert_ne!(snapshot.readiness, CliReadinessStatus::NeedsAuth);
    // Only `--version` ran: no probe was invented for a tool that declares none.
    let extra_probes = harness
        .probes
        .invocations()
        .into_iter()
        .filter(|(_, args)| args != &vec!["--version".to_string()])
        .count();
    assert_eq!(extra_probes, 0);
}

#[test]
fn a_failing_doctor_makes_the_tool_misconfigured_not_broken() {
    let harness = Harness::new();
    install(&harness, "claude-code", "/path/claude", "1.2.0");
    harness
        .probes
        .set_command_output("/path/claude", &["doctor"], Some(1), "1 check failed");

    refresh_one(&harness, "claude-code");

    let snapshot = harness
        .repository
        .snapshot("claude-code")
        .expect("snapshot");
    // The binary runs; something about its configuration does not. Different findings.
    assert_eq!(snapshot.executable, CliExecutableStatus::Healthy);
    assert_eq!(snapshot.readiness, CliReadinessStatus::Misconfigured);
}

#[test]
fn an_older_build_without_the_doctor_subcommand_is_not_misconfigured() {
    let harness = Harness::new();
    install(&harness, "claude-code", "/path/claude", "1.2.0");
    harness.probes.set_command_output(
        "/path/claude",
        &["doctor"],
        Some(2),
        "error: unrecognized subcommand 'doctor'",
    );

    refresh_one(&harness, "claude-code");

    let snapshot = harness
        .repository
        .snapshot("claude-code")
        .expect("snapshot");
    assert_ne!(snapshot.readiness, CliReadinessStatus::Misconfigured);
}

#[test]
fn readiness_probes_are_skipped_when_nothing_runnable_was_found() {
    // Probing a binary that does not run tells us nothing and costs a process launch.
    let harness = Harness::new();
    harness.discovery.set(
        "codex-cli",
        vec![healthy_npm_installation("a", "/path/codex")],
    );
    harness.probes.set_failure("/path/codex", false);

    refresh_one(&harness, "codex-cli");

    let auth_probes = harness
        .probes
        .invocations()
        .into_iter()
        .filter(|(_, args)| args == &vec!["login".to_string(), "status".to_string()])
        .count();
    assert_eq!(auth_probes, 0);

    let snapshot = harness.repository.snapshot("codex-cli").expect("snapshot");
    assert_eq!(snapshot.authentication, CliAuthenticationStatus::Unknown);
    assert_eq!(snapshot.readiness, CliReadinessStatus::Broken);
}

#[test]
fn a_timed_out_auth_probe_leaves_the_state_unknown() {
    let harness = Harness::new();
    install(&harness, "codex-cli", "/path/codex", "1.0.0");
    // No configured response for the auth command and a non-zero fallback: the parser must not
    // read that as a logged-out session.
    harness
        .probes
        .set_command_output("/path/codex", &["login", "status"], None, "");

    refresh_one(&harness, "codex-cli");

    let snapshot = harness.repository.snapshot("codex-cli").expect("snapshot");
    assert_eq!(snapshot.authentication, CliAuthenticationStatus::Unknown);
}
