use super::native_tool_execution::{prepare_attempt, AttemptExecutionIdentity, PreparedOutcome};
use super::native_tool_persistence::{AttemptUpdate, DelegationPersistence};
use super::native_tool_support::{
    envelope, git_head, parse_input, sha256, DelegationInput, REPORT_SCHEMA,
};
use super::{ArtifactChangeSetAdapter, IndependentGitWorkspaceAdapter};
use crate::contexts::agent_runtime::api::{
    CliDelegationPort, DelegationStatus as StoredStatus, NativeToolErrorCode,
    NativeToolPersistencePort, NativeToolPortRequest, NativeToolResultEnvelope,
    NativeToolResultStatus,
};
use crate::contexts::artifacts::application::ArtifactService;
use crate::contexts::cli_delegation::application::{
    DelegationChangeSetLimits, DelegationChangeSetPolicy, DelegationChangeSetSealRequest,
    DelegationChangeSetSealer, DelegationHostEvidence, DelegationMode, DelegationTarget,
    DelegationWorkspace, DelegationWorkspacePort,
};
use crate::contexts::tooling::cli::api::CliApi;
use chrono::Utc;
use serde_json::{json, Value};
use std::path::PathBuf;
use std::sync::Arc;
use uuid::Uuid;

pub(crate) struct ClaudeDelegationNativeToolAdapter {
    cli: CliApi,
    workspaces: IndependentGitWorkspaceAdapter,
    artifacts: Arc<ArtifactService>,
    persistence: DelegationPersistence,
    analyze_ready: bool,
    edit_ready: bool,
}

struct AttemptIdentity {
    delegation_id: String,
    attempt_id: String,
    created_at: String,
    task_hash: String,
}

impl ClaudeDelegationNativeToolAdapter {
    pub(crate) fn new(
        cli: CliApi,
        operations_root: PathBuf,
        artifacts: Arc<ArtifactService>,
        repository: Arc<dyn NativeToolPersistencePort>,
        analyze_ready: bool,
        edit_ready: bool,
    ) -> Self {
        Self {
            cli,
            workspaces: IndependentGitWorkspaceAdapter::new(operations_root),
            artifacts,
            persistence: DelegationPersistence::new(repository),
            analyze_ready,
            edit_ready,
        }
    }

    fn execute(&self, request: &NativeToolPortRequest) -> Result<Value, &'static str> {
        let input = parse_input(&request.input.value)?;
        if !self.mode_ready(input.mode) {
            return Err("target_unavailable");
        }
        let source = request
            .context
            .canonical_workspace
            .as_deref()
            .ok_or("workspace_unavailable")?;
        let identity = AttemptIdentity {
            delegation_id: format!("delegation-{}", Uuid::new_v4()),
            attempt_id: request.context.call_id.clone(),
            created_at: Utc::now().to_rfc3339(),
            task_hash: sha256(input.task.as_bytes()),
        };
        self.save(
            request,
            &input,
            &identity,
            StoredStatus::Running,
            None,
            None,
            None,
            None,
        )?;
        let result = self.execute_attempt(request, source, &input, &identity);
        if let Err(error) = result {
            let status = if request.context.is_cancelled() {
                StoredStatus::Cancelled
            } else {
                StoredStatus::Failed
            };
            self.save(
                request,
                &input,
                &identity,
                status,
                None,
                None,
                None,
                Some(error),
            )?;
        }
        result
    }

    fn mode_ready(&self, mode: DelegationMode) -> bool {
        match mode {
            DelegationMode::Analyze => self.analyze_ready,
            DelegationMode::Edit => self.edit_ready,
        }
    }

    fn execute_attempt(
        &self,
        request: &NativeToolPortRequest,
        source: &std::path::Path,
        input: &DelegationInput,
        identity: &AttemptIdentity,
    ) -> Result<Value, &'static str> {
        let head = git_head(source)?;
        let workspace = self
            .workspaces
            .create(source, &head)
            .map_err(|_| "workspace_failure")?;
        let prepared = prepare_attempt(
            &self.cli,
            &self.artifacts,
            request,
            &workspace,
            input,
            AttemptExecutionIdentity {
                delegation_id: &identity.delegation_id,
                attempt_id: &identity.attempt_id,
                created_at: &identity.created_at,
            },
        );
        let cleanup = self.workspaces.cleanup(&workspace);
        if cleanup.is_err() {
            return Err("cleanup_failure");
        }
        let prepared = prepared?;
        self.finalize(request, &workspace, input, identity, prepared)
    }

    fn finalize(
        &self,
        request: &NativeToolPortRequest,
        workspace: &DelegationWorkspace,
        input: &DelegationInput,
        identity: &AttemptIdentity,
        prepared: PreparedOutcome,
    ) -> Result<Value, &'static str> {
        if input.mode == DelegationMode::Analyze {
            self.save(
                request,
                input,
                identity,
                StoredStatus::Succeeded,
                Some(&prepared.report.summary),
                Some(&prepared.report_artifact.id),
                None,
                None,
            )?;
            return Ok(json!({
                "delegationId": identity.delegation_id,
                "attemptId": identity.attempt_id,
                "reportArtifactId": prepared.report_artifact.id,
                "report": prepared.report,
            }));
        }
        DelegationChangeSetPolicy::validate(
            &prepared.capture,
            DelegationChangeSetLimits::HARD_CEILING,
        )
        .map_err(|_| "changeset_rejected")?;
        let host = DelegationHostEvidence {
            base_commit: workspace.base_commit.clone(),
            changed_files: prepared
                .capture
                .files
                .iter()
                .map(|file| file.path.clone())
                .collect(),
            diff_hash: Some(prepared.capture.diff_hash.clone()),
            exit_code: prepared.exit_code,
            observed_actions: Vec::new(),
            policy_violations: Vec::new(),
            cleanup_succeeded: true,
        };
        let capture_record = prepared.capture.clone();
        let sealed = DelegationChangeSetSealer::new(Arc::new(ArtifactChangeSetAdapter::new(
            self.artifacts.clone(),
        )))
        .seal(DelegationChangeSetSealRequest {
            artifact_identity: format!("changeset-{}", identity.attempt_id),
            delegation_id: identity.delegation_id.clone(),
            attempt_id: identity.attempt_id.clone(),
            repository_identity: workspace.repository_identity.clone(),
            provider: DelegationTarget::ClaudeCode,
            cli_fingerprint: prepared.cli_fingerprint,
            adapter_fingerprint: "claude-v1-no-bash".to_owned(),
            prompt_schema_fingerprint: sha256(REPORT_SCHEMA.as_bytes()),
            capture: prepared.capture,
            provider_report: prepared.report.clone(),
            host_evidence: host,
            risk_classification: "delegated_edit".to_owned(),
            limitations: vec!["Provider claims are untrusted until host verification.".to_owned()],
            created_at: identity.created_at.clone(),
        })
        .map_err(|_| "sealing_failure")?;
        self.persistence.insert_change_set(
            &sealed.artifact_id,
            &sealed.content_hash,
            &workspace.repository_identity,
            &identity.attempt_id,
            &capture_record,
            &identity.created_at,
        )?;
        self.save(
            request,
            input,
            identity,
            StoredStatus::Succeeded,
            Some(&prepared.report.summary),
            Some(&prepared.report_artifact.id),
            Some(&sealed.artifact_id),
            None,
        )?;
        Ok(json!({
            "delegationId": identity.delegation_id,
            "attemptId": identity.attempt_id,
            "reportArtifactId": prepared.report_artifact.id,
            "changeSetArtifactId": sealed.artifact_id,
            "contentHash": sealed.content_hash,
            "diffHash": sealed.diff_hash,
            "report": prepared.report,
        }))
    }

    #[allow(clippy::too_many_arguments)]
    fn save(
        &self,
        request: &NativeToolPortRequest,
        input: &DelegationInput,
        identity: &AttemptIdentity,
        status: StoredStatus,
        summary: Option<&str>,
        report_artifact_id: Option<&str>,
        change_set_artifact_id: Option<&str>,
        error_code: Option<&str>,
    ) -> Result<(), &'static str> {
        self.persistence.save_attempt(AttemptUpdate {
            session_id: &request.context.session_id,
            delegation_id: &identity.delegation_id,
            attempt_id: &identity.attempt_id,
            mode: input.mode,
            task_hash: &identity.task_hash,
            status,
            created_at: &identity.created_at,
            updated_at: &Utc::now().to_rfc3339(),
            summary,
            report_artifact_id,
            change_set_artifact_id,
            error_code,
        })
    }
}

impl CliDelegationPort for ClaudeDelegationNativeToolAdapter {
    fn execute_delegation(&self, request: NativeToolPortRequest) -> NativeToolResultEnvelope {
        let cancelled = request.context.cancelled.clone();
        match self.execute(&request) {
            Ok(output) => envelope(NativeToolResultStatus::Succeeded, Some(output), None),
            Err("target_unavailable") => envelope(
                NativeToolResultStatus::Unavailable,
                None,
                Some(NativeToolErrorCode::Unavailable),
            ),
            Err(_) if cancelled.load(std::sync::atomic::Ordering::Acquire) => envelope(
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
