use super::apply_artifact::ApplyArtifactAdapter;
use super::apply_backend::NativeApplyBackend;
use super::native_tool_support::envelope;
use crate::contexts::agent_runtime::application::{
    ChangeSetApplyPort, ChangeSetApplyRecord, ChangeSetStatus, NativeToolErrorCode,
    NativeToolPortRequest, NativeToolResultEnvelope, NativeToolResultStatus,
};
use crate::contexts::agent_runtime::infrastructure::SqliteNativeToolRepository;
use crate::contexts::artifacts::application::ArtifactService;
use crate::contexts::cli_delegation::application::{
    DelegationApplyPreflightRequest, DelegationApplyPreflightService, DelegationApplyRecoveryPort,
    DelegationApplyRecoveryService, DelegationApplyStagingService, DelegationExactApplyService,
    DelegationPostApplyVerificationService, DelegationRecoveryCapsule, DelegationRecoveryOutcome,
};
use chrono::Utc;
use serde_json::{json, Value};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

pub(crate) struct NativeChangeSetApplyAdapter {
    artifacts: Arc<ArtifactService>,
    backend: Arc<NativeApplyBackend>,
    repository: Arc<SqliteNativeToolRepository>,
    lease: Mutex<()>,
}

struct ApplyInput {
    artifact_id: String,
    content_hash: String,
    diff_hash: String,
    repository_identity: String,
    base_commit: String,
}

impl NativeChangeSetApplyAdapter {
    pub(crate) fn new(
        artifacts: Arc<ArtifactService>,
        repository: Arc<SqliteNativeToolRepository>,
        recovery_root: PathBuf,
    ) -> Result<Self, String> {
        Ok(Self {
            artifacts,
            backend: Arc::new(NativeApplyBackend::new(recovery_root, repository.clone())?),
            repository,
            lease: Mutex::new(()),
        })
    }

    fn execute(&self, request: &NativeToolPortRequest) -> Result<Value, &'static str> {
        let input = parse_input(&request.input.value)?;
        if request.context.is_cancelled() || request.context.deadline_reached() {
            return Err("cancelled");
        }
        let target = request
            .context
            .canonical_workspace
            .clone()
            .ok_or("workspace_unavailable")?;
        let _lease = self.lease.lock().map_err(|_| "lease_failure")?;
        let id = request.context.call_id.clone();
        let created_at = Utc::now().to_rfc3339();
        self.save(
            &id,
            &input,
            &request.input.input_hash,
            ChangeSetStatus::Preflighting,
            None,
            &created_at,
        )?;
        let preflight = DelegationApplyPreflightService::new(
            Arc::new(ApplyArtifactAdapter::new(self.artifacts.clone())),
            self.backend.clone(),
            self.backend.clone(),
        )
        .preflight(DelegationApplyPreflightRequest {
            target_root: target,
            artifact_id: input.artifact_id.clone(),
            expected_content_hash: input.content_hash.clone(),
            expected_diff_hash: input.diff_hash.clone(),
            expected_repository_identity: input.repository_identity.clone(),
            expected_base_commit: input.base_commit.clone(),
            approval_input_hash: request.input.input_hash.clone(),
        });
        let plan = match preflight {
            Ok(plan) => plan,
            Err(_) => {
                self.save(
                    &id,
                    &input,
                    &request.input.input_hash,
                    ChangeSetStatus::Failed,
                    Some("preflight_failed"),
                    &created_at,
                )?;
                return Err("preflight_failed");
            }
        };
        let capsule =
            match DelegationApplyStagingService::new(self.backend.clone()).stage(&plan, &id) {
                Ok(capsule) => capsule,
                Err(_) => {
                    self.save(
                        &id,
                        &input,
                        &request.input.input_hash,
                        ChangeSetStatus::Failed,
                        Some("staging_failed"),
                        &created_at,
                    )?;
                    return Err("staging_failed");
                }
            };
        if request.context.is_cancelled() || request.context.deadline_reached() {
            self.backend
                .remove_capsule(&capsule)
                .map_err(|_| "cleanup_failure")?;
            return Err("cancelled");
        }
        self.save(
            &id,
            &input,
            &request.input.input_hash,
            ChangeSetStatus::Applying,
            None,
            &created_at,
        )?;
        let applied = DelegationExactApplyService::new(self.backend.clone()).apply(&plan, &capsule);
        let applied = match applied {
            Ok(applied) => applied,
            Err(_) => {
                return self.recover(
                    &id,
                    &input,
                    &request.input.input_hash,
                    &created_at,
                    &plan,
                    &capsule,
                )
            }
        };
        if request.context.is_cancelled() || request.context.deadline_reached() {
            return self.recover(
                &id,
                &input,
                &request.input.input_hash,
                &created_at,
                &plan,
                &capsule,
            );
        }
        self.save(
            &id,
            &input,
            &request.input.input_hash,
            ChangeSetStatus::Verifying,
            None,
            &created_at,
        )?;
        if DelegationPostApplyVerificationService::new(self.backend.clone())
            .verify_and_complete(&plan, &capsule, &applied)
            .is_err()
        {
            return self.recover(
                &id,
                &input,
                &request.input.input_hash,
                &created_at,
                &plan,
                &capsule,
            );
        }
        let cleanup_pending = self.backend.remove_capsule(&capsule).is_err();
        Ok(
            json!({"applyAttemptId": id, "artifactId": input.artifact_id, "status": "succeeded", "cleanupPending": cleanup_pending}),
        )
    }

    fn recover(
        &self,
        id: &str,
        input: &ApplyInput,
        approval_hash: &str,
        created_at: &str,
        plan: &crate::contexts::cli_delegation::application::DelegationApplyPlan,
        capsule: &DelegationRecoveryCapsule,
    ) -> Result<Value, &'static str> {
        let outcome = DelegationApplyRecoveryService::new(self.backend.clone())
            .recover(plan, capsule)
            .map_err(|_| "recovery_failure")?;
        let status = match outcome {
            DelegationRecoveryOutcome::RolledBack => ChangeSetStatus::RolledBack,
            DelegationRecoveryOutcome::ManualRecoveryRequired => {
                ChangeSetStatus::ManualRecoveryRequired
            }
        };
        self.save(
            id,
            input,
            approval_hash,
            status,
            Some("apply_failed"),
            created_at,
        )?;
        Err(
            if outcome == DelegationRecoveryOutcome::ManualRecoveryRequired {
                "manual_recovery_required"
            } else {
                "apply_rolled_back"
            },
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn save(
        &self,
        id: &str,
        input: &ApplyInput,
        approval_hash: &str,
        status: ChangeSetStatus,
        error: Option<&str>,
        created_at: &str,
    ) -> Result<(), &'static str> {
        let now = Utc::now().to_rfc3339();
        self.repository
            .save_apply_attempt(&ChangeSetApplyRecord {
                contract_version: 1,
                id: id.to_owned(),
                change_set_artifact_id: input.artifact_id.clone(),
                target_repository_identity: input.repository_identity.clone(),
                expected_base_commit: input.base_commit.clone(),
                approval_input_hash: approval_hash.to_owned(),
                status,
                error_code: error.map(str::to_owned),
                consumed_at: (status == ChangeSetStatus::Succeeded).then(|| now.clone()),
                created_at: created_at.to_owned(),
                updated_at: now,
            })
            .map_err(|_| "storage_failure")
    }
}

impl ChangeSetApplyPort for NativeChangeSetApplyAdapter {
    fn execute_change_set_apply(&self, request: NativeToolPortRequest) -> NativeToolResultEnvelope {
        match self.execute(&request) {
            Ok(output) => envelope(NativeToolResultStatus::Succeeded, Some(output), None),
            Err("manual_recovery_required") => envelope(
                NativeToolResultStatus::Failed,
                None,
                Some(NativeToolErrorCode::Conflict),
            ),
            Err("preflight_failed") => envelope(
                NativeToolResultStatus::Denied,
                None,
                Some(NativeToolErrorCode::StaleApproval),
            ),
            Err("cancelled") => envelope(
                NativeToolResultStatus::Cancelled,
                None,
                Some(NativeToolErrorCode::Cancelled),
            ),
            Err(_) => envelope(
                NativeToolResultStatus::Failed,
                None,
                Some(NativeToolErrorCode::ExternalFailure),
            ),
        }
    }
}

fn parse_input(value: &Value) -> Result<ApplyInput, &'static str> {
    let object = value.as_object().ok_or("invalid_input")?;
    let get = |key: &str| {
        object
            .get(key)
            .and_then(Value::as_str)
            .filter(|value| !value.trim().is_empty())
            .map(str::to_owned)
            .ok_or("invalid_input")
    };
    Ok(ApplyInput {
        artifact_id: get("artifact_id")?,
        content_hash: get("content_hash")?,
        diff_hash: get("diff_hash")?,
        repository_identity: get("target_repository_identity")?,
        base_commit: get("base_commit")?,
    })
}
