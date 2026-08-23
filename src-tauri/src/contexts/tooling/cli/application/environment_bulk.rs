//! Upgrading several CLIs as one reviewed batch, and the read-only Doctor probe.
//!
//! Bulk preparation produces a real plan per eligible tool and a stable reason code per skipped
//! one, so the preview the user confirms is the work that will run. Execution schedules those
//! plans through the same coordinator a single action uses -- there is no bulk-only path that
//! bypasses the one-mutation-per-tool rule.
//!
//! One item failing or going stale never erases the others' outcomes.

use super::environment_error::CliEnvironmentError;
use super::environment_planning::{
    CliActionExecutionReport, ExecuteCliActionInput, PrepareCliActionInput,
};
use super::environment_service::CliEnvironmentService;
use crate::contexts::tooling::cli::domain::action::{
    resolve_target, CliActionKind, CliTargetResolution,
};
use crate::contexts::tooling::cli::domain::bulk::{
    CliBulkActionItem, CliBulkActionPlan, CliBulkItemResult, CliBulkItemStatus, CliBulkSkip,
    CliBulkSkipReason,
};
use crate::contexts::tooling::cli::domain::ids::{CliBulkPlanId, CliToolId};
use crate::contexts::tooling::cli::domain::phase::CliOperationPhase;
use crate::contexts::tooling::cli::domain::plan::{CliActionPlan, CliActionPlanState};
use crate::contexts::tooling::cli::domain::registry::CLI_TOOL_DEFINITIONS;
use crate::contexts::tooling::cli::domain::snapshot::{CliEnvironmentSnapshot, CliMutationOutcome};
use crate::contexts::tooling::cli::domain::status::{CliOverallState, CliUpdateStatus};
use crate::contexts::tooling::cli::domain::version::NormalizedCliVersion;

#[derive(Debug, Clone)]
pub(crate) struct PreparedCliBulkPlanning {
    pub(crate) operation_id: String,
    agent_ids: Vec<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct PreparedCliBulkExecution {
    pub(crate) operation_id: String,
    plan_id: CliBulkPlanId,
    expected_revision: u32,
}

#[derive(Debug, Clone)]
pub(crate) struct PreparedCliDoctor {
    pub(crate) operation_id: String,
    agent_id: CliToolId,
}

impl CliEnvironmentService {
    pub(crate) fn prepare_cli_bulk_upgrade(
        &self,
        agent_ids: Vec<String>,
    ) -> Result<PreparedCliBulkPlanning, CliEnvironmentError> {
        for agent_id in &agent_ids {
            self.resolve_tool(agent_id)?;
        }
        let operation_id = self
            .ports
            .operations
            .start(None, "Preparing CLI upgrades".to_string())?;
        Ok(PreparedCliBulkPlanning {
            operation_id,
            agent_ids,
        })
    }

    pub(crate) fn execute_bulk_planning(
        &self,
        prepared: PreparedCliBulkPlanning,
    ) -> Result<(), CliEnvironmentError> {
        let operation_id = prepared.operation_id.clone();
        match self.build_bulk_plan(&prepared) {
            Ok(plan) => self.ports.operations.complete(
                &operation_id,
                serde_json::json!({
                    "planId": plan.id.as_str(),
                    "revision": plan.revision,
                    "eligible": plan.items.len(),
                    "skipped": plan.skipped.len(),
                }),
            ),
            Err(error) => {
                let message = error.to_string();
                self.ports
                    .diagnostics
                    .record(&operation_id, None, None, &message);
                self.ports.operations.fail(&operation_id, message)
            }
        }
    }

    fn build_bulk_plan(
        &self,
        prepared: &PreparedCliBulkPlanning,
    ) -> Result<CliBulkActionPlan, CliEnvironmentError> {
        let operation_id = &prepared.operation_id;
        self.ports
            .operations
            .report_phase(operation_id, CliOperationPhase::Planning, true)?;

        let fingerprint = self.ports.discovery.environment_fingerprint()?;
        let now = self.ports.clock.now();
        let candidates: Vec<String> = if prepared.agent_ids.is_empty() {
            CLI_TOOL_DEFINITIONS
                .iter()
                .map(|definition| definition.agent_id.to_string())
                .collect()
        } else {
            prepared.agent_ids.clone()
        };

        let mut items = Vec::new();
        let mut item_plans = Vec::new();
        let mut skipped = Vec::new();

        for agent_id in candidates {
            let (tool_id, _) = self.resolve_tool(&agent_id)?;
            let snapshot = self.snapshot_or_never_scanned(&tool_id, &fingerprint)?;
            match self.bulk_candidate(&snapshot) {
                Err(reason) => skipped.push(CliBulkSkip {
                    agent_id: tool_id,
                    reason,
                }),
                Ok((source_id, target)) => {
                    // A real single-use plan per item, built by the same path a single action uses,
                    // so the preview and the execution cannot disagree.
                    let planning = self.prepare_cli_action(PrepareCliActionInput {
                        agent_id: agent_id.clone(),
                        // A bulk batch is upgrades by construction; nothing is derived here.
                        action: Some(CliActionKind::Upgrade),
                        source_id: source_id.clone(),
                        target_version: Some(target.clone()),
                        channel: None,
                    })?;
                    self.execute_action_planning(planning)?;

                    // The plan that sub-operation just persisted is the newest draft for this tool.
                    let Some(plan) = self.latest_draft_plan_for(&tool_id)? else {
                        skipped.push(CliBulkSkip {
                            agent_id: tool_id,
                            reason: CliBulkSkipReason::UnsupportedAction,
                        });
                        continue;
                    };
                    items.push(CliBulkActionItem {
                        agent_id: tool_id,
                        plan_id: plan.id.clone(),
                        source_id: plan.source_id.clone(),
                        current_version: plan.current_version.clone(),
                        target_version: plan.target_version.clone(),
                        requires_elevation: plan.requires_elevation,
                        requires_network: plan.requires_network,
                        state: CliActionPlanState::Draft,
                        skipped_reason: None,
                    });
                    item_plans.push(plan);
                }
            }
        }

        let bulk = CliBulkActionPlan {
            id: self.ports.ids.next_bulk_plan_id(),
            revision: 1,
            items,
            skipped,
            environment_fingerprint: fingerprint,
            created_at: now,
            expires_at: CliActionPlan::default_expiry(now),
        };
        // The batch and every item plan land together or not at all.
        self.ports
            .repository
            .create_bulk_plan_atomic(&bulk, &item_plans)?;
        Ok(bulk)
    }

    /// Whether a tool belongs in the batch, and if not, the stable code saying why.
    fn bulk_candidate(
        &self,
        snapshot: &CliEnvironmentSnapshot,
    ) -> Result<(String, String), CliBulkSkipReason> {
        if !snapshot.discovery.is_installed() {
            return Err(CliBulkSkipReason::NotInstalled);
        }
        // A conflict that makes the target ambiguous excludes the tool from the batch entirely.
        // Upgrading "whichever installation we guess" is exactly what the conflict says not to do.
        if snapshot.blocks_mutation() {
            return Err(CliBulkSkipReason::InstallationConflict);
        }
        match snapshot.overall_state {
            CliOverallState::Broken => return Err(CliBulkSkipReason::Broken),
            CliOverallState::NeedsAuth => return Err(CliBulkSkipReason::NeedsAuth),
            _ => {}
        }
        match snapshot.update {
            CliUpdateStatus::UpToDate | CliUpdateStatus::Ahead => {
                return Err(CliBulkSkipReason::AlreadyCurrent)
            }
            CliUpdateStatus::NotApplicable => return Err(CliBulkSkipReason::DetectOnlySource),
            CliUpdateStatus::CatalogUnavailable => {
                return Err(CliBulkSkipReason::CatalogUnavailable)
            }
            CliUpdateStatus::Unknown => return Err(CliBulkSkipReason::UnorderedVersions),
            CliUpdateStatus::Available => {}
        }

        let installation = snapshot
            .recommended_installation()
            .ok_or(CliBulkSkipReason::NotInstalled)?;
        let source_id = installation
            .source_id
            .as_ref()
            .ok_or(CliBulkSkipReason::SourceOwnershipUnproven)?;
        let upgrade = snapshot
            .allowed_actions
            .iter()
            .find(|action| {
                action.action == CliActionKind::Upgrade
                    && &action.source_id == source_id
                    && action.reason_code.is_none()
            })
            .ok_or(CliBulkSkipReason::UnsupportedAction)?;
        let target = upgrade
            .default_target
            .clone()
            .ok_or(CliBulkSkipReason::CatalogUnavailable)?;

        // Belt and braces: the derived action already excluded equality, and this re-checks it
        // against the version actually held.
        let active = installation.reported_version.as_ref();
        if resolve_target(active, &NormalizedCliVersion::parse(&target))
            == CliTargetResolution::Current
        {
            return Err(CliBulkSkipReason::AlreadyCurrent);
        }
        Ok((source_id.as_str().to_string(), target))
    }

    fn latest_draft_plan_for(
        &self,
        agent_id: &CliToolId,
    ) -> Result<Option<CliActionPlan>, CliEnvironmentError> {
        // The repository has no "find by tool" query by design; planning just persisted one, and
        // the bulk path is the only caller that needs to look it back up.
        let mut newest: Option<CliActionPlan> = None;
        for candidate in self.ports.repository.list_draft_plans(agent_id)? {
            if newest
                .as_ref()
                .is_none_or(|current| candidate.created_at >= current.created_at)
            {
                newest = Some(candidate);
            }
        }
        Ok(newest)
    }

    pub(crate) fn get_cli_bulk_action_plan(
        &self,
        plan_id: &str,
    ) -> Result<CliBulkActionPlan, CliEnvironmentError> {
        let id = CliBulkPlanId::new(plan_id.to_string())
            .map_err(|error| CliEnvironmentError::Validation(error.to_string()))?;
        self.ports
            .repository
            .load_bulk_plan(&id)?
            .ok_or(CliEnvironmentError::PlanNotFound)
    }

    pub(crate) fn prepare_cli_bulk_execution(
        &self,
        plan_id: &str,
        expected_revision: u32,
    ) -> Result<PreparedCliBulkExecution, CliEnvironmentError> {
        let id = CliBulkPlanId::new(plan_id.to_string())
            .map_err(|error| CliEnvironmentError::Validation(error.to_string()))?;
        let operation_id = self
            .ports
            .operations
            .start(None, "Upgrading CLI tools".to_string())?;
        Ok(PreparedCliBulkExecution {
            operation_id,
            plan_id: id,
            expected_revision,
        })
    }

    pub(crate) fn execute_cli_bulk_action(
        &self,
        prepared: PreparedCliBulkExecution,
    ) -> Result<(), CliEnvironmentError> {
        let operation_id = prepared.operation_id.clone();
        match self.run_bulk(&prepared) {
            // The batch succeeded because it collected every item's terminal result. An item that
            // failed is reported in that item's result, not as a failure of the orchestration --
            // conflating the two is how a batch of five with one bad item reads as five failures.
            Ok(results) => self.ports.operations.complete(
                &operation_id,
                serde_json::json!({ "items": encode_item_results(&results) }),
            ),
            // Only the orchestration itself failing gets here: the bulk plan could not be loaded,
            // its revision moved, or it expired before execution started.
            Err(error) => {
                let message = error.to_string();
                self.ports
                    .diagnostics
                    .record(&operation_id, None, None, &message);
                self.ports.operations.fail(&operation_id, message)
            }
        }
    }

    fn run_bulk(
        &self,
        prepared: &PreparedCliBulkExecution,
    ) -> Result<Vec<CliBulkItemResult>, CliEnvironmentError> {
        let operation_id = &prepared.operation_id;
        let cancellation = self.ports.operations.cancellation(operation_id)?;
        let bulk = self
            .ports
            .repository
            .load_bulk_plan(&prepared.plan_id)?
            .ok_or(CliEnvironmentError::PlanNotFound)?;
        if bulk.revision != prepared.expected_revision {
            return Err(CliEnvironmentError::PlanRevisionMismatch {
                expected: prepared.expected_revision,
                actual: bulk.revision,
            });
        }
        if bulk.is_expired(self.ports.clock.now()) {
            return Err(CliEnvironmentError::PlanExpired);
        }

        // Tools the plan already excluded are carried into the results. Reporting only the items
        // that ran would leave a tool the user asked about with no answer at all.
        let mut results: Vec<CliBulkItemResult> = bulk
            .skipped
            .iter()
            .map(|skip| CliBulkItemResult::skipped(skip.agent_id.clone(), skip.reason))
            .collect();

        let total = u32::try_from(bulk.items.len()).unwrap_or(u32::MAX);
        for (index, item) in bulk.items.iter().enumerate() {
            let status = if cancellation.is_cancelled() {
                // Never started. `Cancelled` is the truthful outcome: nothing ran, so nothing on
                // the machine changed, and silently dropping the item would say neither.
                CliBulkItemStatus::Completed(CliMutationOutcome::Cancelled)
            } else {
                self.run_bulk_item(item)
            };
            results.push(CliBulkItemResult::for_item(item, status));

            let completed = u32::try_from(index + 1).unwrap_or(u32::MAX);
            self.ports
                .operations
                .report_units(operation_id, completed, total)?;
        }
        self.ports
            .operations
            .report_phase(operation_id, CliOperationPhase::Completed, false)?;
        Ok(results)
    }

    /// One item, through the same single-action path a lone mutation takes.
    ///
    /// Not a simplified copy of it: the same admission, the same coordinator reservation, the same
    /// post-mutation verification. A batch that had its own package-manager flow would be a second
    /// implementation of the rules, and the two would disagree the first time one changed.
    fn run_bulk_item(&self, item: &CliBulkActionItem) -> CliBulkItemStatus {
        let execution = self.prepare_cli_action_execution(ExecuteCliActionInput {
            plan_id: item.plan_id.as_str().to_string(),
            expected_revision: 1,
        });
        let execution = match execution {
            Ok(execution) => execution,
            Err(error) => return CliBulkItemStatus::Skipped(skip_reason_for(&error)),
        };
        match self.execute_action_recording(execution) {
            // The record carries the five-state outcome the single-action path derived.
            Ok(CliActionExecutionReport::Recorded(record)) => match record.outcome {
                Some(outcome) => CliBulkItemStatus::Completed(outcome),
                // A record with no outcome means the run never reached verification.
                None => CliBulkItemStatus::Skipped(CliBulkSkipReason::OperationConflict),
            },
            Ok(CliActionExecutionReport::Refused(error)) => {
                CliBulkItemStatus::Skipped(skip_reason_for(&error))
            }
            // The operations store itself failed. Nothing about this item was recorded, so the
            // batch says so rather than claiming an outcome it never observed.
            Err(error) => CliBulkItemStatus::Skipped(skip_reason_for(&error)),
        }
    }

    pub(crate) fn prepare_cli_doctor(
        &self,
        agent_id: &str,
    ) -> Result<PreparedCliDoctor, CliEnvironmentError> {
        let (tool_id, _) = self.resolve_tool(agent_id)?;
        let operation_id = self
            .ports
            .operations
            .start(Some(&tool_id), format!("Running diagnostics for {tool_id}"))?;
        Ok(PreparedCliDoctor {
            operation_id,
            agent_id: tool_id,
        })
    }

    pub(crate) fn execute_cli_doctor(
        &self,
        prepared: PreparedCliDoctor,
    ) -> Result<(), CliEnvironmentError> {
        let operation_id = prepared.operation_id.clone();
        match self.run_doctor(&prepared) {
            Ok(result) => self.ports.operations.complete(&operation_id, result),
            Err(error) => {
                let message = error.to_string();
                self.ports.diagnostics.record(
                    &operation_id,
                    Some(&prepared.agent_id),
                    None,
                    &message,
                );
                self.ports.operations.fail(&operation_id, message)
            }
        }
    }

    fn run_doctor(
        &self,
        prepared: &PreparedCliDoctor,
    ) -> Result<serde_json::Value, CliEnvironmentError> {
        let operation_id = &prepared.operation_id;
        let cancellation = self.ports.operations.cancellation(operation_id)?;
        let (_, definition) = self.resolve_tool(prepared.agent_id.as_str())?;
        let fingerprint = self.ports.discovery.environment_fingerprint()?;
        let snapshot = self.snapshot_or_never_scanned(&prepared.agent_id, &fingerprint)?;

        self.ports
            .operations
            .report_phase(operation_id, CliOperationPhase::RunningDoctor, true)?;

        let Some(installation) = snapshot.recommended_installation() else {
            return Ok(serde_json::json!({
                "agentId": prepared.agent_id.as_str(),
                "doctor": "unknown",
                "reason": "not-installed",
            }));
        };
        let Some(command) = definition.probes.doctor.command() else {
            // No documented non-interactive Doctor command. Unknown is the truthful answer; the
            // alternative would be inventing a health verdict from silence.
            return Ok(serde_json::json!({
                "agentId": prepared.agent_id.as_str(),
                "doctor": "unknown",
                "reason": "undocumented-probe",
            }));
        };

        let outcome =
            self.ports
                .probes
                .run_probe(&installation.executable_path, command, &cancellation)?;
        Ok(serde_json::json!({
            "agentId": prepared.agent_id.as_str(),
            "doctor": if outcome.succeeded() { "ok" } else { "problem" },
            "timedOut": outcome.timed_out,
            "truncated": outcome.truncated,
        }))
    }

    /// Bounded maintenance: marks a slice of expired draft plans so the table cannot grow without
    /// limit. Called from refresh, which already runs off the command boundary.
    pub(crate) fn expire_stale_plans(&self) -> Result<usize, CliEnvironmentError> {
        self.ports
            .repository
            .expire_stale_plans(self.ports.clock.now(), 64)
    }
}

/// Maps a refusal from the single-action path onto this item's skip reason.
///
/// Every arm is a stable code the UI localizes. Surfacing the error message instead would put a
/// sentence where a code belongs and lose the ability to group items by cause.
fn skip_reason_for(error: &CliEnvironmentError) -> CliBulkSkipReason {
    match error {
        CliEnvironmentError::PlanStale
        | CliEnvironmentError::PlanRevisionMismatch { .. }
        | CliEnvironmentError::PlanNotFound => CliBulkSkipReason::PlanStale,
        CliEnvironmentError::PlanExpired => CliBulkSkipReason::PlanExpired,
        CliEnvironmentError::PlanConsumed => CliBulkSkipReason::PlanConsumed,
        CliEnvironmentError::OperationConflict { .. } => CliBulkSkipReason::OperationConflict,
        CliEnvironmentError::CatalogUnavailable { .. } => CliBulkSkipReason::CatalogUnavailable,
        CliEnvironmentError::UnsupportedAction { .. }
        | CliEnvironmentError::UnsupportedSource { .. }
        | CliEnvironmentError::RuntimeUnsupported => CliBulkSkipReason::UnsupportedAction,
        CliEnvironmentError::SourceUnavailable { .. }
        | CliEnvironmentError::MissingDependency { .. } => CliBulkSkipReason::DetectOnlySource,
        CliEnvironmentError::ElevationRequired => CliBulkSkipReason::NeedsAuth,
        // Anything else stopped this item before it could run, and the executable is the first
        // thing the user will look at.
        _ => CliBulkSkipReason::Broken,
    }
}

/// The wire shape of the per-item results.
///
/// A discriminated union: `status` says which arm, and exactly one of `outcome` and `reason` is
/// populated. Both keys are always present so a reader never has to tell "absent" from
/// "not applicable".
fn encode_item_results(results: &[CliBulkItemResult]) -> Vec<serde_json::Value> {
    results
        .iter()
        .map(|result| {
            serde_json::json!({
                "agentId": result.agent_id.as_str(),
                "planId": result.plan_id.as_ref().map(|id| id.as_str()),
                "sourceId": result.source_id.as_ref().map(|id| id.as_str()),
                "targetVersion": result.target_version,
                "status": result.status.kind(),
                "outcome": result.status.outcome().map(CliMutationOutcome::as_str),
                "reason": result.status.reason().map(CliBulkSkipReason::as_str),
            })
        })
        .collect()
}

#[cfg(test)]
#[path = "environment_bulk_tests.rs"]
mod tests;
