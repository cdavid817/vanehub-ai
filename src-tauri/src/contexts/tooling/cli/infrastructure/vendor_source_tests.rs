// Included through `#[path]` from vendor_source.rs.
//
// No URL is fetched and no installer is executed. The downloader is a double that records what it
// was asked for, which is how the allowlist and the no-pipe-to-shell rules become assertions.
use std::sync::Mutex;

use super::super::environment_gateway::CliCommandOutput;
use super::*;
use crate::contexts::tooling::cli::domain::registry::{definition, SOURCE_VENDOR};
use crate::contexts::tooling::cli::domain::version::NormalizedCliVersion;

#[derive(Default)]
struct RecordingGateway {
    requests: Mutex<Vec<CliCommandRequest>>,
}

impl RecordingGateway {
    fn new() -> Arc<Self> {
        Arc::new(Self::default())
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
        Ok(CliCommandOutput {
            exit_code: Some(0),
            timed_out: false,
            cancelled: false,
            lines: Vec::new(),
            truncated: false,
        })
    }
}

#[derive(Default)]
struct RecordingDownloader {
    urls: Mutex<Vec<String>>,
}

impl RecordingDownloader {
    fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }

    fn urls(&self) -> Vec<String> {
        self.urls.lock().expect("urls").clone()
    }
}

impl CliInstallerDownloader for RecordingDownloader {
    fn download(
        &self,
        url: &str,
        trust: &CliInstallerTrust,
        _cancellation: &CliCancellation,
    ) -> Result<DownloadedInstaller, CliEnvironmentError> {
        // The real downloader applies the same check on every redirect target; asserting it here
        // keeps the contract visible at the port.
        assert!(
            trust.permits_url(url),
            "downloader received a non-allowlisted url"
        );
        self.urls.lock().expect("urls").push(url.to_string());
        // Its own directory, exactly as the production downloader does, so the handle owns cleanup.
        let directory = tempfile::tempdir().expect("fixture directory");
        let path = directory.path().join("installer.sh");
        std::fs::write(&path, b"#!/bin/sh\nexit 0\n").expect("write fixture installer");
        Ok(DownloadedInstaller {
            path,
            _directory: directory,
        })
    }
}

fn vendor_distribution(agent_id: &str) -> &'static CliDistributionDefinition {
    definition(agent_id)
        .expect("tool")
        .distribution(SOURCE_VENDOR)
        .expect("vendor distribution")
}

fn tool(agent_id: &str) -> CliToolId {
    CliToolId::new(agent_id).expect("tool id")
}

fn source() -> VendorSource {
    VendorSource::new(RecordingGateway::new(), RecordingDownloader::new())
}

fn request<'a>(agent_id: &'a CliToolId, action: CliActionKind) -> CliPlanRequest<'a> {
    CliPlanRequest {
        agent_id,
        action,
        target_version: None,
        channel: None,
        package_reference: None,
        exact_version_confirmed: false,
    }
}

#[test]
fn a_shell_only_vendor_offers_nothing_on_windows() {
    // claude-code publishes only a `.sh` installer. The old code fell through to it on Windows and
    // produced a `bash -lc` plan for a host with no POSIX shell; when that failed, a separate
    // fallback silently ran npm instead.
    let source = source();
    let agent = tool("claude-code");

    let preflight = source
        .preflight(
            vendor_distribution("claude-code"),
            &CliCancellation::never(),
        )
        .expect("preflight");
    let preview = source.build_command_preview(
        &request(&agent, CliActionKind::Install),
        vendor_distribution("claude-code"),
    );

    if cfg!(target_os = "windows") {
        assert!(!preflight.available);
        assert_eq!(
            preview.expect_err("no template on windows").category(),
            "runtime-unsupported"
        );
    } else {
        assert!(preflight.available);
        assert!(preview.is_ok());
    }
}

#[test]
fn the_one_vendor_with_a_windows_template_is_usable_there() {
    let source = source();
    let agent = tool("antigravity-cli");

    let preflight = source
        .preflight(
            vendor_distribution("antigravity-cli"),
            &CliCancellation::never(),
        )
        .expect("preflight");

    // Antigravity ships a `.ps1`, so it is the one vendor actionable on every platform.
    assert!(preflight.available);
    let preview = source
        .build_command_preview(
            &request(&agent, CliActionKind::Install),
            vendor_distribution("antigravity-cli"),
        )
        .expect("preview");

    if cfg!(target_os = "windows") {
        assert_eq!(preview.program, "powershell");
        assert!(preview.args.contains(&"-File".to_string()));
    } else {
        assert_eq!(preview.program, "bash");
    }
}

#[test]
fn powershell_is_invoked_with_file_never_with_command() {
    // `-Command "irm URL | iex"` interprets its argument as a shell string. `-File` takes a path.
    let source = source();
    let agent = tool("antigravity-cli");

    let preview = source
        .build_command_preview(
            &request(&agent, CliActionKind::Install),
            vendor_distribution("antigravity-cli"),
        )
        .expect("preview");

    if preview.program == "powershell" {
        assert!(preview.args.contains(&"-File".to_string()));
        assert!(!preview.args.contains(&"-Command".to_string()));
        assert!(!preview.args.iter().any(|arg| arg.contains("iex")));
        assert!(!preview.args.iter().any(|arg| arg.contains("irm ")));
    }
}

#[test]
fn no_preview_contains_a_pipe_to_shell_flow() {
    let source = source();
    for agent_id in ["claude-code", "opencode", "antigravity-cli"] {
        let agent = tool(agent_id);
        let Ok(preview) = source.build_command_preview(
            &request(&agent, CliActionKind::Install),
            vendor_distribution(agent_id),
        ) else {
            continue;
        };
        // The removed shape was `bash -lc "tmp=$(mktemp) && wget -qO ... | bash"`.
        assert!(preview.is_shell_free(), "{agent_id}");
        assert!(
            !preview.args.iter().any(|arg| arg.contains('|')),
            "{agent_id}"
        );
        assert!(
            !preview.args.iter().any(|arg| arg.contains("mktemp")),
            "{agent_id}"
        );
        assert!(!preview.args.contains(&"-lc".to_string()), "{agent_id}");
        assert!(!preview.args.contains(&"-c".to_string()), "{agent_id}");
    }
}

#[test]
fn a_vendor_installer_publishes_no_version_catalog() {
    let source = source();
    let catalog = source
        .list_versions(
            &tool("claude-code"),
            vendor_distribution("claude-code"),
            None,
            &CliCancellation::never(),
        )
        .expect("catalog");

    // Not-applicable rather than unavailable: there is nothing to query, and a retry would not
    // help. Crucially it is stamped with `vendor`, so npm's catalog cannot answer for it.
    assert!(!catalog.is_available());
    assert_eq!(catalog.source_id.as_str(), "vendor");
    assert_eq!(catalog.latest, None);
}

#[test]
fn an_exact_target_is_refused_because_no_convention_is_verified() {
    let source = source();
    let agent = tool("antigravity-cli");
    let target = NormalizedCliVersion::parse("1.2.3");

    let error = source
        .build_command_preview(
            &CliPlanRequest {
                target_version: Some(&target),
                ..request(&agent, CliActionKind::Install)
            },
            vendor_distribution("antigravity-cli"),
        )
        .expect_err("refused");

    // Passing it anyway would install latest and report success for 1.2.3.
    assert_eq!(error.category(), "invalid-version");
}

#[test]
fn an_installer_installs_and_refuses_everything_else() {
    let source = source();
    let agent = tool("antigravity-cli");

    for action in [
        CliActionKind::Uninstall,
        CliActionKind::Downgrade,
        CliActionKind::Reinstall,
        CliActionKind::Repair,
    ] {
        let error = source
            .build_command_preview(
                &request(&agent, action),
                vendor_distribution("antigravity-cli"),
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
fn the_preview_names_the_audited_url_not_a_temporary_path() {
    let source = source();
    let agent = tool("antigravity-cli");

    let preview = source
        .build_command_preview(
            &request(&agent, CliActionKind::Install),
            vendor_distribution("antigravity-cli"),
        )
        .expect("preview");

    // A temporary path has no business in a persisted plan or a review dialog; the URL is what the
    // user is actually being asked to approve.
    assert!(preview
        .args
        .iter()
        .any(|arg| arg.contains("antigravity.google")));
    assert!(!preview.args.iter().any(|arg| arg.contains("Temp")));
}

#[test]
fn a_downloaded_installer_is_removed_when_it_drops() {
    let directory = tempfile::tempdir().expect("directory");
    let path = directory.path().join("installer.sh");
    std::fs::write(&path, b"x").expect("write");
    assert!(path.exists());
    {
        let _installer = DownloadedInstaller {
            path: path.clone(),
            _directory: directory,
        };
    }
    // Removed on every path -- success, failure, timeout, cancellation, and panic alike. The
    // directory goes with it, so an installer that wrote a sibling file leaves nothing either.
    assert!(!path.exists());
}

#[test]
fn execution_downloads_the_allowlisted_url_and_runs_the_file() {
    if cfg!(target_os = "windows") {
        // Only the Antigravity template is actionable on Windows; the Unix branch below covers the
        // shell shape. Both go through the same code path.
    }
    let gateway = RecordingGateway::new();
    let downloader = RecordingDownloader::new();
    let source = VendorSource::new(
        Arc::clone(&gateway) as Arc<_>,
        Arc::clone(&downloader) as Arc<_>,
    );
    let agent = tool("antigravity-cli");

    let preview = source
        .build_command_preview(
            &request(&agent, CliActionKind::Install),
            vendor_distribution("antigravity-cli"),
        )
        .expect("preview");
    let spec = CliExecutionSpec {
        program: preview.program.clone(),
        args: preview.args.clone(),
        requires_network: true,
        requires_elevation: false,
    };

    struct Sink;
    impl CliOutputSink for Sink {
        fn emit(&self, _line: &str) {}
    }

    #[derive(Default)]
    struct Phases(Mutex<Vec<(String, bool)>>);
    impl CliPhaseSink for Phases {
        fn enter(&self, phase: CliOperationPhase, cancellable: bool) {
            self.0
                .lock()
                .expect("phases")
                .push((phase.as_str().to_string(), cancellable));
        }
    }
    let phases = Phases::default();

    let outcome = source
        .execute(spec, &CliCancellation::never(), &Sink, &phases)
        .expect("execution");

    assert!(outcome.succeeded());
    // The download is cancellable; the installer that follows it is not. This adapter is the only
    // thing that knows where that line falls.
    assert_eq!(
        phases.0.lock().expect("phases").as_slice(),
        [
            ("downloading".to_string(), true),
            ("mutating".to_string(), false)
        ]
    );
    // The URL actually fetched is the audited one from the registry.
    let urls = downloader.urls();
    assert_eq!(urls.len(), 1);
    assert!(urls[0].starts_with("https://antigravity.google/cli/install."));

    // The command that ran points at a real file on disk, not at a URL and not at a pipeline.
    let request = gateway.requests().into_iter().next().expect("request");
    assert!(!request.args.iter().any(|arg| arg.contains("://")));
    assert!(!request.args.iter().any(|arg| arg.contains('|')));
    if request.program == "powershell" {
        assert!(request.args.contains(&"-File".to_string()));
    }
}

#[test]
fn each_vendor_install_serializes_only_against_its_own_tool() {
    let source = source();
    let claude = tool("claude-code");
    let antigravity = tool("antigravity-cli");

    // Different trees, so two vendor installs may run concurrently.
    assert_ne!(
        source.mutation_key(&claude),
        source.mutation_key(&antigravity)
    );
    assert_eq!(source.mutation_key(&claude).as_str(), "vendor:claude-code");
    assert_eq!(source.source_id().as_str(), "vendor");
}

#[test]
fn every_registered_installer_url_is_https_on_its_own_allowlist() {
    for agent_id in ["claude-code", "opencode", "antigravity-cli"] {
        let distribution = vendor_distribution(agent_id);
        let trust = distribution.trust.installer().expect("installer trust");
        for template in trust.templates {
            assert!(template.url.starts_with("https://"), "{agent_id}");
            assert!(trust.permits_url(template.url), "{agent_id}");
        }
        assert!(trust.max_download_bytes > 0, "{agent_id}");
        assert!(trust.download_timeout_seconds > 0, "{agent_id}");
    }
}
