// Included through `#[path]` from npm_source.rs.
//
// Every case runs against a recording gateway. No real npm is invoked, no registry is contacted,
// and the assertions are about the exact argv a real npm would have received.
use std::sync::Mutex;

use super::*;
use crate::contexts::tooling::cli::domain::registry::{definition, SOURCE_NPM};

#[derive(Default)]
struct RecordingGateway {
    requests: Mutex<Vec<CliCommandRequest>>,
    responses: Mutex<Vec<super::super::environment_gateway::CliCommandOutput>>,
}

impl RecordingGateway {
    fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }

    fn respond(&self, exit_code: Option<i32>, output: &str) {
        self.responses.lock().expect("responses").push(
            super::super::environment_gateway::CliCommandOutput {
                exit_code,
                timed_out: false,
                cancelled: false,
                lines: output.lines().map(str::to_string).collect(),
                truncated: false,
            },
        );
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
    ) -> Result<super::super::environment_gateway::CliCommandOutput, CliEnvironmentError> {
        self.requests.lock().expect("requests").push(request);
        let mut responses = self.responses.lock().expect("responses");
        if responses.is_empty() {
            return Ok(super::super::environment_gateway::CliCommandOutput {
                exit_code: Some(1),
                timed_out: false,
                cancelled: false,
                lines: vec!["no response configured".to_string()],
                truncated: false,
            });
        }
        Ok(responses.remove(0))
    }
}

fn npm_distribution() -> &'static CliDistributionDefinition {
    definition("claude-code")
        .expect("claude-code")
        .distribution(SOURCE_NPM)
        .expect("npm distribution")
}

fn tool() -> CliToolId {
    CliToolId::new("claude-code").expect("tool id")
}

fn request<'a>(
    agent_id: &'a CliToolId,
    action: CliActionKind,
    target: Option<&'a NormalizedCliVersion>,
) -> CliPlanRequest<'a> {
    CliPlanRequest {
        agent_id,
        action,
        target_version: target,
        channel: Some("stable"),
        // Deliberately wrong: the adapter must read the package from the definition, not from here.
        package_reference: Some("attacker-supplied-package"),
        exact_version_confirmed: true,
    }
}

#[test]
fn the_requested_version_reaches_the_package_spec_verbatim() {
    let source = NpmSource::new(RecordingGateway::new());
    let agent = tool();

    for (action, version) in [
        (CliActionKind::Install, "1.1.0"),
        (CliActionKind::Upgrade, "1.3.0"),
        (CliActionKind::Downgrade, "0.9.1"),
        (CliActionKind::Reinstall, "1.2.0"),
    ] {
        let parsed = NormalizedCliVersion::parse(version);
        let preview = source
            .build_command_preview(&request(&agent, action, Some(&parsed)), npm_distribution())
            .expect("preview");

        assert_eq!(
            preview.args,
            vec![
                "install".to_string(),
                "--global".to_string(),
                format!("@anthropic-ai/claude-code@{version}"),
            ],
            "{}",
            action.as_str()
        );
        // Never substituted with latest.
        assert!(!preview.args.iter().any(|arg| arg.ends_with("@latest")));
    }
}

#[test]
fn the_package_name_comes_from_the_backend_registry_not_the_request() {
    let source = NpmSource::new(RecordingGateway::new());
    let agent = tool();
    let version = NormalizedCliVersion::parse("1.3.0");

    let preview = source
        .build_command_preview(
            &request(&agent, CliActionKind::Upgrade, Some(&version)),
            npm_distribution(),
        )
        .expect("preview");

    assert!(preview
        .args
        .iter()
        .any(|arg| arg.starts_with("@anthropic-ai/claude-code@")));
    assert!(!preview.joined_contains("attacker-supplied-package"));
}

#[test]
fn no_target_resolves_to_latest_only_when_none_was_requested() {
    let source = NpmSource::new(RecordingGateway::new());
    let agent = tool();

    let preview = source
        .build_command_preview(
            &request(&agent, CliActionKind::Install, None),
            npm_distribution(),
        )
        .expect("preview");

    assert_eq!(preview.args[2], "@anthropic-ai/claude-code@latest");
}

#[test]
fn uninstall_carries_the_package_without_a_version() {
    let source = NpmSource::new(RecordingGateway::new());
    let agent = tool();

    let preview = source
        .build_command_preview(
            &request(&agent, CliActionKind::Uninstall, None),
            npm_distribution(),
        )
        .expect("preview");

    assert_eq!(
        preview.args,
        vec![
            "uninstall".to_string(),
            "--global".to_string(),
            "@anthropic-ai/claude-code".to_string()
        ]
    );
}

#[test]
fn npm_refuses_repair_rather_than_improvising_one() {
    let source = NpmSource::new(RecordingGateway::new());
    let agent = tool();

    let error = source
        .build_command_preview(
            &request(&agent, CliActionKind::Repair, None),
            npm_distribution(),
        )
        .expect_err("repair is unsupported");

    assert_eq!(error.category(), "unsupported-action");
}

#[test]
fn every_preview_is_explicit_argv_with_nothing_a_shell_would_interpret() {
    let source = NpmSource::new(RecordingGateway::new());
    let agent = tool();
    let version = NormalizedCliVersion::parse("1.3.0");

    let preview = source
        .build_command_preview(
            &request(&agent, CliActionKind::Upgrade, Some(&version)),
            npm_distribution(),
        )
        .expect("preview");

    assert!(preview.is_shell_free());
    assert!(!preview.args.iter().any(|arg| arg.contains(' ')));
}

#[test]
fn execution_is_derived_from_the_reviewed_preview_not_rebuilt() {
    let source = NpmSource::new(RecordingGateway::new());
    let preview = CliCommandPreview::new(
        "npm",
        vec![
            "install".to_string(),
            "--global".to_string(),
            "@anthropic-ai/claude-code@1.1.0".to_string(),
        ],
    );
    let plan = plan_with(preview.clone());

    let spec = source
        .build_execution(&plan, npm_distribution())
        .expect("spec");

    // Byte-for-byte the reviewed command; there is no second construction to disagree with it.
    assert_eq!(spec.program, preview.program);
    assert_eq!(spec.args, preview.args);
    assert!(spec.requires_network);
    assert!(!spec.requires_elevation);
}

#[test]
fn the_catalog_query_asks_npm_for_that_package_only() {
    let gateway = RecordingGateway::new();
    gateway.respond(Some(0), r#"["1.1.0","1.2.0","1.3.0"]"#);
    let source = NpmSource::new(Arc::clone(&gateway) as Arc<_>);

    let catalog = source
        .list_versions(
            &tool(),
            npm_distribution(),
            Some("stable"),
            &CliCancellation::never(),
        )
        .expect("catalog");

    let request = gateway.requests().into_iter().next().expect("request");
    assert_eq!(
        request.args,
        vec![
            "view".to_string(),
            "@anthropic-ai/claude-code".to_string(),
            "versions".to_string(),
            "--json".to_string()
        ]
    );
    // The catalog is stamped with this source, so it cannot answer for another one.
    assert_eq!(catalog.source_id.as_str(), "npm");
    assert_eq!(catalog.agent_id.as_str(), "claude-code");
    assert!(catalog.is_available());
}

#[test]
fn versions_are_ordered_newest_first_and_latest_prefers_a_stable_release() {
    let gateway = RecordingGateway::new();
    gateway.respond(Some(0), r#"["1.9.0","1.10.0","2.0.0-rc.1","1.2.0"]"#);
    let source = NpmSource::new(Arc::clone(&gateway) as Arc<_>);

    let catalog = source
        .list_versions(&tool(), npm_distribution(), None, &CliCancellation::never())
        .expect("catalog");

    assert_eq!(
        catalog
            .versions
            .iter()
            .map(NormalizedCliVersion::as_str)
            .collect::<Vec<_>>(),
        vec!["2.0.0-rc.1", "1.10.0", "1.9.0", "1.2.0"]
    );
    // A prerelease is newest but is not what an unspecified target resolves to.
    assert_eq!(
        catalog.latest.as_ref().map(NormalizedCliVersion::as_str),
        Some("1.10.0")
    );
}

#[test]
fn a_single_published_version_arrives_as_a_bare_string() {
    let gateway = RecordingGateway::new();
    gateway.respond(Some(0), r#""1.0.0""#);
    let source = NpmSource::new(Arc::clone(&gateway) as Arc<_>);

    let catalog = source
        .list_versions(&tool(), npm_distribution(), None, &CliCancellation::never())
        .expect("catalog");

    assert_eq!(catalog.versions.len(), 1);
    assert_eq!(
        catalog.latest.as_ref().map(NormalizedCliVersion::as_str),
        Some("1.0.0")
    );
}

#[test]
fn unparseable_registry_output_yields_an_unavailable_catalog_not_a_guess() {
    for raw in ["not json at all", "{}", "[]"] {
        let gateway = RecordingGateway::new();
        gateway.respond(Some(0), raw);
        let source = NpmSource::new(Arc::clone(&gateway) as Arc<_>);

        let catalog = source
            .list_versions(&tool(), npm_distribution(), None, &CliCancellation::never())
            .expect("catalog");

        assert!(!catalog.is_available(), "{raw}");
        assert_eq!(catalog.latest, None, "{raw}");
        // Still stamped with its source, so nothing else can be substituted for it.
        assert_eq!(catalog.source_id.as_str(), "npm");
    }
}

#[test]
fn a_failing_registry_query_yields_an_unavailable_catalog() {
    let gateway = RecordingGateway::new();
    gateway.respond(Some(1), "npm error code E404");
    let source = NpmSource::new(Arc::clone(&gateway) as Arc<_>);

    let catalog = source
        .list_versions(&tool(), npm_distribution(), None, &CliCancellation::never())
        .expect("catalog");

    assert!(!catalog.is_available());
}

#[test]
fn preflight_reports_availability_from_npms_own_version_command() {
    let gateway = RecordingGateway::new();
    gateway.respond(Some(0), "10.8.2");
    let source = NpmSource::new(Arc::clone(&gateway) as Arc<_>);

    let preflight = source
        .preflight(npm_distribution(), &CliCancellation::never())
        .expect("preflight");

    assert!(preflight.available);
    assert_eq!(preflight.source_version.as_deref(), Some("10.8.2"));
    // npm pins by construction, so exact-version support needs no dynamic confirmation.
    assert!(preflight.supports_exact_version);
    assert!(!preflight.supports_repair);
    assert!(!preflight.requires_elevation);
}

#[test]
fn a_missing_npm_reports_unavailable_rather_than_erroring() {
    let gateway = RecordingGateway::new();
    gateway.respond(Some(1), "");
    let source = NpmSource::new(Arc::clone(&gateway) as Arc<_>);

    let preflight = source
        .preflight(npm_distribution(), &CliCancellation::never())
        .expect("preflight");

    assert!(!preflight.available);
    assert_eq!(preflight.source_version, None);
}

#[test]
fn two_tools_installed_through_npm_share_one_mutation_key() {
    let source = NpmSource::new(RecordingGateway::new());
    let claude = CliToolId::new("claude-code").expect("id");
    let codex = CliToolId::new("codex-cli").expect("id");

    // The global prefix is one resource; the tools are irrelevant to the contention.
    assert_eq!(source.mutation_key(&claude), source.mutation_key(&codex));
    assert_eq!(source.mutation_key(&claude).as_str(), "npm-global");
    assert_eq!(source.source_id().as_str(), "npm");
}

// Small helpers kept at the bottom so the assertions above read first.

impl CliCommandPreview {
    fn joined_contains(&self, needle: &str) -> bool {
        self.args.iter().any(|arg| arg.contains(needle)) || self.program.contains(needle)
    }
}

fn plan_with(preview: CliCommandPreview) -> CliActionPlan {
    let created_at = chrono::DateTime::from_timestamp(1_000, 0).expect("timestamp");
    CliActionPlan {
        id: crate::contexts::tooling::cli::domain::ids::CliActionPlanId::new("plan-1")
            .expect("plan id"),
        revision: 1,
        agent_id: tool(),
        action: CliActionKind::Upgrade,
        source_id: CliSourceId::new("npm").expect("source id"),
        installation_id: None,
        current_version: Some("1.2.0".to_string()),
        target_version: Some("1.1.0".to_string()),
        channel: Some("stable".to_string()),
        command_preview: preview,
        preconditions: Vec::new(),
        warnings: Vec::new(),
        requires_elevation: false,
        requires_network: true,
        fallback_policy: crate::contexts::tooling::cli::domain::plan::CliFallbackPolicy::None,
        environment_fingerprint: "fingerprint-a".to_string(),
        state: crate::contexts::tooling::cli::domain::plan::CliActionPlanState::Draft,
        created_at,
        expires_at: CliActionPlan::default_expiry(created_at),
    }
}
