use super::*;
use std::collections::BTreeMap;

struct Probe(BTreeMap<DelegationTarget, Result<DelegationProbeObservation, ()>>);

impl DelegationProbePort for Probe {
    fn probe(&self, target: DelegationTarget) -> Result<DelegationProbeObservation, ()> {
        self.0.get(&target).cloned().unwrap_or(Err(()))
    }
}

fn observation(target: DelegationTarget, version: &str) -> DelegationProbeObservation {
    let help = match target {
        DelegationTarget::ClaudeCode => {
            "--print --output-format --json-schema --permission-mode --tools --strict-mcp-config --max-turns"
        }
        DelegationTarget::CodexCli => "exec --json --output-schema --sandbox --ephemeral",
    };
    DelegationProbeObservation {
        executable: PathBuf::from(format!("C:/tools/{}.exe", target.as_str())),
        executable_sha256: "a".repeat(64),
        version: version.to_owned(),
        help: help.to_owned(),
        authentication: DelegationAuthentication::Available,
    }
}

fn dependencies() -> DelegationCapabilityDependencies {
    DelegationCapabilityDependencies {
        process_tree_control: true,
        analyze_isolation: true,
        edit_isolation: true,
        artifact_storage: true,
        codex_network_isolation_canary: true,
    }
}

fn service(
    claude: Result<DelegationProbeObservation, ()>,
    codex: Result<DelegationProbeObservation, ()>,
    dependencies: DelegationCapabilityDependencies,
) -> DelegationReadinessService {
    DelegationReadinessService::new(
        Arc::new(Probe(BTreeMap::from([
            (DelegationTarget::ClaudeCode, claude),
            (DelegationTarget::CodexCli, codex),
        ]))),
        dependencies,
    )
}

#[test]
fn aggregate_readiness_is_separate_for_each_target_and_mode() {
    let readiness = service(
        Ok(observation(DelegationTarget::ClaudeCode, "claude 2.1.0")),
        Err(()),
        dependencies(),
    )
    .check();
    assert_eq!(readiness.len(), 4);
    assert!(readiness
        .iter()
        .filter(|item| item.target == DelegationTarget::ClaudeCode)
        .all(|item| item.state == DelegationReadinessState::Ready
            && item.executable_sha256.as_deref() == Some(&"a".repeat(64))));
    assert!(readiness
        .iter()
        .filter(|item| item.target == DelegationTarget::CodexCli)
        .all(|item| item.reason == DelegationReadinessReason::ProbeFailed));
}

#[test]
fn newer_unreviewed_version_degrades_analyze_and_blocks_edit() {
    let readiness = service(
        Ok(observation(DelegationTarget::ClaudeCode, "claude 3.0.0")),
        Ok(observation(DelegationTarget::CodexCli, "codex 1.0.0")),
        dependencies(),
    )
    .check();
    for target in [DelegationTarget::ClaudeCode, DelegationTarget::CodexCli] {
        let analyze = readiness
            .iter()
            .find(|item| item.target == target && item.mode == DelegationMode::Analyze)
            .expect("analyze");
        let edit = readiness
            .iter()
            .find(|item| item.target == target && item.mode == DelegationMode::Edit)
            .expect("edit");
        assert_eq!(analyze.state, DelegationReadinessState::Degraded);
        assert_eq!(edit.state, DelegationReadinessState::Blocked);
        assert_eq!(edit.reason, DelegationReadinessReason::VersionAboveReviewed);
    }
}

#[test]
fn analyze_can_remain_ready_when_edit_dependencies_are_unavailable() {
    let mut capabilities = dependencies();
    capabilities.edit_isolation = false;
    capabilities.artifact_storage = false;
    let readiness = service(
        Ok(observation(DelegationTarget::ClaudeCode, "claude 2.1.0")),
        Ok(observation(DelegationTarget::CodexCli, "codex 0.50.0")),
        capabilities,
    )
    .check();
    assert!(readiness
        .iter()
        .filter(|item| item.mode == DelegationMode::Analyze)
        .all(|item| item.state == DelegationReadinessState::Ready));
    assert!(readiness
        .iter()
        .filter(|item| item.mode == DelegationMode::Edit)
        .all(|item| item.reason == DelegationReadinessReason::EditIsolationUnavailable));
}

#[test]
fn passive_probe_failures_have_stable_fail_closed_reasons() {
    let mut missing_flag = observation(DelegationTarget::ClaudeCode, "claude 2.1.0");
    missing_flag.help = "--print".to_owned();
    missing_flag.authentication = DelegationAuthentication::Unknown;
    let mut unauthenticated = observation(DelegationTarget::CodexCli, "codex 0.50.0");
    unauthenticated.authentication = DelegationAuthentication::Unavailable;
    let readiness = service(Ok(missing_flag), Ok(unauthenticated), dependencies()).check();
    assert!(readiness
        .iter()
        .filter(|item| item.target == DelegationTarget::ClaudeCode)
        .all(|item| item.reason == DelegationReadinessReason::RequiredFlagsMissing));
    assert!(readiness
        .iter()
        .filter(|item| item.target == DelegationTarget::CodexCli)
        .all(|item| item.reason == DelegationReadinessReason::AuthenticationUnavailable));
    assert_eq!(
        DelegationReadinessReason::RequiredFlagsMissing.as_str(),
        "required_flags_missing"
    );
}

#[cfg(windows)]
#[test]
fn windows_codex_modes_remain_blocked_until_network_isolation_canary_passes() {
    let mut capabilities = dependencies();
    capabilities.codex_network_isolation_canary = false;
    let readiness = service(
        Err(()),
        Ok(observation(DelegationTarget::CodexCli, "codex 0.50.0")),
        capabilities,
    )
    .check();
    let codex = readiness
        .iter()
        .filter(|item| item.target == DelegationTarget::CodexCli)
        .collect::<Vec<_>>();
    assert_eq!(codex.len(), 2);
    assert!(codex.iter().all(|item| {
        item.state == DelegationReadinessState::Blocked
            && item.reason == DelegationReadinessReason::ProviderChildNetworkIsolationUnavailable
    }));
}
