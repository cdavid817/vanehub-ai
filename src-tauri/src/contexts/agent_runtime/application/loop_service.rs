use super::{
    assessment_digest, AgentClockPort, AgentLogLevel, AgentRegistryRepository,
    AgentRuntimeApplicationError, ApiAgentGateway, CanonicalLoopSignal, LoopAdmissionView,
    LoopAssessmentService, LoopAuditConsumption, LoopAuditReceipt, LoopControlAction,
    LoopControlEnvelope, LoopControlOperationClaim, LoopDefinitionView, LoopExecutionAssessment,
    LoopOperationContext, LoopOperationKind, LoopOperationObserver, LoopProjectPort,
    LoopReadinessCheckView, LoopReadinessReportView, LoopRepository, LoopRunView,
    PrepareLoopAdmissionRequest, SaveLoopDefinitionRequest, StartLoopResultView,
    AUDIT_RECEIPT_TTL_SECONDS,
};
use crate::contexts::agent_runtime::domain::{
    InteractionMode, LoopDefinition, LoopDefinitionInput, LoopReadinessCategory,
    LoopReadinessCheckCode, LoopRequestedMode, LoopRun, LoopRunStatus, LoopScopeState,
    LoopTerminalReason,
};
use std::sync::Arc;
use uuid::Uuid;

#[derive(Clone)]
pub(crate) struct LoopApplicationPorts {
    pub(crate) loops: Arc<dyn LoopRepository>,
    pub(crate) registry: Arc<dyn AgentRegistryRepository>,
    pub(crate) api_agents: Arc<dyn ApiAgentGateway>,
    pub(crate) projects: Arc<dyn LoopProjectPort>,
    pub(crate) observer: LoopOperationObserver,
    pub(crate) clock: Arc<dyn AgentClockPort>,
    pub(crate) assessment: LoopAssessmentService,
    /// A per-process token. Receipts issued by a previous process never authorize this one.
    pub(crate) app_epoch: String,
}

#[derive(Clone)]
pub(crate) struct LoopApplicationService {
    ports: LoopApplicationPorts,
}

impl LoopApplicationService {
    pub(crate) fn new(ports: LoopApplicationPorts) -> Self {
        Self { ports }
    }

    pub(crate) fn list_definitions(
        &self,
    ) -> Result<Vec<LoopDefinitionView>, AgentRuntimeApplicationError> {
        self.ports
            .loops
            .list_definitions()
            .map(|definitions| definitions.iter().map(LoopDefinitionView::from).collect())
    }

    pub(crate) fn create_definition(
        &self,
        mut request: SaveLoopDefinitionRequest,
    ) -> Result<LoopDefinitionView, AgentRuntimeApplicationError> {
        request.project_path = self.validate_definition_environment(&request)?;
        let now = self.ports.clock.now();
        let definition = LoopDefinition::new(definition_input(
            format!("loop-{}", Uuid::new_v4()),
            request,
            1,
            now.clone(),
            now,
        ))?;
        self.ports.loops.create_definition(&definition)?;
        Ok(LoopDefinitionView::from(&definition))
    }

    pub(crate) fn update_definition(
        &self,
        definition_id: &str,
        mut request: SaveLoopDefinitionRequest,
    ) -> Result<LoopDefinitionView, AgentRuntimeApplicationError> {
        let current = self
            .ports
            .loops
            .find_definition(definition_id)?
            .ok_or_else(|| loop_validation("Loop definition not found."))?;
        let expected_version = request.expected_version.unwrap_or(current.values().version);
        if expected_version != current.values().version {
            return Err(loop_validation(
                "Loop definition was updated by another operation.",
            ));
        }
        request.project_path = self.validate_definition_environment(&request)?;
        let definition = LoopDefinition::new(definition_input(
            current.values().id.clone(),
            request,
            expected_version.saturating_add(1),
            current.values().created_at.clone(),
            self.ports.clock.now(),
        ))?;
        self.ports
            .loops
            .update_definition(&definition, expected_version)?;
        Ok(LoopDefinitionView::from(&definition))
    }

    pub(crate) fn delete_definition(
        &self,
        definition_id: &str,
    ) -> Result<(), AgentRuntimeApplicationError> {
        if self.ports.loops.has_active_run(definition_id)? {
            return Err(loop_validation(
                "Cannot delete a Loop definition with an active run.",
            ));
        }
        self.ports.loops.delete_definition(definition_id)
    }

    pub(crate) fn list_runs(
        &self,
        definition_id: Option<&str>,
    ) -> Result<Vec<LoopRunView>, AgentRuntimeApplicationError> {
        self.ports.loops.list_run_views(definition_id)
    }

    pub(crate) fn get_run(
        &self,
        run_id: &str,
    ) -> Result<LoopRunView, AgentRuntimeApplicationError> {
        self.ports
            .loops
            .find_run_view(run_id)?
            .ok_or_else(|| loop_validation("Loop run not found."))
    }

    /// The non-launching preflight. Every check here is the same predicate authoritative start
    /// re-runs against freshly loaded state; readiness never creates a run, worktree or session.
    pub(crate) fn readiness(
        &self,
        definition_id: &str,
    ) -> Result<LoopReadinessReportView, AgentRuntimeApplicationError> {
        let definition = self
            .ports
            .loops
            .find_definition(definition_id)?
            .ok_or_else(|| loop_validation("Loop definition not found."))?;
        let values = definition.values();
        let project = self
            .ports
            .projects
            .validate_local_git_project(&values.project_path);
        let project_ready = project.is_ok();
        let branch = project.as_ref().ok().map_or(Ok(false), |canonical| {
            self.ports
                .projects
                .base_branch_available(canonical, &values.base_branch)
        });
        let worker = self.validate_agent(&values.worker_agent_id);
        let verifier = self.validate_agent(&values.verifier_agent_id);
        let commands_ready = !values.verification_commands.is_empty()
            && values.verification_commands.iter().all(|command| {
                !command.program().trim().is_empty() && command.timeout_seconds() > 0
            });
        let scope = definition.scope();
        let scope_supported = definition.scope_state() == LoopScopeState::Verified;
        let assessment = self.ports.assessment.assess(&definition)?;
        let no_active_run = !self.ports.loops.has_active_run(definition_id)?;
        let checks = vec![
            readiness_check(
                LoopReadinessCheckCode::DefinitionEnabled,
                LoopReadinessCategory::Definition,
                values.enabled,
                None,
                "definition",
            ),
            readiness_check(
                LoopReadinessCheckCode::ProjectAvailable,
                LoopReadinessCategory::Workspace,
                project_ready,
                project.err().map(|error| error.to_string()),
                "project",
            ),
            readiness_check(
                LoopReadinessCheckCode::BranchAvailable,
                LoopReadinessCategory::Workspace,
                branch.as_ref().copied().unwrap_or(false),
                branch.err().map(|error| error.to_string()),
                "branch",
            ),
            readiness_check(
                LoopReadinessCheckCode::WorkerEligible,
                LoopReadinessCategory::Agent,
                worker.is_ok(),
                worker.err().map(|error| error.to_string()),
                "worker",
            ),
            readiness_check(
                LoopReadinessCheckCode::VerifierEligible,
                LoopReadinessCategory::Agent,
                verifier.is_ok(),
                verifier.err().map(|error| error.to_string()),
                "verifier",
            ),
            readiness_check(
                LoopReadinessCheckCode::VerificationValid,
                LoopReadinessCategory::Verification,
                commands_ready,
                None,
                "verification",
            ),
            readiness_check(
                LoopReadinessCheckCode::ScopeVersionSupported,
                LoopReadinessCategory::Definition,
                scope_supported,
                (!scope_supported).then(|| {
                    "This definition predates scope enforcement; save it with explicit allowed paths and a requested mode.".to_string()
                }),
                "definition",
            ),
            readiness_check(
                LoopReadinessCheckCode::PathScopeValid,
                LoopReadinessCategory::Verification,
                scope_supported && scope.is_ok(),
                scope.as_ref().err().map(|error| error.to_string()),
                "definition",
            ),
            readiness_check(
                LoopReadinessCheckCode::ExecutionCoverage,
                LoopReadinessCategory::Agent,
                assessment.satisfies_requested_mode,
                (!assessment.satisfies_requested_mode)
                    .then(|| assessment.blockers.join(" ")),
                "worker",
            ),
            readiness_check(
                LoopReadinessCheckCode::NoActiveRun,
                LoopReadinessCategory::Runtime,
                no_active_run,
                None,
                "runs",
            ),
        ];
        let report = LoopReadinessReportView {
            definition_id: definition_id.to_string(),
            ready: checks.iter().all(|check| check.passed),
            checks,
            checked_at: self.ports.clock.now(),
            requested_mode: definition.requested_mode(),
            scope_state: definition.scope_state(),
            acknowledgement_required: assessment.acknowledgement_required,
            assessment: Some(assessment),
            definition_revision: values.version,
        };
        if !report.ready {
            let blocked = report
                .checks
                .iter()
                .filter(|check| !check.passed)
                .map(|check| check.code.as_str())
                .collect::<Vec<_>>()
                .join(",");
            let _ = self.ports.observer.record(
                &LoopOperationContext {
                    run_id: definition_id.to_string(),
                    iteration_id: None,
                    kind: LoopOperationKind::Readiness,
                },
                None,
                AgentLogLevel::Warn,
                &format!("Loop readiness blocked: {blocked}"),
            );
        }
        Ok(report)
    }

    /// Issues the facts a human must see before an artifact-audited transition, plus a short
    /// lived challenge when one can be acknowledged. No run, worktree or session is created.
    pub(crate) fn prepare_admission(
        &self,
        request: PrepareLoopAdmissionRequest,
    ) -> Result<LoopAdmissionView, AgentRuntimeApplicationError> {
        let (target_id, definition, revision, scope_digest) = match request.action {
            LoopControlAction::Start => {
                let definition_id = request
                    .definition_id
                    .clone()
                    .ok_or_else(|| loop_validation("A definition id is required to start."))?;
                let definition = self
                    .ports
                    .loops
                    .find_definition(&definition_id)?
                    .ok_or_else(|| loop_validation("Loop definition not found."))?;
                let scope = definition
                    .scope()
                    .map_err(|error| loop_validation(&error.to_string()))?;
                let mode = definition
                    .requested_mode()
                    .ok_or_else(|| loop_validation("Loop definition has no requested mode."))?;
                let version = definition.values().version;
                (definition_id, definition, version, scope.digest(mode))
            }
            LoopControlAction::Resume | LoopControlAction::Continue => {
                let run_id = request
                    .run_id
                    .clone()
                    .ok_or_else(|| loop_validation("A run id is required."))?;
                let snapshot = self
                    .ports
                    .loops
                    .find_run_definition_snapshot(&run_id)?
                    .ok_or_else(|| loop_validation("Loop run not found."))?;
                let record = self
                    .ports
                    .loops
                    .find_run_scope(&run_id)?
                    .ok_or_else(|| loop_validation("Loop run not found."))?;
                let binding = record.binding.ok_or_else(|| {
                    loop_validation(
                        "This run has no trustworthy scope binding; cancel it and start a new run from the confirmed definition.",
                    )
                })?;
                (run_id, snapshot, record.revision, binding.scope_digest)
            }
        };
        if request.expected_revision != revision {
            return Err(loop_validation(
                "The Loop changed since it was loaded; refresh and review the current state.",
            ));
        }
        let assessment = self.ports.assessment.assess(&definition)?;
        let mode = definition
            .requested_mode()
            .unwrap_or(LoopRequestedMode::PreventiveRequired);
        let can_acknowledge =
            mode == LoopRequestedMode::ArtifactAudited && assessment.satisfies_requested_mode;
        let (challenge_id, expires_at) = if can_acknowledge {
            let now = self.ports.clock.now();
            let receipt = LoopAuditReceipt {
                id: format!("loop-audit-{}", Uuid::new_v4()),
                challenge_id: format!("loop-challenge-{}", Uuid::new_v4()),
                action: request.action,
                target_id: target_id.clone(),
                expected_revision: revision,
                scope_digest: scope_digest.clone(),
                assessment_digest: assessment_digest(&assessment),
                client_context: bounded(&request.client_context, 256),
                app_epoch: self.ports.app_epoch.clone(),
                created_at: now.clone(),
                expires_at: add_seconds(&now, AUDIT_RECEIPT_TTL_SECONDS)?,
                acknowledged_at: None,
                consumed_at: None,
            };
            self.ports.loops.create_audit_challenge(&receipt)?;
            (Some(receipt.challenge_id), Some(receipt.expires_at))
        } else {
            (None, None)
        };
        let values = definition.values();
        Ok(LoopAdmissionView {
            action: request.action,
            target_id,
            expected_revision: revision,
            requested_mode: mode,
            scope_digest,
            allowed_paths: values.allowed_paths.clone(),
            protected_paths: values.protected_paths.clone(),
            acknowledgement_required: assessment.acknowledgement_required,
            assessment,
            challenge_id,
            expires_at,
        })
    }

    /// Turns a challenge into a receipt. The receipt is a record of a human confirmation and
    /// nothing more: it grants no tool permission and is consumed by exactly one transition.
    pub(crate) fn acknowledge_audit(
        &self,
        challenge_id: &str,
    ) -> Result<LoopAuditReceipt, AgentRuntimeApplicationError> {
        let now = self.ports.clock.now();
        let receipt = self
            .ports
            .loops
            .acknowledge_audit_challenge(challenge_id, &now)?;
        if receipt.app_epoch != self.ports.app_epoch || receipt.expires_at <= now {
            return Err(loop_validation(
                "The audit challenge expired; refresh the assessment and confirm again.",
            ));
        }
        Ok(receipt)
    }

    /// Loads a receipt and checks every binding it carries against the transition about to
    /// commit. The repository consumes it atomically with that transition.
    pub(crate) fn audit_consumption(
        &self,
        envelope: &LoopControlEnvelope,
        action: LoopControlAction,
        target_id: &str,
        expected_revision: u64,
        scope_digest: &str,
        assessment: &LoopExecutionAssessment,
    ) -> Result<LoopAuditConsumption, AgentRuntimeApplicationError> {
        let receipt_id = envelope.audit_acknowledgement_id.as_deref().ok_or_else(|| {
            loop_validation(
                "artifact-audited execution requires a fresh explicit acknowledgement for this exact action.",
            )
        })?;
        let receipt = self
            .ports
            .loops
            .find_audit_receipt(receipt_id)?
            .ok_or_else(|| loop_validation("The audit acknowledgement is unknown."))?;
        let now = self.ports.clock.now();
        let digest = assessment_digest(assessment);
        let matches = receipt.acknowledged_at.is_some()
            && receipt.consumed_at.is_none()
            && receipt.action == action
            && receipt.target_id == target_id
            && receipt.expected_revision == expected_revision
            && receipt.scope_digest == scope_digest
            && receipt.assessment_digest == digest
            && receipt.app_epoch == self.ports.app_epoch
            && receipt.expires_at > now
            && receipt.created_at <= now;
        if !matches {
            return Err(loop_validation(
                "The audit acknowledgement does not match this action, revision, scope, assessment or is no longer valid; confirm again.",
            ));
        }
        Ok(LoopAuditConsumption {
            receipt_id: receipt.id,
            action,
            target_id: target_id.to_string(),
            expected_revision,
            scope_digest: scope_digest.to_string(),
            assessment_digest: digest,
            app_epoch: self.ports.app_epoch.clone(),
            now,
        })
    }

    pub(crate) fn assessment(&self) -> &LoopAssessmentService {
        &self.ports.assessment
    }

    pub(crate) fn start_manual(
        &self,
        definition_id: &str,
        envelope: LoopControlEnvelope,
    ) -> Result<StartLoopResultView, AgentRuntimeApplicationError> {
        let claim = match envelope.idempotency_key.as_deref() {
            Some(key) => Some((
                key.to_string(),
                self.ports.loops.claim_control_operation(
                    key,
                    LoopControlAction::Start.as_str(),
                    definition_id,
                    &self.ports.clock.now(),
                )?,
            )),
            None => None,
        };
        if let Some((_, LoopControlOperationClaim::Existing(outcome))) = &claim {
            return decode_start_outcome(outcome);
        }
        if let Some((_, LoopControlOperationClaim::InFlight)) = &claim {
            return Err(loop_validation(
                "This start request is already being processed.",
            ));
        }
        let key = claim.map(|(key, _)| key);
        let result = self.start_manual_inner(definition_id, &envelope);
        match (&key, &result) {
            (Some(key), Ok(result)) => {
                self.ports.loops.record_control_operation(
                    key,
                    &serde_json::json!({
                        "runId": result.run_id,
                        "operationId": result.operation_id,
                    })
                    .to_string(),
                )?;
            }
            (Some(key), Err(_)) => {
                let _ = self.ports.loops.release_control_operation(key);
            }
            (None, _) => {}
        }
        result
    }

    fn start_manual_inner(
        &self,
        definition_id: &str,
        envelope: &LoopControlEnvelope,
    ) -> Result<StartLoopResultView, AgentRuntimeApplicationError> {
        let definition = self
            .ports
            .loops
            .find_definition(definition_id)?
            .ok_or_else(|| loop_validation("Loop definition not found."))?;
        self.validate_start(&definition)?;
        if let Some(expected) = envelope.expected_revision {
            if expected != definition.values().version {
                return Err(loop_validation(
                    "The Loop definition changed since readiness was checked; review it again.",
                ));
            }
        }
        if definition.scope_state() != LoopScopeState::Verified {
            return Err(loop_validation(
                "This definition predates scope enforcement; save it with explicit allowed paths and a requested mode before starting.",
            ));
        }
        let scope = definition
            .scope()
            .map_err(|error| loop_validation(&error.to_string()))?;
        let mode = definition
            .requested_mode()
            .ok_or_else(|| loop_validation("Loop definition has no requested mode."))?;
        let assessment = self.ports.assessment.assess(&definition)?;
        if !assessment.satisfies_requested_mode {
            return Err(loop_validation(&format!(
                "Execution coverage does not satisfy {}: {}",
                mode.as_str(),
                assessment.blockers.join("; ")
            )));
        }
        let scope_digest = scope.digest(mode);
        let audit = if mode == LoopRequestedMode::ArtifactAudited {
            Some(self.audit_consumption(
                envelope,
                LoopControlAction::Start,
                definition_id,
                definition.values().version,
                &scope_digest,
                &assessment,
            )?)
        } else {
            None
        };

        if self.ports.loops.has_active_run(definition_id)? {
            return Err(loop_validation(
                "This Loop definition already has an active run.",
            ));
        }

        let now = self.ports.clock.now();
        let mut run = LoopRun::new(
            format!("loop-run-{}", Uuid::new_v4()),
            definition_id.to_string(),
        )?;
        self.ports.loops.create_run_with_scope(
            &run,
            &definition,
            &definition.values().project_path,
            &now,
            &assessment,
            audit.as_ref(),
        )?;
        if let Err(error) = self
            .ports
            .observer
            .start_canonical_loop(run.id(), definition_id)
        {
            self.fail_queued_run(&mut run, &now);
            return Err(error);
        }

        let context = LoopOperationContext {
            run_id: run.id().to_string(),
            iteration_id: None,
            kind: LoopOperationKind::Worktree,
        };
        let operation = match self
            .ports
            .observer
            .start(context, "Preparing isolated worktree")
        {
            Ok(operation) => operation,
            Err(error) => {
                self.fail_queued_run(&mut run, &now);
                return Err(error);
            }
        };

        if let Err(error) = self.ports.loops.attach_run_operation(
            run.id(),
            &operation.id,
            LoopRunStatus::Queued,
            &now,
        ) {
            let _ = self.ports.observer.fail(
                &operation,
                "Loop run changed before preparation could be associated.",
            );
            self.fail_queued_run(&mut run, &now);
            return Err(error);
        }

        Ok(StartLoopResultView {
            run_id: run.id().to_string(),
            operation_id: operation.id,
        })
    }

    fn validate_start(
        &self,
        definition: &LoopDefinition,
    ) -> Result<(), AgentRuntimeApplicationError> {
        let values = definition.values();
        if !values.enabled {
            return Err(loop_validation("Loop definition is disabled."));
        }
        self.ports
            .projects
            .validate_local_git_project(&values.project_path)?;
        self.validate_agent(&values.worker_agent_id)?;
        self.validate_agent(&values.verifier_agent_id)
    }

    fn validate_definition_environment(
        &self,
        request: &SaveLoopDefinitionRequest,
    ) -> Result<String, AgentRuntimeApplicationError> {
        let canonical = self
            .ports
            .projects
            .validate_local_git_project(&request.project_path)?;
        self.validate_agent(&request.worker_agent_id)?;
        self.validate_agent(&request.verifier_agent_id)?;
        Ok(canonical)
    }

    fn validate_agent(&self, agent_id: &str) -> Result<(), AgentRuntimeApplicationError> {
        let agent = self
            .ports
            .registry
            .find(agent_id)?
            .ok_or_else(|| AgentRuntimeApplicationError::AgentNotFound(agent_id.to_string()))?;
        if agent.supports(InteractionMode::Cli) {
            agent.ensure_selectable(InteractionMode::Cli)?;
            return Ok(());
        }
        agent.ensure_selectable(InteractionMode::Api)?;
        let trusted = self
            .ports
            .api_agents
            .provider_config(agent_id)?
            .map(|config| config.auto_approve_tools)
            .unwrap_or(false);
        if !trusted {
            return Err(loop_validation(
                "API agent must have tool-use trust enabled before it can be used as a Loop Worker or Verifier.",
            ));
        }
        Ok(())
    }

    fn fail_queued_run(&self, run: &mut LoopRun, now: &str) {
        if run.fail(LoopTerminalReason::RuntimeError).is_ok() {
            let _ =
                self.ports
                    .loops
                    .save_run_transition(run, LoopRunStatus::Queued, now, Some(now));
        }
        let _ = self
            .ports
            .observer
            .signal_canonical_loop(run.id(), CanonicalLoopSignal::Failed);
    }
}

fn decode_start_outcome(
    outcome: &str,
) -> Result<StartLoopResultView, AgentRuntimeApplicationError> {
    let value: serde_json::Value = serde_json::from_str(outcome)
        .map_err(|_| loop_validation("Recorded start outcome is unreadable."))?;
    match (
        value.get("runId").and_then(serde_json::Value::as_str),
        value.get("operationId").and_then(serde_json::Value::as_str),
    ) {
        (Some(run_id), Some(operation_id)) => Ok(StartLoopResultView {
            run_id: run_id.to_string(),
            operation_id: operation_id.to_string(),
        }),
        _ => Err(loop_validation("Recorded start outcome is incomplete.")),
    }
}

pub(crate) fn add_seconds(
    timestamp: &str,
    seconds: i64,
) -> Result<String, AgentRuntimeApplicationError> {
    let parsed = chrono::DateTime::parse_from_rfc3339(timestamp)
        .map_err(|_| loop_validation("Clock produced an unreadable timestamp."))?;
    let shifted = parsed
        .checked_add_signed(chrono::Duration::seconds(seconds))
        .ok_or_else(|| loop_validation("Clock overflow."))?;
    Ok(shifted
        .with_timezone(&chrono::Utc)
        .to_rfc3339_opts(chrono::SecondsFormat::Millis, true))
}

fn bounded(value: &str, max: usize) -> String {
    value
        .chars()
        .filter(|character| !character.is_control())
        .take(max)
        .collect()
}

fn definition_input(
    id: String,
    request: SaveLoopDefinitionRequest,
    version: u64,
    created_at: String,
    updated_at: String,
) -> LoopDefinitionInput {
    LoopDefinitionInput {
        id,
        name: request.name,
        enabled: request.enabled,
        project_path: request.project_path,
        base_branch: request.base_branch,
        goal: request.goal,
        acceptance_criteria: request.acceptance_criteria,
        allowed_paths: request.allowed_paths,
        protected_paths: request.protected_paths,
        worker_agent_id: request.worker_agent_id,
        verifier_agent_id: request.verifier_agent_id,
        verification_commands: request.verification_commands,
        limits: request.limits,
        version,
        created_at,
        updated_at,
        scope_schema_version: request.scope_schema_version,
        requested_mode: request.requested_mode,
    }
}

pub(crate) fn loop_validation(message: &str) -> AgentRuntimeApplicationError {
    AgentRuntimeApplicationError::Validation(message.to_string())
}

fn readiness_check(
    code: LoopReadinessCheckCode,
    category: LoopReadinessCategory,
    passed: bool,
    detail: Option<String>,
    remediation_target: &'static str,
) -> LoopReadinessCheckView {
    LoopReadinessCheckView {
        code,
        category,
        passed,
        detail,
        remediation_target: (!passed).then_some(remediation_target),
    }
}
