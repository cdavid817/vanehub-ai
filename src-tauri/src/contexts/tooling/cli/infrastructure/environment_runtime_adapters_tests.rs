// Included through `#[path]` from environment_runtime_adapters.rs.
//
// Nothing here starts a process, opens a database, or touches the operations store. These assert
// the policy each adapter owns: id shape, reservation rules, and registry keying.
use super::*;

use crate::contexts::tooling::cli::application::environment_ports::{
    CliExecutionSpec, CliOutputSink, CliPhaseSink, CliPlanRequest, CliProcessOutcome,
    CliSourcePreflight,
};
use crate::contexts::tooling::cli::domain::catalog::CliVersionCatalog;
use crate::contexts::tooling::cli::domain::definition::CliDistributionDefinition;
use crate::contexts::tooling::cli::domain::plan::{CliActionPlan, CliCommandPreview};

fn tool(value: &str) -> CliToolId {
    CliToolId::new(value).expect("tool id")
}

fn source(value: &str) -> CliSourceId {
    CliSourceId::new(value).expect("source id")
}

/// An adapter that reports one source id and refuses to do anything else.
struct StubSource(CliSourceId);

impl CliDistributionPort for StubSource {
    fn source_id(&self) -> CliSourceId {
        self.0.clone()
    }

    fn mutation_key(&self, agent_id: &CliToolId) -> CliMutationKey {
        CliMutationKey::vendor(agent_id.as_str())
    }

    fn preflight(
        &self,
        _definition: &CliDistributionDefinition,
        _cancellation: &CliCancellation,
    ) -> Result<CliSourcePreflight, CliEnvironmentError> {
        unreachable!("registry tests never run a source")
    }

    fn list_versions(
        &self,
        _agent_id: &CliToolId,
        _definition: &CliDistributionDefinition,
        _channel: Option<&str>,
        _cancellation: &CliCancellation,
    ) -> Result<CliVersionCatalog, CliEnvironmentError> {
        unreachable!("registry tests never run a source")
    }

    fn build_command_preview(
        &self,
        _request: &CliPlanRequest<'_>,
        _definition: &CliDistributionDefinition,
    ) -> Result<CliCommandPreview, CliEnvironmentError> {
        unreachable!("registry tests never run a source")
    }

    fn build_execution(
        &self,
        _plan: &CliActionPlan,
        _definition: &CliDistributionDefinition,
    ) -> Result<CliExecutionSpec, CliEnvironmentError> {
        unreachable!("registry tests never run a source")
    }

    fn execute(
        &self,
        _spec: CliExecutionSpec,
        _cancellation: &CliCancellation,
        _output: &dyn CliOutputSink,
        _phases: &dyn CliPhaseSink,
    ) -> Result<CliProcessOutcome, CliEnvironmentError> {
        unreachable!("registry tests never run a source")
    }
}

#[test]
fn generated_plan_ids_are_unique_and_carry_their_kind() {
    let factory = UuidCliIdFactory::default();
    let first = factory.next_plan_id();
    let second = factory.next_plan_id();
    let bulk = factory.next_bulk_plan_id();

    assert_ne!(first.as_str(), second.as_str());
    assert!(first.as_str().starts_with("cli-plan-"));
    assert!(bulk.as_str().starts_with("cli-bulk-"));
    // Two plans prepared in the same millisecond stay distinguishable in a log read afterwards.
    assert_ne!(first.as_str(), bulk.as_str());
}

#[test]
fn a_registry_keys_every_adapter_by_the_id_it_reports() {
    let registry = CliSourceAdapterRegistry::default()
        .with(Arc::new(StubSource(source("npm"))))
        .with(Arc::new(StubSource(source("winget"))));

    assert_eq!(registry.registered_source_ids(), vec!["npm", "winget"]);
    assert!(registry.adapter(&source("npm")).is_some());
    // A plan naming a source nothing is registered for resolves to nothing, rather than to
    // whichever adapter happens to be available.
    assert!(registry.adapter(&source("vendor")).is_none());
}

#[test]
fn a_registered_adapter_answers_for_its_own_id_only() {
    let registry = CliSourceAdapterRegistry::default().with(Arc::new(StubSource(source("npm"))));

    let resolved = registry.adapter(&source("npm")).expect("npm adapter");
    assert_eq!(resolved.source_id().as_str(), "npm");
}

#[test]
fn one_tool_cannot_hold_two_mutations_at_once() {
    let coordinator = CliEnvironmentMutationCoordinator::default();
    let first = coordinator
        .try_reserve(&tool("claude-code"), &CliMutationKey::npm_global())
        .expect("reserve")
        .expect("granted");

    // Same tool, different resource: still refused. One machine change per tool.
    assert!(coordinator
        .try_reserve(&tool("claude-code"), &CliMutationKey::vendor("claude-code"))
        .expect("reserve")
        .is_none());

    first.release();
    assert!(coordinator
        .try_reserve(&tool("claude-code"), &CliMutationKey::vendor("claude-code"))
        .expect("reserve")
        .is_some());
}

#[test]
fn one_package_manager_resource_cannot_be_written_by_two_tools() {
    let coordinator = CliEnvironmentMutationCoordinator::default();
    let _first = coordinator
        .try_reserve(&tool("claude-code"), &CliMutationKey::npm_global())
        .expect("reserve")
        .expect("granted");

    // Different tool, same npm global tree.
    assert!(coordinator
        .try_reserve(&tool("codex-cli"), &CliMutationKey::npm_global())
        .expect("reserve")
        .is_none());
}

#[test]
fn independent_resources_run_concurrently_up_to_the_documented_bound() {
    let coordinator = CliEnvironmentMutationCoordinator::default();
    let _npm = coordinator
        .try_reserve(&tool("claude-code"), &CliMutationKey::npm_global())
        .expect("reserve")
        .expect("granted");
    let _vendor = coordinator
        .try_reserve(
            &tool("antigravity-cli"),
            &CliMutationKey::vendor("antigravity-cli"),
        )
        .expect("reserve")
        .expect("granted");

    assert_eq!(coordinator.global_capacity(), 2);
    // The third is refused by the global ceiling even though its resource is free.
    assert!(coordinator
        .try_reserve(&tool("opencode"), &CliMutationKey::vendor("opencode"))
        .expect("reserve")
        .is_none());
}

#[test]
fn a_dropped_lease_releases_its_reservation() {
    let coordinator = CliEnvironmentMutationCoordinator::default();
    {
        let _lease = coordinator
            .try_reserve(&tool("claude-code"), &CliMutationKey::npm_global())
            .expect("reserve")
            .expect("granted");
        // A panicking or early-returning use case must not leave a tool locked forever.
    }

    assert!(coordinator
        .try_reserve(&tool("claude-code"), &CliMutationKey::npm_global())
        .expect("reserve")
        .is_some());
}

#[test]
fn releasing_twice_does_not_free_someone_elses_reservation() {
    let coordinator = CliEnvironmentMutationCoordinator::default();
    let lease = coordinator
        .try_reserve(&tool("claude-code"), &CliMutationKey::npm_global())
        .expect("reserve")
        .expect("granted");
    lease.release();

    let second = coordinator
        .try_reserve(&tool("codex-cli"), &CliMutationKey::npm_global())
        .expect("reserve")
        .expect("granted");
    // The first lease's second release must not remove the reservation the second one now holds.
    lease.release();

    assert!(coordinator
        .try_reserve(&tool("opencode"), &CliMutationKey::npm_global())
        .expect("reserve")
        .is_none());
    second.release();
}

#[test]
fn detection_waits_while_a_resource_is_being_written() {
    let coordinator = CliEnvironmentMutationCoordinator::default();
    assert!(coordinator.may_detect_now(&CliMutationKey::npm_global()));

    let lease = coordinator
        .try_reserve(&tool("claude-code"), &CliMutationKey::npm_global())
        .expect("reserve")
        .expect("granted");

    // No source declares reads safe during its own writes, so probing npm's tree now would read a
    // half-installed state and report it as the machine's.
    assert!(!coordinator.may_detect_now(&CliMutationKey::npm_global()));
    // An unrelated resource is unaffected.
    assert!(coordinator.may_detect_now(&CliMutationKey::winget()));

    lease.release();
    assert!(coordinator.may_detect_now(&CliMutationKey::npm_global()));
}
