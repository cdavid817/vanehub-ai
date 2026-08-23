//! The published surface of the source-aware CLI environment context.
//!
//! Commands depend on this, never on the service or its ports. Every method that can touch a
//! process, a package manager, or the network is split in two: a `prepare_*` that returns an
//! operation id without doing any of it, and an `execute_*` a background thread runs. Cached reads
//! and persisted-plan reads stay direct, because a bounded read that returns an operation id would
//! make the caller poll for something already known.

use crate::contexts::tooling::cli::application::environment_bulk::{
    PreparedCliBulkExecution, PreparedCliBulkPlanning, PreparedCliDoctor,
};
use crate::contexts::tooling::cli::application::environment_launch::CliLaunchTarget;
use crate::contexts::tooling::cli::application::environment_planning::{
    ExecuteCliActionInput, PrepareCliActionInput, PreparedCliActionExecution,
    PreparedCliActionPlanning,
};
use crate::contexts::tooling::cli::application::environment_refresh::PreparedEnvironmentRefresh;
use crate::contexts::tooling::cli::application::environment_service::CliEnvironmentService;
use crate::contexts::tooling::cli::domain::bulk::CliBulkActionPlan;
use crate::contexts::tooling::cli::domain::plan::CliActionPlan;
use crate::contexts::tooling::cli::domain::snapshot::CliEnvironmentSnapshot;

pub(crate) use crate::contexts::tooling::cli::application::environment_error::CliEnvironmentError;

#[derive(Clone)]
pub(crate) struct CliEnvironmentApi {
    service: CliEnvironmentService,
}

impl CliEnvironmentApi {
    pub(crate) fn new(service: CliEnvironmentService) -> Self {
        Self { service }
    }

    /// Which executable the Agent Runtime should launch for a tool.
    ///
    /// Bounded: a stored snapshot, or a bounded live lookup when nothing has been scanned yet.
    /// Never a probe and never a mutation.
    pub(crate) fn resolve_launch_target(
        &self,
        agent_id: &str,
    ) -> Result<CliLaunchTarget, CliEnvironmentError> {
        self.service.resolve_launch_target(agent_id)
    }

    /// Bounded: reads storage and computes the environment fingerprint. Starts nothing.
    pub(crate) fn list_environments(
        &self,
    ) -> Result<Vec<CliEnvironmentSnapshot>, CliEnvironmentError> {
        self.service.list_cli_environments()
    }

    pub(crate) fn prepare_refresh(
        &self,
        agent_ids: Vec<String>,
        force_catalog: bool,
    ) -> Result<PreparedEnvironmentRefresh, CliEnvironmentError> {
        self.service.prepare_refresh(agent_ids, force_catalog)
    }

    pub(crate) fn execute_refresh(
        &self,
        prepared: PreparedEnvironmentRefresh,
    ) -> Result<(), CliEnvironmentError> {
        self.service.execute_refresh(prepared)
    }

    pub(crate) fn prepare_action(
        &self,
        input: PrepareCliActionInput,
    ) -> Result<PreparedCliActionPlanning, CliEnvironmentError> {
        self.service.prepare_cli_action(input)
    }

    pub(crate) fn execute_action_planning(
        &self,
        prepared: PreparedCliActionPlanning,
    ) -> Result<(), CliEnvironmentError> {
        self.service.execute_action_planning(prepared)
    }

    /// Direct: a persisted plan is a stored row, and reviewing it starts nothing.
    pub(crate) fn get_action_plan(
        &self,
        plan_id: &str,
    ) -> Result<CliActionPlan, CliEnvironmentError> {
        self.service.get_cli_action_plan(plan_id)
    }

    pub(crate) fn prepare_action_execution(
        &self,
        input: ExecuteCliActionInput,
    ) -> Result<PreparedCliActionExecution, CliEnvironmentError> {
        self.service.prepare_cli_action_execution(input)
    }

    pub(crate) fn execute_action(
        &self,
        prepared: PreparedCliActionExecution,
    ) -> Result<(), CliEnvironmentError> {
        self.service.execute_cli_action(prepared)
    }

    pub(crate) fn prepare_bulk_planning(
        &self,
        agent_ids: Vec<String>,
    ) -> Result<PreparedCliBulkPlanning, CliEnvironmentError> {
        self.service.prepare_cli_bulk_upgrade(agent_ids)
    }

    pub(crate) fn execute_bulk_planning(
        &self,
        prepared: PreparedCliBulkPlanning,
    ) -> Result<(), CliEnvironmentError> {
        self.service.execute_bulk_planning(prepared)
    }

    pub(crate) fn get_bulk_action_plan(
        &self,
        plan_id: &str,
    ) -> Result<CliBulkActionPlan, CliEnvironmentError> {
        self.service.get_cli_bulk_action_plan(plan_id)
    }

    pub(crate) fn prepare_bulk_execution(
        &self,
        plan_id: &str,
        expected_revision: u32,
    ) -> Result<PreparedCliBulkExecution, CliEnvironmentError> {
        self.service
            .prepare_cli_bulk_execution(plan_id, expected_revision)
    }

    pub(crate) fn execute_bulk_action(
        &self,
        prepared: PreparedCliBulkExecution,
    ) -> Result<(), CliEnvironmentError> {
        self.service.execute_cli_bulk_action(prepared)
    }

    pub(crate) fn prepare_doctor(
        &self,
        agent_id: &str,
    ) -> Result<PreparedCliDoctor, CliEnvironmentError> {
        self.service.prepare_cli_doctor(agent_id)
    }

    pub(crate) fn execute_doctor(
        &self,
        prepared: PreparedCliDoctor,
    ) -> Result<(), CliEnvironmentError> {
        self.service.execute_cli_doctor(prepared)
    }
}
