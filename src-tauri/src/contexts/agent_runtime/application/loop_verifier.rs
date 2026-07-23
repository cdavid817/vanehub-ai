use super::{
    AgentRuntimeApplicationError, LoopIterationRepository, LoopRoleGenerationOutcome,
    LoopRoleGenerationTerminal, LoopRoleSessionPort, LoopRoleSessionRequest,
    LoopVerifierContextPort, LoopVerifierGenerationPort, LoopVerifierRecommendation,
    LoopVerifierResult, SaveLoopVerifierResultRequest, StartLoopVerifierRequest,
    StartedLoopVerifierView,
};
use serde::Deserialize;
use std::sync::Arc;

const MAX_CONTEXT_BYTES: usize = 32 * 1024;
const MAX_FINDINGS: usize = 20;
const MAX_FINDING_BYTES: usize = 2 * 1024;

#[derive(Clone)]
pub(crate) struct LoopVerifierApplicationPorts {
    pub(crate) iterations: Arc<dyn LoopIterationRepository>,
    pub(crate) roles: Arc<dyn LoopRoleSessionPort>,
    pub(crate) context: Arc<dyn LoopVerifierContextPort>,
    pub(crate) generations: Arc<dyn LoopVerifierGenerationPort>,
}

#[derive(Clone)]
pub(crate) struct LoopVerifierApplicationService {
    ports: LoopVerifierApplicationPorts,
}

impl LoopVerifierApplicationService {
    pub(crate) fn new(ports: LoopVerifierApplicationPorts) -> Self {
        Self { ports }
    }

    pub(crate) fn start(
        &self,
        request: StartLoopVerifierRequest,
    ) -> Result<StartedLoopVerifierView, AgentRuntimeApplicationError> {
        validate_request(&request)?;
        let session_id = self
            .ports
            .roles
            .create_verifier_session(LoopRoleSessionRequest {
                run_id: request.run_id.clone(),
                iteration_id: request.iteration_id.clone(),
                agent_id: request.definition_snapshot.verifier_agent_id.clone(),
                project_path: request.project_path.clone(),
                worktree_path: request.worktree_path.clone(),
                worktree_name: request.worktree_name.clone(),
                worktree_branch: request.worktree_branch.clone(),
            })?;
        self.ports
            .iterations
            .attach_verifier_session(&request.iteration_id, &session_id)?;
        let diff = self.ports.context.bounded_diff(&session_id)?;
        let prompt = verifier_prompt(&request, &diff);
        let context_bytes = prompt.len();
        let message_id = self
            .ports
            .generations
            .start_verifier_generation(&session_id, &prompt)?;
        Ok(StartedLoopVerifierView {
            session_id,
            message_id,
            context_bytes,
        })
    }

    pub(crate) fn complete(
        &self,
        terminal: LoopRoleGenerationTerminal,
    ) -> Result<LoopVerifierResult, AgentRuntimeApplicationError> {
        validate_terminal(&terminal)?;
        let content = terminal
            .content
            .as_deref()
            .ok_or_else(|| validation("Completed Loop Verifier result has no content."))?;
        let result = parse_result(content)?;
        self.ports
            .iterations
            .save_verifier_result(&SaveLoopVerifierResultRequest {
                run_id: terminal.run_id,
                iteration_id: terminal.iteration_id,
                session_id: terminal.session_id,
                result: result.clone(),
            })?;
        Ok(result)
    }
}

fn validate_request(
    request: &StartLoopVerifierRequest,
) -> Result<(), AgentRuntimeApplicationError> {
    for (value, label) in [
        (&request.run_id, "Loop run id"),
        (&request.iteration_id, "Loop iteration id"),
        (&request.project_path, "Loop project path"),
        (&request.worktree_path, "Loop worktree path"),
        (&request.worktree_name, "Loop worktree name"),
        (&request.worktree_branch, "Loop worktree branch"),
    ] {
        if value.trim().is_empty() || value.chars().any(char::is_control) {
            return Err(validation(format!("{label} is required.")));
        }
    }
    if request.check_evidence.iter().any(|evidence| {
        evidence.run_id != request.run_id
            || evidence.iteration_id.as_deref() != Some(request.iteration_id.as_str())
            || evidence.kind != "verification-command"
    }) {
        return Err(validation(
            "Loop Verifier evidence must belong to this run and iteration.",
        ));
    }
    Ok(())
}

fn validate_terminal(
    terminal: &LoopRoleGenerationTerminal,
) -> Result<(), AgentRuntimeApplicationError> {
    if terminal.role != "verifier" {
        return Err(validation("Loop role result is not from a Verifier."));
    }
    match terminal.outcome {
        LoopRoleGenerationOutcome::Completed => Ok(()),
        LoopRoleGenerationOutcome::Failed => Err(AgentRuntimeApplicationError::Loop(
            terminal
                .error
                .clone()
                .unwrap_or_else(|| "Loop Verifier generation failed.".to_string()),
        )),
        LoopRoleGenerationOutcome::Cancelled => Err(AgentRuntimeApplicationError::Loop(
            "Loop Verifier generation was cancelled.".to_string(),
        )),
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct VerifierWireResult {
    recommendation: LoopVerifierRecommendation,
    findings: Vec<String>,
}

fn parse_result(content: &str) -> Result<LoopVerifierResult, AgentRuntimeApplicationError> {
    let parsed: VerifierWireResult = serde_json::from_str(content.trim()).map_err(|error| {
        validation(format!(
            "Loop Verifier must return the required JSON object: {error}"
        ))
    })?;
    if parsed.findings.len() > MAX_FINDINGS {
        return Err(validation("Loop Verifier returned too many findings."));
    }
    if parsed.findings.iter().any(|finding| {
        let value = finding.trim();
        value.is_empty()
            || value.len() > MAX_FINDING_BYTES
            || value
                .chars()
                .any(|character| character.is_control() && !matches!(character, '\n' | '\t'))
    }) {
        return Err(validation("Loop Verifier returned an invalid finding."));
    }
    if parsed.recommendation != LoopVerifierRecommendation::Pass && parsed.findings.is_empty() {
        return Err(validation(
            "Revise or blocked recommendations require at least one finding.",
        ));
    }
    Ok(LoopVerifierResult {
        recommendation: parsed.recommendation,
        findings: parsed
            .findings
            .into_iter()
            .map(|finding| finding.trim().to_string())
            .collect(),
    })
}

fn verifier_prompt(request: &StartLoopVerifierRequest, diff: &str) -> String {
    let fixed = concat!(
        "You are the read-only Verifier for a VaneHub Loop iteration. Inspect only; do not ",
        "modify files, execute mutating commands, or change run state. Return exactly one JSON ",
        "object with no markdown: {\"recommendation\":\"pass|revise|blocked\",",
        "\"findings\":[\"bounded actionable finding\"]}.\n\n"
    );
    let mut context = format!(
        "GOAL (immutable)\n{}\n\nACCEPTANCE CRITERIA (immutable)\n{}\n\nDETERMINISTIC CHECK EVIDENCE\n{}\n\nBOUNDED GIT DIFF\n{}",
        request.definition_snapshot.goal,
        bullet_list(&request.definition_snapshot.acceptance_criteria),
        evidence_summary(request),
        diff,
    );
    truncate_utf8(&mut context, MAX_CONTEXT_BYTES.saturating_sub(fixed.len()));
    format!("{fixed}{context}")
}

fn evidence_summary(request: &StartLoopVerifierRequest) -> String {
    if request.check_evidence.is_empty() {
        return "- No configured deterministic checks.".to_string();
    }
    request
        .check_evidence
        .iter()
        .map(|evidence| {
            format!(
                "- {} [{}]: {}",
                evidence.command_id.as_deref().unwrap_or("unknown"),
                evidence.status,
                evidence.summary
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn bullet_list(values: &[String]) -> String {
    values
        .iter()
        .map(|value| format!("- {}", value.trim()))
        .collect::<Vec<_>>()
        .join("\n")
}

fn truncate_utf8(value: &mut String, max_bytes: usize) {
    let mut boundary = max_bytes.min(value.len());
    while !value.is_char_boundary(boundary) {
        boundary -= 1;
    }
    value.truncate(boundary);
}

fn validation(message: impl Into<String>) -> AgentRuntimeApplicationError {
    AgentRuntimeApplicationError::Validation(message.into())
}
