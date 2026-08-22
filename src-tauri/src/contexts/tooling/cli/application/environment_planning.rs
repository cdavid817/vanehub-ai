//! Preparing, reviewing, and executing a single machine change.
//!
//! The shape is the point. `prepare_cli_action` receives the source, channel, and target the user
//! chose and records them in a persisted plan. `execute_cli_action` receives a plan id and a
//! revision and nothing else -- there is no parameter it could rebuild a command from, so the
//! selected version cannot be dropped between review and execution.
//!
//! Four rules enforced here that the previous implementation broke:
//!
//! - a target equal to the active version produces no plan at all;
//! - the source recorded in the plan is the source that runs, with no fallback;
//! - post-mutation detection runs after success *and* failure, and its result is what gets saved;
//! - a failed verification never restores the pre-operation snapshot.

use chrono::{DateTime, Utc};

use super::environment_error::CliEnvironmentError;
use super::environment_ports::{CliCancellation, CliOutputSink, CliPhaseSink, CliPlanRequest};
use super::environment_service::CliEnvironmentService;
use crate::contexts::tooling::cli::domain::action::{
    resolve_target, CliActionKind, CliTargetResolution,
};
use crate::contexts::tooling::cli::domain::definition::{
    CliDistributionAction, CliDistributionDefinition, CliToolDefinition,
};
use crate::contexts::tooling::cli::domain::ids::{CliActionPlanId, CliSourceId, CliToolId};
use crate::contexts::tooling::cli::domain::operation_record::{
    CliOperationRecord, CliOperationTermination, CliVerificationWarning,
};
use crate::contexts::tooling::cli::domain::phase::CliOperationPhase;
use crate::contexts::tooling::cli::domain::plan::{
    CliActionPlan, CliActionPlanState, CliFallbackPolicy, CliPlanWarning, CliPrecondition,
};
use crate::contexts::tooling::cli::domain::snapshot::{
    CliEnvironmentSnapshot, CliMutationOutcome, CliMutationSummary,
};
use crate::contexts::tooling::cli::domain::source::{CliPlatform, CliTargetVersionMode};
use crate::contexts::tooling::cli::domain::version::NormalizedCliVersion;

/// What the frontend submits to prepare a plan. Exactly the fields the user chose.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PrepareCliActionInput {
    pub(crate) agent_id: String,
    pub(crate) action: CliActionKind,
    pub(crate) source_id: String,
    pub(crate) target_version: Option<String>,
    pub(crate) channel: Option<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct PreparedCliActionPlanning {
    pub(crate) operation_id: String,
    input: PrepareCliActionInput,
}

/// What the frontend submits to execute. No command, no version, no source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ExecuteCliActionInput {
    pub(crate) plan_id: String,
    pub(crate) expected_revision: u32,
}

#[derive(Debug, Clone)]
pub(crate) struct PreparedCliActionExecution {
    pub(crate) operation_id: String,
    plan_id: CliActionPlanId,
    expected_revision: u32,
}

/// Streams adapter output onto the operation.
struct OperationSink<'a> {
    service: &'a CliEnvironmentService,
    operation_id: String,
}

impl CliOutputSink for OperationSink<'_> {
    fn emit(&self, line: &str) {
        // Already bounded and redacted by the adapter; failure to record output must not abort the
        // mutation that is otherwise proceeding.
        let _ = self
            .service
            .ports
            .operations
            .append_output(&self.operation_id, line);
    }
}

/// Relays the phases an adapter announces onto the operation.
struct OperationPhases<'a> {
    service: &'a CliEnvironmentService,
    operation_id: String,
}

impl CliPhaseSink for OperationPhases<'_> {
    fn enter(&self, phase: CliOperationPhase, cancellable: bool) {
        // A failure to record a label must not abort a mutation that is otherwise proceeding --
        // least of all one that has already started writing.
        let _ = self
            .service
            .ports
            .operations
            .report_phase(&self.operation_id, phase, cancellable);
    }
}

impl CliEnvironmentService {
    pub(crate) fn prepare_cli_action(
        &self,
        input: PrepareCliActionInput,
    ) -> Result<PreparedCliActionPlanning, CliEnvironmentError> {
        // Validated before an operation exists, so a typo does not leave a failed operation behind.
        let (agent_id, _) = self.resolve_tool(&input.agent_id)?;
        let operation_id = self.ports.operations.start(
            Some(&agent_id),
            format!("Preparing {} for {}", input.action.as_str(), agent_id),
        )?;
        Ok(PreparedCliActionPlanning {
            operation_id,
            input,
        })
    }

    pub(crate) fn execute_action_planning(
        &self,
        prepared: PreparedCliActionPlanning,
    ) -> Result<(), CliEnvironmentError> {
        let operation_id = prepared.operation_id.clone();
        match self.build_plan(&prepared) {
            Ok(plan) => self.ports.operations.complete(
                &operation_id,
                serde_json::json!({
                    "planId": plan.id.as_str(),
                    "revision": plan.revision,
                    "agentId": plan.agent_id.as_str(),
                    "sourceId": plan.source_id.as_str(),
                    "targetVersion": plan.target_version,
                }),
            ),
            Err(error) => {
                let message = error.to_string();
                self.ports.diagnostics.record(
                    &operation_id,
                    None,
                    Some(prepared.input.action),
                    &message,
                );
                self.ports.operations.fail(&operation_id, message)
            }
        }
    }

    fn build_plan(
        &self,
        prepared: &PreparedCliActionPlanning,
    ) -> Result<CliActionPlan, CliEnvironmentError> {
        let operation_id = &prepared.operation_id;
        let input = &prepared.input;
        let cancellation = self.ports.operations.cancellation(operation_id)?;
        let (agent_id, definition) = self.resolve_tool(&input.agent_id)?;

        self.ports
            .operations
            .report_phase(operation_id, CliOperationPhase::Preflight, true)?;
        let source_id = CliSourceId::new(input.source_id.clone())
            .map_err(|error| CliEnvironmentError::Validation(error.to_string()))?;
        let distribution = definition.distribution(source_id.as_str()).ok_or_else(|| {
            CliEnvironmentError::UnsupportedSource {
                agent_id: agent_id.as_str().to_string(),
                source_id: source_id.as_str().to_string(),
            }
        })?;
        let platform = CliPlatform::current().ok_or(CliEnvironmentError::RuntimeUnsupported)?;
        if !distribution.is_actionable_on(platform) {
            // Includes a vendor with no template for this platform. Withholding here is what stops
            // a Bash installer being selected on Windows.
            return Err(CliEnvironmentError::RuntimeUnsupported);
        }

        self.ports.operations.report_phase(
            operation_id,
            CliOperationPhase::ResolvingSource,
            true,
        )?;
        let adapter = self.ports.sources.adapter(&source_id).ok_or_else(|| {
            CliEnvironmentError::SourceUnavailable {
                source_id: source_id.as_str().to_string(),
            }
        })?;
        let preflight = adapter.preflight(distribution, &cancellation)?;
        if !preflight.available {
            return Err(CliEnvironmentError::SourceUnavailable {
                source_id: source_id.as_str().to_string(),
            });
        }

        let fingerprint = self.ports.discovery.environment_fingerprint()?;
        let snapshot = self.snapshot_or_never_scanned(&agent_id, &fingerprint)?;
        let active_version = snapshot
            .recommended_installation()
            .and_then(|installation| installation.reported_version.clone());

        self.ports
            .operations
            .report_phase(operation_id, CliOperationPhase::Planning, true)?;
        // An unspecified channel means the source's default, not "no channel". Leaving it unset
        // would look up a catalog under a different key than the one refresh stored.
        let channel = input.channel.clone().or_else(|| {
            distribution
                .default_channel()
                .map(|channel| channel.id.to_string())
        });
        let target = self.resolve_plan_target(
            input,
            channel.as_deref(),
            &agent_id,
            distribution,
            platform,
            active_version.as_ref(),
        )?;

        let mut warnings = Vec::new();
        let target_mode = version_mode(distribution, input.action, platform);
        if target_mode == CliTargetVersionMode::LatestOnly && target.is_some() {
            // The source runs at whatever it considers latest. Saying so is what stops the result
            // being labelled with a version the source never honoured.
            warnings.push(CliPlanWarning::TargetIsLatestOnly);
        }
        if target_mode.accepts_exact_target() && !preflight.supports_exact_version {
            warnings.push(CliPlanWarning::ExactVersionNotConfirmed);
        }
        if input.action == CliActionKind::Downgrade {
            warnings.push(CliPlanWarning::DowngradeMayLoseState);
        }

        let preview = adapter.build_command_preview(
            &CliPlanRequest {
                agent_id: &agent_id,
                action: input.action,
                target_version: target.as_ref(),
                channel: channel.as_deref(),
                package_reference: distribution
                    .package_reference
                    .map(|reference| reference.identifier),
                exact_version_confirmed: preflight.supports_exact_version,
            },
            distribution,
        )?;

        let created_at = self.ports.clock.now();
        let plan = CliActionPlan {
            id: self.ports.ids.next_plan_id(),
            revision: 1,
            agent_id: agent_id.clone(),
            action: input.action,
            source_id: source_id.clone(),
            installation_id: snapshot.recommended_installation_id.clone(),
            current_version: active_version.map(|version| version.as_str().to_string()),
            target_version: target.map(|version| version.as_str().to_string()),
            // The resolved channel, so the plan states which one it was built against rather than
            // leaving the reviewer to infer a default.
            channel: channel.clone(),
            command_preview: preview,
            preconditions: preconditions_for(&source_id, &preflight.requires_elevation),
            warnings,
            requires_elevation: preflight.requires_elevation,
            requires_network: true,
            // The one policy in this change. A plan runs its disclosed source or fails.
            fallback_policy: CliFallbackPolicy::None,
            environment_fingerprint: fingerprint,
            state: CliActionPlanState::Draft,
            created_at,
            expires_at: CliActionPlan::default_expiry(created_at),
        };

        let violations = plan.violations();
        if !violations.is_empty() {
            return Err(CliEnvironmentError::Validation(format!(
                "the prepared plan is not well formed: {violations:?}"
            )));
        }
        self.ports.repository.create_action_plan(&plan)?;
        Ok(plan)
    }

    /// Validates the target the user chose against the source that will run.
    ///
    /// Returns `None` for a latest-only action, which carries no target at all. Returns an error
    /// when the target is already active -- the redundant-mutation case.
    fn resolve_plan_target(
        &self,
        input: &PrepareCliActionInput,
        channel: Option<&str>,
        agent_id: &CliToolId,
        distribution: &CliDistributionDefinition,
        platform: CliPlatform,
        active_version: Option<&NormalizedCliVersion>,
    ) -> Result<Option<NormalizedCliVersion>, CliEnvironmentError> {
        let source_id = distribution
            .source_id()
            .map_err(|error| CliEnvironmentError::Validation(error.to_string()))?;
        let source_id = &source_id;
        let mode = version_mode(distribution, input.action, platform);
        if !mode.is_supported() {
            return Err(CliEnvironmentError::UnsupportedAction {
                agent_id: agent_id.as_str().to_string(),
                source_id: source_id.as_str().to_string(),
                action: input.action.as_str(),
            });
        }
        if !matches!(
            input.action,
            CliActionKind::Install | CliActionKind::Upgrade | CliActionKind::Downgrade
        ) {
            // Uninstall and repair carry no version.
            return Ok(None);
        }

        let requested = input
            .target_version
            .as_deref()
            .map(NormalizedCliVersion::parse);
        let catalog = self
            .ports
            .repository
            .load_catalog(agent_id, source_id, channel)?;

        let target = match requested {
            Some(requested) => {
                if !mode.accepts_exact_target() {
                    // Asked for an exact version from a source that cannot aim. Running latest and
                    // calling it the requested version is exactly what must not happen.
                    return Err(CliEnvironmentError::UnsupportedAction {
                        agent_id: agent_id.as_str().to_string(),
                        source_id: source_id.as_str().to_string(),
                        action: input.action.as_str(),
                    });
                }
                let offered = catalog
                    .as_ref()
                    .is_some_and(|catalog| catalog.offers(&requested));
                if !offered {
                    return Err(CliEnvironmentError::InvalidVersion {
                        source_id: source_id.as_str().to_string(),
                        value: requested.as_str().to_string(),
                    });
                }
                requested
            }
            None => {
                let Some(latest) = catalog.as_ref().and_then(|catalog| catalog.latest.clone())
                else {
                    return Err(CliEnvironmentError::CatalogUnavailable {
                        source_id: source_id.as_str().to_string(),
                        reason:
                            crate::contexts::tooling::cli::domain::catalog::CliCatalogUnavailableReason::QueryFailed,
                    });
                };
                latest
            }
        };

        // The redundant-mutation gate. Equality has no mutation, so there is nothing to plan.
        if resolve_target(active_version, &target) == CliTargetResolution::Current {
            return Err(CliEnvironmentError::Validation(format!(
                "{} is already the active version",
                target.as_str()
            )));
        }
        Ok(if mode.accepts_exact_target() {
            Some(target)
        } else {
            // Latest-only: the plan records no target, so nothing downstream can label the result
            // with a version the source did not honour.
            None
        })
    }

    pub(crate) fn get_cli_action_plan(
        &self,
        plan_id: &str,
    ) -> Result<CliActionPlan, CliEnvironmentError> {
        let id = CliActionPlanId::new(plan_id.to_string())
            .map_err(|error| CliEnvironmentError::Validation(error.to_string()))?;
        self.ports
            .repository
            .load_action_plan(&id)?
            .ok_or(CliEnvironmentError::PlanNotFound)
    }

    pub(crate) fn prepare_cli_action_execution(
        &self,
        input: ExecuteCliActionInput,
    ) -> Result<PreparedCliActionExecution, CliEnvironmentError> {
        let plan_id = CliActionPlanId::new(input.plan_id.clone())
            .map_err(|error| CliEnvironmentError::Validation(error.to_string()))?;
        let plan = self
            .ports
            .repository
            .load_action_plan(&plan_id)?
            .ok_or(CliEnvironmentError::PlanNotFound)?;

        let operation_id = self.ports.operations.start(
            Some(&plan.agent_id),
            format!("{} {}", plan.action.as_str(), plan.agent_id),
        )?;
        Ok(PreparedCliActionExecution {
            operation_id,
            plan_id,
            expected_revision: input.expected_revision,
        })
    }

    pub(crate) fn execute_cli_action(
        &self,
        prepared: PreparedCliActionExecution,
    ) -> Result<(), CliEnvironmentError> {
        let operation_id = prepared.operation_id.clone();
        let started_at = self.ports.clock.now();
        match self.run_action(&prepared) {
            Ok(record) => {
                // The whole context, not just the outcome: a reader deciding whether to retry needs
                // to know the command exited 0 and verification could not confirm it, which one
                // label cannot carry.
                self.ports
                    .operations
                    .complete(&operation_id, encode_operation_record(&record))
            }
            Err(error) => {
                let message = error.to_string();
                let record = CliOperationRecord {
                    elapsed_ms: elapsed_ms(started_at, self.ports.clock.now()),
                    ..CliOperationRecord::unstarted(
                        operation_id.clone(),
                        CliOperationPhase::Preflight,
                    )
                };
                // Recorded on the failure path too. An operation that never reached a process is
                // still an operation someone will ask about.
                self.ports.diagnostics.record(
                    &operation_id,
                    None,
                    None,
                    &format!("{message} ({})", describe_record(&record)),
                );
                self.ports.operations.fail(&operation_id, message)
            }
        }
    }

    fn run_action(
        &self,
        prepared: &PreparedCliActionExecution,
    ) -> Result<CliOperationRecord, CliEnvironmentError> {
        let operation_id = &prepared.operation_id;
        let started_at = self.ports.clock.now();
        let cancellation = self.ports.operations.cancellation(operation_id)?;
        // Nothing has been applied yet, so everything up to the adapter is still cancellable.
        self.ports
            .operations
            .report_phase(operation_id, CliOperationPhase::Preflight, true)?;
        let fingerprint = self.ports.discovery.environment_fingerprint()?;
        let now = self.ports.clock.now();

        // Single-use admission, atomic in the repository. Expired, consumed, superseded, and stale
        // plans are all refused here -- before any external effect.
        let plan = self.ports.repository.begin_action_plan_execution(
            &prepared.plan_id,
            prepared.expected_revision,
            &fingerprint,
            now,
        )?;

        let definition =
            crate::contexts::tooling::cli::domain::registry::definition(plan.agent_id.as_str())
                .ok_or_else(|| CliEnvironmentError::UnknownTool {
                    agent_id: plan.agent_id.as_str().to_string(),
                })?;
        let distribution = definition
            .distribution(plan.source_id.as_str())
            .ok_or_else(|| CliEnvironmentError::UnsupportedSource {
                agent_id: plan.agent_id.as_str().to_string(),
                source_id: plan.source_id.as_str().to_string(),
            })?;
        self.ports.operations.report_phase(
            operation_id,
            CliOperationPhase::ResolvingSource,
            true,
        )?;
        // Resolved from the plan's recorded source id. There is no other source this can reach.
        let adapter = self.ports.sources.adapter(&plan.source_id).ok_or_else(|| {
            CliEnvironmentError::SourceUnavailable {
                source_id: plan.source_id.as_str().to_string(),
            }
        })?;

        let lease = self
            .ports
            .coordinator
            .try_reserve(&plan.agent_id, &adapter.mutation_key(&plan.agent_id))?
            .ok_or_else(|| {
                self.ports.diagnostics.record(
                    operation_id,
                    Some(&plan.agent_id),
                    Some(plan.action),
                    &format!(
                        "mutation capacity is {}; the reservation could not be taken",
                        self.ports.coordinator.global_capacity()
                    ),
                );
                CliEnvironmentError::OperationConflict {
                    agent_id: plan.agent_id.as_str().to_string(),
                }
            })?;

        let spec = adapter.build_execution(&plan, distribution)?;
        // Recorded before the external effect so a stuck mutation can be attributed to the tool
        // and the resource it holds, not just to an operation id.
        self.ports.diagnostics.record(
            operation_id,
            Some(lease.agent_id()),
            Some(plan.action),
            &format!("holding mutation key {}", lease.mutation_key().as_str()),
        );
        let sink = OperationSink {
            service: self,
            operation_id: operation_id.clone(),
        };
        // The adapter announces `downloading` and `mutating` itself: only it knows where the
        // download ends and the irreversible part begins.
        let phases = OperationPhases {
            service: self,
            operation_id: operation_id.clone(),
        };
        let process = adapter.execute(spec, &cancellation, &sink, &phases);
        // Exactly once on this path; `Drop` covers every other one, including a panic between here
        // and the end of the function.
        lease.release();

        // Post-mutation detection runs whatever happened. The process may have changed the machine
        // even when it reported failure, and the snapshot must describe what is actually there.
        self.ports.operations.report_phase(
            operation_id,
            CliOperationPhase::RefreshingEnvironment,
            false,
        )?;
        let verified = self.verify_and_persist(
            &plan,
            &process,
            operation_id,
            &fingerprint,
            &adapter.mutation_key(&plan.agent_id),
        )?;

        self.ports.repository.finish_action_plan(
            &plan.id,
            match verified.outcome {
                CliMutationOutcome::Verified | CliMutationOutcome::AppliedUnverified => {
                    CliActionPlanState::Completed
                }
                CliMutationOutcome::Cancelled => CliActionPlanState::Cancelled,
                _ => CliActionPlanState::Failed,
            },
            self.ports.clock.now(),
        )?;

        Ok(CliOperationRecord {
            operation_id: operation_id.clone(),
            agent_id: Some(plan.agent_id.clone()),
            source_id: Some(plan.source_id.clone()),
            action: Some(plan.action),
            target_version: plan
                .target_version
                .as_deref()
                .map(NormalizedCliVersion::parse),
            observed_version: verified
                .observed_version
                .as_deref()
                .map(NormalizedCliVersion::parse),
            phase: CliOperationPhase::Completed,
            termination: termination_of(&process),
            elapsed_ms: elapsed_ms(started_at, self.ports.clock.now()),
            outcome: Some(verified.outcome),
            warnings: verified.warnings,
            output_truncated: process
                .as_ref()
                .map(|outcome| outcome.truncated)
                .unwrap_or(false),
        })
    }

    /// Detects the post-operation machine state and classifies the outcome.
    ///
    /// The snapshot saved here is the *observed* one. There is no branch that writes the
    /// pre-operation snapshot back, which is the regression this replaces: the old code saved
    /// `status.clone()` on any error, including the case where the package command had already
    /// succeeded and only verification failed.
    fn verify_and_persist(
        &self,
        plan: &CliActionPlan,
        process: &Result<super::environment_ports::CliProcessOutcome, CliEnvironmentError>,
        operation_id: &str,
        fingerprint: &str,
        mutation_key: &crate::contexts::tooling::cli::domain::source::CliMutationKey,
    ) -> Result<VerifiedMutation, CliEnvironmentError> {
        let definition =
            crate::contexts::tooling::cli::domain::registry::definition(plan.agent_id.as_str())
                .ok_or_else(|| CliEnvironmentError::UnknownTool {
                    agent_id: plan.agent_id.as_str().to_string(),
                })?;
        // Deliberately not the operation's own cancellation. See `CliCancellation::uncancelled`:
        // the user cancelled the mutation, not the observation of what it already did.
        let cancellation = CliCancellation::uncancelled();
        let before = self.snapshot_or_never_scanned(&plan.agent_id, fingerprint)?;
        let before_version = active_version_string(&before);
        let mut warnings = Vec::new();

        // Best-effort *when safe*. Another operation may still hold this package manager's
        // resource, and probing a tree it is halfway through writing reports a transient state as
        // the machine's state -- which is worse than admitting the answer is not yet known.
        let detected = if self.ports.coordinator.may_detect_now(mutation_key) {
            self.refresh_one_for_verification(
                &plan.agent_id,
                definition,
                fingerprint,
                operation_id,
                &cancellation,
            )
        } else {
            self.ports.diagnostics.record(
                operation_id,
                Some(&plan.agent_id),
                Some(plan.action),
                &format!(
                    "skipped post-mutation detection while {} is being written",
                    mutation_key.as_str()
                ),
            );
            warnings.push(CliVerificationWarning::DetectionSkippedWhileBusy);
            Err(CliEnvironmentError::OperationConflict {
                agent_id: plan.agent_id.as_str().to_string(),
            })
        };

        let (mut snapshot, detection_failed) = match detected {
            Ok(snapshot) => (snapshot, false),
            Err(error) => {
                if warnings.is_empty() {
                    self.ports.diagnostics.record(
                        operation_id,
                        Some(&plan.agent_id),
                        Some(plan.action),
                        &error.to_string(),
                    );
                    warnings.push(CliVerificationWarning::DetectionFailed);
                }
                // Detection did not produce an answer. What is held describes the machine *before*
                // the command, so it is kept as last-known and labelled stale -- never presented as
                // the current state.
                let mut stale = before.clone();
                stale.mark_stale();
                (stale, true)
            }
        };

        let after_version = active_version_string(&snapshot);
        // Only positive evidence counts as a change. A version that could not be re-read is not a
        // version that vanished -- treating "not observed" as "changed" is the same silence-as-
        // consent mistake the readiness probes exist to avoid, and here it would report a machine
        // as modified on the strength of a probe that never ran.
        let machine_changed = match (&before_version, &after_version) {
            (Some(before_version), Some(after_version)) => before_version != after_version,
            _ => !detection_failed && before.installations.len() != snapshot.installations.len(),
        };

        let outcome = match process {
            Ok(process) if process.cancelled => {
                if machine_changed {
                    CliMutationOutcome::ChangedButFailed
                } else {
                    CliMutationOutcome::Cancelled
                }
            }
            Ok(process) if process.succeeded() => {
                let reached_target = plan
                    .target_version
                    .as_deref()
                    .is_none_or(|target| after_version.as_deref() == Some(target));
                if !reached_target && !detection_failed {
                    warnings.push(CliVerificationWarning::TargetVersionNotObserved);
                }
                if detection_failed || !reached_target {
                    // The command completed. Verification did not confirm it. This is not a
                    // failure that can be undone by restoring an older row.
                    CliMutationOutcome::AppliedUnverified
                } else {
                    CliMutationOutcome::Verified
                }
            }
            // Failed or errored, but detection shows the machine moved anyway.
            _ if machine_changed => CliMutationOutcome::ChangedButFailed,
            _ => CliMutationOutcome::NoChangeFailed,
        };

        snapshot.record_mutation(CliMutationSummary {
            outcome,
            source_id: plan.source_id.clone(),
            action: plan.action.as_str().to_string(),
            target_version: plan.target_version.clone(),
            operation_id: operation_id.to_string(),
            completed_at: self.ports.clock.now(),
        });
        self.ports.repository.save_snapshot_atomic(&snapshot)?;
        Ok(VerifiedMutation {
            outcome,
            observed_version: after_version,
            warnings,
        })
    }

    fn refresh_one_for_verification(
        &self,
        agent_id: &CliToolId,
        definition: &'static CliToolDefinition,
        fingerprint: &str,
        operation_id: &str,
        cancellation: &CliCancellation,
    ) -> Result<CliEnvironmentSnapshot, CliEnvironmentError> {
        self.ports.operations.report_phase(
            operation_id,
            CliOperationPhase::VerifyingExecutable,
            false,
        )?;
        self.refresh_one(
            agent_id,
            definition,
            fingerprint,
            false,
            operation_id,
            cancellation,
        )
    }
}

fn version_mode(
    distribution: &CliDistributionDefinition,
    action: CliActionKind,
    platform: CliPlatform,
) -> CliTargetVersionMode {
    match action {
        CliActionKind::Install => {
            distribution.target_mode_on(CliDistributionAction::Install, platform)
        }
        CliActionKind::Upgrade => {
            distribution.target_mode_on(CliDistributionAction::Upgrade, platform)
        }
        CliActionKind::Downgrade => {
            distribution.target_mode_on(CliDistributionAction::Downgrade, platform)
        }
        CliActionKind::Reinstall => {
            distribution.target_mode_on(CliDistributionAction::Reinstall, platform)
        }
        CliActionKind::Uninstall => {
            if distribution.capabilities.uninstall {
                CliTargetVersionMode::LatestOnly
            } else {
                CliTargetVersionMode::Unsupported
            }
        }
        CliActionKind::Repair => {
            if distribution.capabilities.repair.needs_preflight() {
                CliTargetVersionMode::LatestOnly
            } else {
                CliTargetVersionMode::Unsupported
            }
        }
    }
}

fn preconditions_for(source_id: &CliSourceId, requires_elevation: &bool) -> Vec<CliPrecondition> {
    let mut preconditions = vec![CliPrecondition::SourceExecutableAvailable {
        source: source_id.as_str().to_string(),
    }];
    if *requires_elevation {
        preconditions.push(CliPrecondition::ElevatedPrivileges);
    }
    preconditions
}

/// What post-mutation verification established, beyond the outcome label.
struct VerifiedMutation {
    outcome: CliMutationOutcome,
    observed_version: Option<String>,
    warnings: Vec<CliVerificationWarning>,
}

/// Maps a process result onto how the operation ended.
///
/// An adapter error is `NotStarted` rather than a failed exit: nothing established that a process
/// ran at all, and inventing an exit code for it would put a number in the record that no process
/// ever reported.
fn termination_of(
    process: &Result<super::environment_ports::CliProcessOutcome, CliEnvironmentError>,
) -> CliOperationTermination {
    match process {
        Err(_) => CliOperationTermination::NotStarted,
        // Checked before the exit code: a cancelled process can still exit 0, and reporting that
        // as a clean exit is how a partial change gets recorded as a completed one.
        Ok(outcome) if outcome.cancelled => CliOperationTermination::Cancelled,
        Ok(outcome) if outcome.timed_out => CliOperationTermination::TimedOut,
        Ok(outcome) => match outcome.exit_code {
            Some(code) => CliOperationTermination::Exited { code },
            None => CliOperationTermination::ExitedWithoutCode,
        },
    }
}

/// Wall-clock duration, floored at zero.
///
/// A clock that moves backwards -- an NTP correction mid-operation -- yields zero rather than a
/// wrapped value that would read as a multi-century operation.
fn elapsed_ms(started_at: DateTime<Utc>, finished_at: DateTime<Utc>) -> u64 {
    u64::try_from((finished_at - started_at).num_milliseconds().max(0)).unwrap_or(0)
}

/// The record as the operation store, the frontend DTO, and the log all see it.
///
/// One encoder for all three, so the three boundaries cannot drift into showing different fields.
fn encode_operation_record(record: &CliOperationRecord) -> serde_json::Value {
    serde_json::json!({
        "operationId": record.operation_id,
        "agentId": record.agent_id.as_ref().map(|id| id.as_str()),
        "sourceId": record.source_id.as_ref().map(|id| id.as_str()),
        "action": record.action.map(|action| action.as_str()),
        "targetVersion": record.target_version.as_ref().map(|v| v.as_str()),
        "observedVersion": record.observed_version.as_ref().map(|v| v.as_str()),
        "phase": record.phase.as_str(),
        "termination": record.termination.as_str(),
        "exitCode": record.termination.exit_code(),
        "elapsedMs": record.elapsed_ms,
        "outcome": record.outcome.map(CliMutationOutcome::as_str),
        "warnings": record
            .warnings
            .iter()
            .map(|warning| warning.as_str())
            .collect::<Vec<_>>(),
        "outputTruncated": record.output_truncated,
        "warning": record.warrants_attention(),
    })
}

/// A one-line form of the record for the diagnostic log. Identifiers and enums only.
fn describe_record(record: &CliOperationRecord) -> String {
    format!(
        "phase={} termination={} elapsedMs={}",
        record.phase.as_str(),
        record.termination.as_str(),
        record.elapsed_ms
    )
}

fn active_version_string(snapshot: &CliEnvironmentSnapshot) -> Option<String> {
    snapshot
        .recommended_installation()
        .and_then(|installation| installation.reported_version.as_ref())
        .map(|version| version.as_str().to_string())
}

#[cfg(test)]
#[path = "environment_planning_tests.rs"]
mod tests;
