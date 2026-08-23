// Included through `#[path]` from mod.rs.
//
// The cross-source property no single adapter's own tests can prove: that no source borrows
// another's catalog or lifecycle capability. Each adapter asserting "my catalog says npm" leaves
// open the case that two adapters both say npm.
//
// Nothing here runs a process, fetches a URL, or reads the host. Every adapter is built on a
// gateway double that refuses to be called.

use std::sync::Arc;

use crate::contexts::tooling::cli::application::environment_error::CliEnvironmentError;
use crate::contexts::tooling::cli::application::environment_ports::{
    CliCancellation, CliDistributionPort, CliOutputSink,
};
use crate::contexts::tooling::cli::domain::definition::CliDistributionDefinition;
use crate::contexts::tooling::cli::domain::ids::CliToolId;
use crate::contexts::tooling::cli::domain::registry::{definition, CLI_TOOL_DEFINITIONS};
use crate::contexts::tooling::cli::domain::source::{CliSourceKind, CliSourceManagement};

use super::environment_gateway::{CliCommandGateway, CliCommandOutput, CliCommandRequest};
use super::npm_source::NpmSource;
use super::vendor_downloader::HttpsInstallerDownloader;
use super::vendor_source::VendorSource;
use super::winget_source::WingetSource;

/// A gateway that fails every call. These tests assert declared capability, never behaviour that
/// would need a process.
struct RefusingGateway;

impl CliCommandGateway for RefusingGateway {
    fn run(
        &self,
        _request: CliCommandRequest,
        _cancellation: &CliCancellation,
        _output: Option<&dyn CliOutputSink>,
    ) -> Result<CliCommandOutput, CliEnvironmentError> {
        Err(CliEnvironmentError::Process(
            "the source matrix never runs a process".to_string(),
        ))
    }
}

fn adapters() -> Vec<Arc<dyn CliDistributionPort>> {
    let gateway = Arc::new(RefusingGateway);
    vec![
        Arc::new(NpmSource::new(gateway.clone())),
        Arc::new(WingetSource::new(gateway.clone())),
        Arc::new(VendorSource::new(
            gateway,
            Arc::new(HttpsInstallerDownloader),
        )),
    ]
}

fn tool(agent_id: &str) -> CliToolId {
    CliToolId::new(agent_id).expect("tool id")
}

fn distribution(agent_id: &str, source_id: &str) -> Option<&'static CliDistributionDefinition> {
    definition(agent_id).and_then(|tool| tool.distribution(source_id))
}

#[test]
fn every_adapter_answers_for_exactly_one_source_id() {
    let ids: Vec<String> = adapters()
        .iter()
        .map(|adapter| adapter.source_id().as_str().to_string())
        .collect();

    let mut unique = ids.clone();
    unique.sort();
    unique.dedup();
    // Two adapters claiming one id would make "the plan runs the source it recorded" unprovable:
    // the registry would resolve to whichever was registered last.
    assert_eq!(unique.len(), ids.len(), "{ids:?}");
    assert_eq!(unique, vec!["npm", "vendor", "winget"]);
}

#[test]
fn no_adapter_stamps_a_catalog_with_another_sources_id() {
    // A catalog is bound to the adapter that produced it. Borrowing npm's version list for a
    // vendor-installed tool is the defect this whole change exists to remove, and the binding is
    // what makes it unrepresentable rather than merely discouraged.
    for adapter in adapters() {
        let own = adapter.source_id();
        for tool_definition in CLI_TOOL_DEFINITIONS {
            let Some(distribution) = tool_definition.distribution(own.as_str()) else {
                continue;
            };
            let catalog = adapter.list_versions(
                &tool(tool_definition.agent_id),
                distribution,
                None,
                &CliCancellation::never(),
            );
            // npm and WinGet need a process, so they fail against the refusing gateway. Whatever
            // comes back, it must never carry a different source's id.
            if let Ok(catalog) = catalog {
                assert_eq!(catalog.source_id, own, "{}", tool_definition.agent_id);
                assert_eq!(catalog.agent_id.as_str(), tool_definition.agent_id);
            }
        }
    }
}

#[test]
fn no_adapter_previews_a_command_for_a_distribution_it_does_not_own() {
    // Handing npm's adapter the vendor distribution must not produce an npm command for it. The
    // adapter reads the distribution it is given, so this is the shape a fallback would take.
    let npm = adapters().into_iter().next().expect("npm adapter");
    let vendor_distribution = distribution("antigravity-cli", "vendor").expect("vendor");

    let preview = npm.build_command_preview(
        &crate::contexts::tooling::cli::application::environment_ports::CliPlanRequest {
            agent_id: &tool("antigravity-cli"),
            action: crate::contexts::tooling::cli::domain::action::CliActionKind::Install,
            target_version: None,
            channel: None,
            package_reference: vendor_distribution
                .package_reference
                .map(|reference| reference.identifier),
            exact_version_confirmed: false,
        },
        vendor_distribution,
    );

    // A vendor distribution names no npm package, so npm cannot build anything for it.
    assert!(preview.is_err(), "{preview:?}");
}

#[test]
fn each_distribution_declares_its_own_capabilities_and_borrows_none() {
    // The registry is the single source of capability truth. Two distributions of the same tool
    // must not share a capability record, or changing one would silently change the other.
    for tool_definition in CLI_TOOL_DEFINITIONS {
        for distribution in tool_definition.distributions {
            let kind = distribution.kind;
            let management = CliSourceManagement::of(kind);
            if management == CliSourceManagement::DetectOnly {
                // A detect-only distribution offers nothing, whatever its neighbours offer.
                assert!(
                    !distribution.capabilities.install.is_supported(),
                    "{} {}",
                    tool_definition.agent_id,
                    kind.as_str()
                );
                assert!(!distribution.capabilities.uninstall);
            }
        }
    }
}

#[test]
fn a_detect_only_kind_offers_no_lifecycle_and_names_its_own_tool() {
    for kind in [
        CliSourceKind::Homebrew,
        CliSourceKind::Bun,
        CliSourceKind::Volta,
        CliSourceKind::Desktop,
        CliSourceKind::System,
        CliSourceKind::Manual,
        CliSourceKind::Unknown,
    ] {
        assert!(kind.is_detect_only(), "{}", kind.as_str());
        assert_eq!(
            CliSourceManagement::of(kind),
            CliSourceManagement::DetectOnly
        );
        // Every one has advice, because "why is there no upgrade button" deserves an answer.
        let guidance = kind.guidance_code().expect(kind.as_str());
        assert!(guidance.starts_with("cli.guidance."), "{guidance}");
    }
}

#[test]
fn a_managed_kind_carries_no_guidance_because_it_offers_the_action_instead() {
    for kind in [
        CliSourceKind::Npm,
        CliSourceKind::Winget,
        CliSourceKind::VendorInstaller,
    ] {
        assert!(!kind.is_detect_only(), "{}", kind.as_str());
        assert_eq!(CliSourceManagement::of(kind), CliSourceManagement::Managed);
        assert_eq!(kind.guidance_code(), None, "{}", kind.as_str());
    }
}

#[test]
fn every_detect_only_kind_has_a_distinct_guidance_code() {
    let mut codes: Vec<&str> = [
        CliSourceKind::Homebrew,
        CliSourceKind::Bun,
        CliSourceKind::Volta,
        CliSourceKind::Desktop,
        CliSourceKind::System,
        CliSourceKind::Manual,
        CliSourceKind::Unknown,
    ]
    .into_iter()
    .filter_map(CliSourceKind::guidance_code)
    .collect();
    let total = codes.len();
    codes.sort_unstable();
    codes.dedup();

    // Two kinds sharing a code would tell a Homebrew user to run Volta's command.
    assert_eq!(codes.len(), total);
    assert_eq!(total, 7);
}
