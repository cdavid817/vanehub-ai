// Included through `#[path]` from winget_source.rs.
//
// Fixture-driven: no real WinGet is invoked. The assertions are about the exact argv a real WinGet
// would have received, which is the only way to catch a dropped `--version`.
use std::sync::Mutex;

use super::super::environment_gateway::CliCommandOutput;
use super::*;
use crate::contexts::tooling::cli::domain::registry::{definition, SOURCE_WINGET};

#[derive(Default)]
struct RecordingGateway {
    requests: Mutex<Vec<CliCommandRequest>>,
    responses: Mutex<Vec<CliCommandOutput>>,
}

impl RecordingGateway {
    fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }

    fn respond(&self, exit_code: Option<i32>, output: &str) {
        self.responses
            .lock()
            .expect("responses")
            .push(CliCommandOutput {
                exit_code,
                timed_out: false,
                cancelled: false,
                lines: output.lines().map(str::to_string).collect(),
                truncated: false,
            });
    }

    fn requests(&self) -> Vec<CliCommandRequest> {
        self.requests.lock().expect("requests").clone()
    }
}

impl CliCommandGateway for RecordingGateway {
    fn run(
        &self,
        request: CliCommandRequest,
        _cancellation: &CliCancellation,
        _output: Option<&dyn CliOutputSink>,
    ) -> Result<CliCommandOutput, CliEnvironmentError> {
        self.requests.lock().expect("requests").push(request);
        let mut responses = self.responses.lock().expect("responses");
        if responses.is_empty() {
            return Ok(CliCommandOutput {
                exit_code: Some(1),
                timed_out: false,
                cancelled: false,
                lines: Vec::new(),
                truncated: false,
            });
        }
        Ok(responses.remove(0))
    }
}

fn winget_distribution() -> &'static CliDistributionDefinition {
    definition("claude-code")
        .expect("claude-code")
        .distribution(SOURCE_WINGET)
        .expect("winget distribution")
}

fn tool() -> CliToolId {
    CliToolId::new("claude-code").expect("tool id")
}

fn request<'a>(
    agent_id: &'a CliToolId,
    action: CliActionKind,
    target: Option<&'a NormalizedCliVersion>,
    exact_confirmed: bool,
) -> CliPlanRequest<'a> {
    CliPlanRequest {
        agent_id,
        action,
        target_version: target,
        channel: None,
        package_reference: Some("attacker-supplied"),
        exact_version_confirmed: exact_confirmed,
    }
}

#[test]
fn a_confirmed_exact_target_reaches_the_version_argument() {
    // The regression: the old plan was `winget upgrade --id <id> --exact` with no `--version` at
    // all, so a requested version was silently discarded and latest was installed.
    let source = WingetSource::new(RecordingGateway::new());
    let agent = tool();
    let target = NormalizedCliVersion::parse("1.3.0");

    for action in [CliActionKind::Install, CliActionKind::Upgrade] {
        let preview = source
            .build_command_preview(
                &request(&agent, action, Some(&target), true),
                winget_distribution(),
            )
            .expect("preview");

        let position = preview
            .args
            .iter()
            .position(|arg| arg == "--version")
            .unwrap_or_else(|| panic!("{} must carry --version", action.as_str()));
        assert_eq!(preview.args[position + 1], "1.3.0");
        assert!(preview.args.contains(&"--exact".to_string()));
        assert!(preview.args.contains(&"Anthropic.ClaudeCode".to_string()));
    }
}

#[test]
fn an_unconfirmed_exact_target_is_refused_rather_than_run_as_latest() {
    let source = WingetSource::new(RecordingGateway::new());
    let agent = tool();
    let target = NormalizedCliVersion::parse("1.3.0");

    let error = source
        .build_command_preview(
            &request(&agent, CliActionKind::Upgrade, Some(&target), false),
            winget_distribution(),
        )
        .expect_err("refused");

    // Running without `--version` here would install latest and report it as 1.3.0.
    assert_eq!(error.category(), "invalid-version");
}

#[test]
fn an_upgrade_with_no_target_carries_no_version_argument() {
    let source = WingetSource::new(RecordingGateway::new());
    let agent = tool();

    let preview = source
        .build_command_preview(
            &request(&agent, CliActionKind::Upgrade, None, true),
            winget_distribution(),
        )
        .expect("preview");

    assert!(!preview.args.contains(&"--version".to_string()));
}

#[test]
fn downgrade_and_reinstall_are_refused_because_no_argument_form_is_verified() {
    let source = WingetSource::new(RecordingGateway::new());
    let agent = tool();
    let target = NormalizedCliVersion::parse("1.0.0");

    for action in [CliActionKind::Downgrade, CliActionKind::Reinstall] {
        let error = source
            .build_command_preview(
                &request(&agent, action, Some(&target), true),
                winget_distribution(),
            )
            .expect_err("refused");
        assert_eq!(
            error.category(),
            "unsupported-action",
            "{}",
            action.as_str()
        );
    }
}

#[test]
fn the_package_id_comes_from_the_registry_not_the_request() {
    let source = WingetSource::new(RecordingGateway::new());
    let agent = tool();

    let preview = source
        .build_command_preview(
            &request(&agent, CliActionKind::Uninstall, None, true),
            winget_distribution(),
        )
        .expect("preview");

    assert!(preview.args.contains(&"Anthropic.ClaudeCode".to_string()));
    assert!(!preview.args.iter().any(|arg| arg == "attacker-supplied"));
}

#[test]
fn mutating_commands_are_non_interactive() {
    let source = WingetSource::new(RecordingGateway::new());
    let agent = tool();

    // Without these WinGet blocks on a prompt no desktop app can answer, and the operation hangs
    // to its timeout instead of failing usefully.
    let upgrade = source
        .build_command_preview(
            &request(&agent, CliActionKind::Upgrade, None, true),
            winget_distribution(),
        )
        .expect("preview");
    for flag in [
        "--accept-package-agreements",
        "--accept-source-agreements",
        "--disable-interactivity",
    ] {
        assert!(upgrade.args.contains(&flag.to_string()), "{flag}");
    }

    let uninstall = source
        .build_command_preview(
            &request(&agent, CliActionKind::Uninstall, None, true),
            winget_distribution(),
        )
        .expect("preview");
    assert!(uninstall
        .args
        .contains(&"--disable-interactivity".to_string()));
    // Agreement flags are meaningless for uninstall and are not sent.
    assert!(!uninstall
        .args
        .contains(&"--accept-package-agreements".to_string()));
}

#[test]
fn every_preview_is_explicit_argv() {
    let source = WingetSource::new(RecordingGateway::new());
    let agent = tool();
    let target = NormalizedCliVersion::parse("1.3.0");

    let preview = source
        .build_command_preview(
            &request(&agent, CliActionKind::Install, Some(&target), true),
            winget_distribution(),
        )
        .expect("preview");

    assert!(preview.is_shell_free());
    assert_eq!(preview.program, "winget");
}

#[test]
fn capability_detection_reads_the_reported_winget_version() {
    // `--version` on install/upgrade arrived in 1.2; repair in 1.7.
    assert!(!supports_exact_version("v1.1.0"));
    assert!(supports_exact_version("v1.2.0"));
    assert!(supports_exact_version("v1.6.3482"));
    assert!(!supports_repair("v1.6.3482"));
    assert!(supports_repair("v1.7.10582"));

    // An unrecognisable version withholds both rather than assuming either. Assuming produces a
    // command the local WinGet rejects.
    assert!(!supports_exact_version("unknown build"));
    assert!(!supports_repair(""));
}

#[test]
fn preflight_reports_capabilities_from_the_local_winget() {
    let gateway = RecordingGateway::new();
    gateway.respond(Some(0), "v1.6.3482");
    let source = WingetSource::new(Arc::clone(&gateway) as Arc<_>);

    let preflight = source
        .preflight(winget_distribution(), &CliCancellation::never())
        .expect("preflight");

    assert!(preflight.available);
    assert!(preflight.supports_exact_version);
    // 1.6 predates repair, so the dynamic capability stays closed.
    assert!(!preflight.supports_repair);
    assert_eq!(preflight.source_version.as_deref(), Some("v1.6.3482"));
}

#[test]
fn a_missing_winget_reports_unavailable_with_no_capabilities() {
    let gateway = RecordingGateway::new();
    gateway.respond(Some(1), "");
    let source = WingetSource::new(Arc::clone(&gateway) as Arc<_>);

    let preflight = source
        .preflight(winget_distribution(), &CliCancellation::never())
        .expect("preflight");

    assert!(!preflight.available);
    assert!(!preflight.supports_exact_version);
    assert!(!preflight.supports_repair);
}

#[test]
fn the_catalog_query_asks_winget_for_its_own_versions() {
    let gateway = RecordingGateway::new();
    gateway.respond(Some(0), "Version\n-------\n1.3.0\n1.2.0\n1.1.0\n");
    let source = WingetSource::new(Arc::clone(&gateway) as Arc<_>);

    let catalog = source
        .list_versions(
            &tool(),
            winget_distribution(),
            None,
            &CliCancellation::never(),
        )
        .expect("catalog");

    let request = gateway.requests().into_iter().next().expect("request");
    assert!(request.args.contains(&"Anthropic.ClaudeCode".to_string()));
    assert!(request.args.contains(&"--versions".to_string()));
    // Stamped with WinGet, so an npm catalog can never stand in for it.
    assert_eq!(catalog.source_id.as_str(), "winget");
    assert_eq!(
        catalog.latest.as_ref().map(NormalizedCliVersion::as_str),
        Some("1.3.0")
    );
    assert_eq!(catalog.versions.len(), 3);
}

#[test]
fn localized_output_that_yields_no_versions_reports_unavailable() {
    // Header wording is localized, so the parser keys off shape. Output with no version-shaped
    // line yields nothing -- and nothing is the correct answer, not npm's list.
    let gateway = RecordingGateway::new();
    gateway.respond(Some(0), "找不到与输入条件匹配的程序包。\n");
    let source = WingetSource::new(Arc::clone(&gateway) as Arc<_>);

    let catalog = source
        .list_versions(
            &tool(),
            winget_distribution(),
            None,
            &CliCancellation::never(),
        )
        .expect("catalog");

    assert!(!catalog.is_available());
    assert_eq!(catalog.latest, None);
}

#[test]
fn a_localized_header_does_not_stop_versions_from_being_read() {
    let gateway = RecordingGateway::new();
    gateway.respond(Some(0), "版本\n----\n2.0.0\n1.9.0\n");
    let source = WingetSource::new(Arc::clone(&gateway) as Arc<_>);

    let catalog = source
        .list_versions(
            &tool(),
            winget_distribution(),
            None,
            &CliCancellation::never(),
        )
        .expect("catalog");

    assert!(catalog.is_available());
    assert_eq!(
        catalog.latest.as_ref().map(NormalizedCliVersion::as_str),
        Some("2.0.0")
    );
}

#[test]
fn a_failing_catalog_query_reports_unavailable() {
    let gateway = RecordingGateway::new();
    gateway.respond(Some(1), "no package found");
    let source = WingetSource::new(Arc::clone(&gateway) as Arc<_>);

    let catalog = source
        .list_versions(
            &tool(),
            winget_distribution(),
            None,
            &CliCancellation::never(),
        )
        .expect("catalog");

    assert!(!catalog.is_available());
}

#[test]
fn winget_serializes_against_its_own_resource() {
    let source = WingetSource::new(RecordingGateway::new());
    let claude = CliToolId::new("claude-code").expect("id");
    let codex = CliToolId::new("codex-cli").expect("id");

    assert_eq!(source.mutation_key(&claude).as_str(), "winget");
    assert_eq!(source.mutation_key(&claude), source.mutation_key(&codex));
    assert_eq!(source.source_id().as_str(), "winget");
}
