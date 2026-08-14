use super::native_tool_support::{
    executable_fingerprint, parse_report, prompt, ArtifactInputs, DelegationInput, REPORT_SCHEMA,
};
use super::{
    GitDelegationChangeSetCapture, ManagedDelegationProcessLauncher,
    SystemDelegationMaterializationAdapter,
};
use crate::contexts::agent_runtime::application::NativeToolPortRequest;
use crate::contexts::artifacts::application::{
    ArtifactCreateRequest, ArtifactCreator, ArtifactDescriptor, ArtifactEvidenceKind,
    ArtifactService, ArtifactVisibility,
};
use crate::contexts::cli_delegation::application::{
    ClaudeDelegationInvocationBuilder, ClaudeInvocationProfile, ClaudeInvocationRequest,
    DelegationAgentReportV1, DelegationChangeSetCapture, DelegationChangeSetCapturePort,
    DelegationEnvironmentBuilder, DelegationExecutionLimits, DelegationExecutionRequest,
    DelegationExecutionRunner, DelegationMaterializationPort, DelegationMaterializationRequest,
    DelegationMaterializer, DelegationMode, DelegationTarget, DelegationWorkspace,
};
use crate::contexts::tooling::cli::api::CliApi;
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

pub(super) struct PreparedOutcome {
    pub(super) report: DelegationAgentReportV1,
    pub(super) report_artifact: ArtifactDescriptor,
    pub(super) capture: DelegationChangeSetCapture,
    pub(super) exit_code: i32,
    pub(super) cli_fingerprint: String,
}

pub(super) struct AttemptExecutionIdentity<'a> {
    pub(super) delegation_id: &'a str,
    pub(super) attempt_id: &'a str,
    pub(super) created_at: &'a str,
}

pub(super) fn prepare_attempt(
    cli: &CliApi,
    artifacts: &Arc<ArtifactService>,
    request: &NativeToolPortRequest,
    workspace: &DelegationWorkspace,
    input: &DelegationInput,
    identity: AttemptExecutionIdentity<'_>,
) -> Result<PreparedOutcome, &'static str> {
    let materializer = DelegationMaterializer::new(
        Arc::new(ArtifactInputs(artifacts.clone())),
        Arc::new(SystemDelegationMaterializationAdapter),
    );
    let paths = materializer
        .materialize(
            workspace,
            &DelegationMaterializationRequest {
                task: input.task.clone(),
                context_summary: input.context_summary.clone(),
                artifact_ids: input.artifact_ids.clone(),
            },
        )
        .map_err(|_| "materialization_failure")?;
    let storage = SystemDelegationMaterializationAdapter;
    storage
        .write_new(&workspace.control.join("settings.json"), b"{}", true)
        .map_err(|_| "control_failure")?;
    storage
        .write_new(&workspace.control.join("mcp.json"), b"{}", true)
        .map_err(|_| "control_failure")?;
    let executable = PathBuf::from(
        cli.resolve_executable("claude-code")
            .map_err(|_| "target_unavailable")?
            .ok_or("target_unavailable")?,
    );
    let cli_fingerprint = executable_fingerprint(&executable)?;
    let task_prompt = prompt(&input.task, input.context_summary.as_deref(), &paths);
    let invocation = ClaudeDelegationInvocationBuilder::build(ClaudeInvocationRequest {
        executable,
        workspace: workspace.workspace.clone(),
        settings_file: &workspace.control.join("settings.json"),
        empty_mcp_config: &workspace.control.join("mcp.json"),
        task_prompt: &task_prompt,
        schema_json: REPORT_SCHEMA,
        mode: input.mode,
        maximum_turns: 32,
        maximum_budget_microusd: None,
        profile: ClaudeInvocationProfile::default(),
    })
    .map_err(|_| "invocation_failure")?;
    let environment = DelegationEnvironmentBuilder
        .build(
            DelegationTarget::ClaudeCode,
            &std::env::vars().collect::<BTreeMap<_, _>>(),
            &workspace.workspace,
        )
        .map_err(|_| "environment_failure")?;
    let observation = DelegationExecutionRunner::new(Arc::new(ManagedDelegationProcessLauncher))
        .run(
            &DelegationExecutionRequest {
                executable: invocation.executable,
                arguments: invocation.args,
                working_directory: invocation.working_directory,
                environment,
                stdin: invocation.stdin,
                limits: execution_limits(input.mode),
            },
            request.context.cancelled.as_ref(),
        )
        .map_err(|_| "execution_failure")?;
    let report = parse_report(&observation.stdout, observation.exit_code)?;
    let report_artifact = create_report(
        artifacts,
        identity.delegation_id,
        identity.attempt_id,
        identity.created_at,
        input,
        &report,
    )?;
    let capture = GitDelegationChangeSetCapture::new()
        .capture(
            &workspace.workspace,
            &workspace.control,
            &workspace.base_commit,
        )
        .map_err(|_| "capture_failure")?;
    if input.mode == DelegationMode::Analyze && !capture.files.is_empty() {
        return Err("analyze_mutation");
    }
    Ok(PreparedOutcome {
        report,
        report_artifact,
        capture,
        exit_code: observation.exit_code,
        cli_fingerprint,
    })
}

fn create_report(
    artifacts: &ArtifactService,
    delegation_id: &str,
    attempt_id: &str,
    created_at: &str,
    input: &DelegationInput,
    report: &DelegationAgentReportV1,
) -> Result<ArtifactDescriptor, &'static str> {
    artifacts
        .create_json(
            ArtifactCreateRequest {
                operation_id: delegation_id.to_owned(),
                display_name: format!("delegation-report-{attempt_id}.json"),
                media_type: "application/json".to_owned(),
                creator: ArtifactCreator {
                    kind: "delegation-attempt".to_owned(),
                    id: attempt_id.to_owned(),
                },
                evidence_kind: ArtifactEvidenceKind::ProviderReported,
                visibility: ArtifactVisibility::Private,
                source_artifact_ids: input.artifact_ids.clone(),
                created_at: created_at.to_owned(),
                expires_at: None,
            },
            &serde_json::to_value(report).map_err(|_| "report_failure")?,
        )
        .map_err(|_| "artifact_failure")
}

fn execution_limits(mode: DelegationMode) -> DelegationExecutionLimits {
    DelegationExecutionLimits {
        wall_time: Duration::from_secs(if mode == DelegationMode::Edit {
            1800
        } else {
            900
        }),
        stdout_bytes: 8 * 1024 * 1024,
        stderr_bytes: 1024 * 1024,
        events: 2048,
    }
}
