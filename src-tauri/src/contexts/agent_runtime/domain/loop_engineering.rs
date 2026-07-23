use super::AgentRuntimeDomainError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LoopRunStatus {
    Queued,
    Running,
    Paused,
    AwaitingAcceptance,
    Succeeded,
    Failed,
    Cancelled,
}

impl LoopRunStatus {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Queued => "queued",
            Self::Running => "running",
            Self::Paused => "paused",
            Self::AwaitingAcceptance => "awaiting-acceptance",
            Self::Succeeded => "succeeded",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
        }
    }

    pub(crate) fn parse(value: &str) -> Result<Self, AgentRuntimeDomainError> {
        match value {
            "queued" => Ok(Self::Queued),
            "running" => Ok(Self::Running),
            "paused" => Ok(Self::Paused),
            "awaiting-acceptance" => Ok(Self::AwaitingAcceptance),
            "succeeded" => Ok(Self::Succeeded),
            "failed" => Ok(Self::Failed),
            "cancelled" => Ok(Self::Cancelled),
            _ => Err(AgentRuntimeDomainError::InvalidLoopValue("status")),
        }
    }

    pub(crate) fn is_active(self) -> bool {
        matches!(
            self,
            Self::Queued | Self::Running | Self::Paused | Self::AwaitingAcceptance
        )
    }

    pub(crate) fn is_terminal(self) -> bool {
        matches!(self, Self::Succeeded | Self::Failed | Self::Cancelled)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LoopRunPhase {
    Preparing,
    Acting,
    Verifying,
    Deciding,
    Finalizing,
}

impl LoopRunPhase {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Preparing => "preparing",
            Self::Acting => "acting",
            Self::Verifying => "verifying",
            Self::Deciding => "deciding",
            Self::Finalizing => "finalizing",
        }
    }

    pub(crate) fn parse(value: &str) -> Result<Self, AgentRuntimeDomainError> {
        match value {
            "preparing" => Ok(Self::Preparing),
            "acting" => Ok(Self::Acting),
            "verifying" => Ok(Self::Verifying),
            "deciding" => Ok(Self::Deciding),
            "finalizing" => Ok(Self::Finalizing),
            _ => Err(AgentRuntimeDomainError::InvalidLoopValue("phase")),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LoopTerminalReason {
    GoalMet,
    MaxIterations,
    TimeBudget,
    PhaseTimeout,
    RuntimeErrors,
    NoProgress,
    VerificationFailed,
    VerifierBlocked,
    RuntimeError,
    RecoveryRequired,
    UserRejected,
    UserStopped,
}

impl LoopTerminalReason {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::GoalMet => "goal-met",
            Self::MaxIterations => "max-iterations",
            Self::TimeBudget => "time-budget",
            Self::PhaseTimeout => "phase-timeout",
            Self::RuntimeErrors => "runtime-errors",
            Self::NoProgress => "no-progress",
            Self::VerificationFailed => "verification-failed",
            Self::VerifierBlocked => "verifier-blocked",
            Self::RuntimeError => "runtime-error",
            Self::RecoveryRequired => "recovery-required",
            Self::UserRejected => "user-rejected",
            Self::UserStopped => "user-stopped",
        }
    }

    pub(crate) fn parse(value: &str) -> Result<Self, AgentRuntimeDomainError> {
        match value {
            "goal-met" => Ok(Self::GoalMet),
            "max-iterations" => Ok(Self::MaxIterations),
            "time-budget" => Ok(Self::TimeBudget),
            "phase-timeout" => Ok(Self::PhaseTimeout),
            "runtime-errors" => Ok(Self::RuntimeErrors),
            "no-progress" => Ok(Self::NoProgress),
            "verification-failed" => Ok(Self::VerificationFailed),
            "verifier-blocked" => Ok(Self::VerifierBlocked),
            "runtime-error" => Ok(Self::RuntimeError),
            "recovery-required" => Ok(Self::RecoveryRequired),
            "user-rejected" => Ok(Self::UserRejected),
            "user-stopped" => Ok(Self::UserStopped),
            _ => Err(AgentRuntimeDomainError::InvalidLoopValue("terminal reason")),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LoopLimits {
    max_iterations: u16,
    step_timeout_seconds: u64,
    total_timeout_seconds: u64,
    max_consecutive_runtime_errors: u16,
    max_consecutive_no_progress: u16,
}

impl LoopLimits {
    pub(crate) fn new(
        max_iterations: u16,
        step_timeout_seconds: u64,
        total_timeout_seconds: u64,
        max_consecutive_runtime_errors: u16,
        max_consecutive_no_progress: u16,
    ) -> Result<Self, AgentRuntimeDomainError> {
        if !(1..=20).contains(&max_iterations) {
            return Err(AgentRuntimeDomainError::InvalidLoopLimit("max iterations"));
        }
        if step_timeout_seconds == 0 || total_timeout_seconds < step_timeout_seconds {
            return Err(AgentRuntimeDomainError::InvalidLoopLimit("timeout"));
        }
        if max_consecutive_runtime_errors == 0 || max_consecutive_no_progress == 0 {
            return Err(AgentRuntimeDomainError::InvalidLoopLimit(
                "consecutive failures",
            ));
        }
        Ok(Self {
            max_iterations,
            step_timeout_seconds,
            total_timeout_seconds,
            max_consecutive_runtime_errors,
            max_consecutive_no_progress,
        })
    }

    pub(crate) fn max_iterations(&self) -> u16 {
        self.max_iterations
    }

    pub(crate) fn step_timeout_seconds(&self) -> u64 {
        self.step_timeout_seconds
    }

    pub(crate) fn total_timeout_seconds(&self) -> u64 {
        self.total_timeout_seconds
    }

    pub(crate) fn max_consecutive_runtime_errors(&self) -> u16 {
        self.max_consecutive_runtime_errors
    }

    pub(crate) fn max_consecutive_no_progress(&self) -> u16 {
        self.max_consecutive_no_progress
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LoopVerificationCommand {
    id: String,
    program: String,
    args: Vec<String>,
    working_directory: Option<String>,
    timeout_seconds: u64,
    required: bool,
}

impl LoopVerificationCommand {
    pub(crate) fn new(
        id: String,
        program: String,
        args: Vec<String>,
        working_directory: Option<String>,
        timeout_seconds: u64,
        required: bool,
    ) -> Result<Self, AgentRuntimeDomainError> {
        let id = required_text(id, "verification command id")?;
        let program = required_text(program, "verification program")?;
        if timeout_seconds == 0
            || contains_control(&program)
            || args.iter().any(|arg| contains_control(arg))
        {
            return Err(AgentRuntimeDomainError::InvalidLoopValue(
                "verification command",
            ));
        }
        if let Some(directory) = working_directory.as_deref() {
            if directory.is_empty()
                || directory.starts_with(['/', '\\'])
                || directory.as_bytes().get(1) == Some(&b':')
                || directory.split(['/', '\\']).any(|part| part == "..")
            {
                return Err(AgentRuntimeDomainError::InvalidLoopValue(
                    "verification working directory",
                ));
            }
        }
        Ok(Self {
            id,
            program,
            args,
            working_directory,
            timeout_seconds,
            required,
        })
    }

    pub(crate) fn id(&self) -> &str {
        &self.id
    }
    pub(crate) fn program(&self) -> &str {
        &self.program
    }
    pub(crate) fn args(&self) -> &[String] {
        &self.args
    }
    pub(crate) fn working_directory(&self) -> Option<&str> {
        self.working_directory.as_deref()
    }
    pub(crate) fn timeout_seconds(&self) -> u64 {
        self.timeout_seconds
    }
    pub(crate) fn required(&self) -> bool {
        self.required
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LoopDefinitionInput {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) enabled: bool,
    pub(crate) project_path: String,
    pub(crate) base_branch: String,
    pub(crate) goal: String,
    pub(crate) acceptance_criteria: Vec<String>,
    pub(crate) allowed_paths: Vec<String>,
    pub(crate) protected_paths: Vec<String>,
    pub(crate) worker_agent_id: String,
    pub(crate) verifier_agent_id: String,
    pub(crate) verification_commands: Vec<LoopVerificationCommand>,
    pub(crate) limits: LoopLimits,
    pub(crate) version: u64,
    pub(crate) created_at: String,
    pub(crate) updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LoopDefinition {
    input: LoopDefinitionInput,
}

impl LoopDefinition {
    pub(crate) fn new(mut input: LoopDefinitionInput) -> Result<Self, AgentRuntimeDomainError> {
        input.id = required_text(input.id, "Loop id")?;
        input.name = required_text(input.name, "Loop name")?;
        input.project_path = required_text(input.project_path, "project path")?;
        input.base_branch = required_text(input.base_branch, "base branch")?;
        input.goal = required_text(input.goal, "Loop goal")?;
        input.worker_agent_id = required_text(input.worker_agent_id, "Worker Agent id")?;
        input.verifier_agent_id = required_text(input.verifier_agent_id, "Verifier Agent id")?;
        input.acceptance_criteria = normalized_values(input.acceptance_criteria);
        input.allowed_paths = normalized_values(input.allowed_paths);
        input.protected_paths = normalized_values(input.protected_paths);
        if input.acceptance_criteria.is_empty()
            || input.verification_commands.is_empty()
            || input.version == 0
        {
            return Err(AgentRuntimeDomainError::InvalidLoopValue("definition"));
        }
        Ok(Self { input })
    }

    pub(crate) fn values(&self) -> &LoopDefinitionInput {
        &self.input
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LoopRun {
    id: String,
    definition_id: String,
    status: LoopRunStatus,
    phase: LoopRunPhase,
    terminal_reason: Option<LoopTerminalReason>,
    current_iteration: u16,
    consecutive_runtime_errors: u16,
    consecutive_no_progress: u16,
    pause_requested: bool,
}

impl LoopRun {
    pub(crate) fn new(id: String, definition_id: String) -> Result<Self, AgentRuntimeDomainError> {
        Ok(Self {
            id: required_text(id, "Loop run id")?,
            definition_id: required_text(definition_id, "Loop definition id")?,
            status: LoopRunStatus::Queued,
            phase: LoopRunPhase::Preparing,
            terminal_reason: None,
            current_iteration: 1,
            consecutive_runtime_errors: 0,
            consecutive_no_progress: 0,
            pause_requested: false,
        })
    }

    pub(crate) fn rehydrate(
        id: String,
        definition_id: String,
        status: LoopRunStatus,
        phase: LoopRunPhase,
        terminal_reason: Option<LoopTerminalReason>,
        current_iteration: u16,
        consecutive_runtime_errors: u16,
        consecutive_no_progress: u16,
        pause_requested: bool,
    ) -> Result<Self, AgentRuntimeDomainError> {
        if current_iteration == 0
            || !Self::is_valid_state(status, phase, terminal_reason, pause_requested)
        {
            return Err(AgentRuntimeDomainError::InvalidLoopValue("run state"));
        }
        Ok(Self {
            id: required_text(id, "Loop run id")?,
            definition_id: required_text(definition_id, "Loop definition id")?,
            status,
            phase,
            terminal_reason,
            current_iteration,
            consecutive_runtime_errors,
            consecutive_no_progress,
            pause_requested,
        })
    }

    pub(crate) fn begin(&mut self) -> Result<(), AgentRuntimeDomainError> {
        self.require_status(LoopRunStatus::Queued)?;
        self.status = LoopRunStatus::Running;
        self.phase = LoopRunPhase::Acting;
        Ok(())
    }

    pub(crate) fn move_to(&mut self, phase: LoopRunPhase) -> Result<(), AgentRuntimeDomainError> {
        self.require_status(LoopRunStatus::Running)?;
        let valid = matches!(
            (self.phase, phase),
            (LoopRunPhase::Acting, LoopRunPhase::Verifying)
                | (LoopRunPhase::Verifying, LoopRunPhase::Deciding)
                | (LoopRunPhase::Deciding, LoopRunPhase::Acting)
        );
        if !valid {
            return Err(self.transition_error(phase.as_str()));
        }
        self.phase = phase;
        Ok(())
    }

    pub(crate) fn request_pause(&mut self) -> Result<(), AgentRuntimeDomainError> {
        if self.pause_requested
            || !matches!(self.status, LoopRunStatus::Queued | LoopRunStatus::Running)
        {
            return Err(self.transition_error("pause-requested"));
        }
        self.pause_requested = true;
        Ok(())
    }

    pub(crate) fn pause_at_boundary(&mut self) -> Result<(), AgentRuntimeDomainError> {
        if !self.pause_requested
            || !matches!(self.status, LoopRunStatus::Queued | LoopRunStatus::Running)
        {
            return Err(self.transition_error("paused"));
        }
        self.status = LoopRunStatus::Paused;
        self.pause_requested = false;
        Ok(())
    }

    pub(crate) fn resume(&mut self) -> Result<(), AgentRuntimeDomainError> {
        self.require_status(LoopRunStatus::Paused)?;
        self.status = match self.phase {
            LoopRunPhase::Preparing => LoopRunStatus::Queued,
            LoopRunPhase::Finalizing => LoopRunStatus::AwaitingAcceptance,
            LoopRunPhase::Acting | LoopRunPhase::Verifying | LoopRunPhase::Deciding => {
                LoopRunStatus::Running
            }
        };
        self.terminal_reason = None;
        Ok(())
    }

    pub(crate) fn recover_orphaned(&mut self) -> Result<(), AgentRuntimeDomainError> {
        if !matches!(
            self.status,
            LoopRunStatus::Queued | LoopRunStatus::Running | LoopRunStatus::AwaitingAcceptance
        ) {
            return Err(self.transition_error("recovery-required"));
        }
        self.status = LoopRunStatus::Paused;
        self.terminal_reason = Some(LoopTerminalReason::RecoveryRequired);
        self.pause_requested = false;
        Ok(())
    }

    pub(crate) fn await_acceptance(
        &mut self,
        required_checks_passed: bool,
    ) -> Result<(), AgentRuntimeDomainError> {
        self.require_status(LoopRunStatus::Running)?;
        if self.phase != LoopRunPhase::Deciding || !required_checks_passed {
            return Err(self.transition_error("awaiting-acceptance"));
        }
        self.status = LoopRunStatus::AwaitingAcceptance;
        self.phase = LoopRunPhase::Finalizing;
        Ok(())
    }

    pub(crate) fn continue_iteration(
        &mut self,
        limits: &LoopLimits,
    ) -> Result<(), AgentRuntimeDomainError> {
        self.require_status(LoopRunStatus::AwaitingAcceptance)?;
        if self.current_iteration >= limits.max_iterations() {
            return Err(AgentRuntimeDomainError::LoopLimitReached("max iterations"));
        }
        self.current_iteration += 1;
        self.status = LoopRunStatus::Running;
        self.phase = LoopRunPhase::Acting;
        Ok(())
    }

    pub(crate) fn accept(&mut self) -> Result<(), AgentRuntimeDomainError> {
        self.require_status(LoopRunStatus::AwaitingAcceptance)?;
        self.status = LoopRunStatus::Succeeded;
        self.terminal_reason = Some(LoopTerminalReason::GoalMet);
        Ok(())
    }

    pub(crate) fn fail(
        &mut self,
        reason: LoopTerminalReason,
    ) -> Result<(), AgentRuntimeDomainError> {
        if self.status.is_terminal() {
            return Err(self.transition_error("failed"));
        }
        self.status = LoopRunStatus::Failed;
        self.phase = LoopRunPhase::Finalizing;
        self.terminal_reason = Some(reason);
        self.pause_requested = false;
        Ok(())
    }

    pub(crate) fn cancel(
        &mut self,
        reason: LoopTerminalReason,
    ) -> Result<(), AgentRuntimeDomainError> {
        if self.status.is_terminal() {
            return Err(self.transition_error("cancelled"));
        }
        self.status = LoopRunStatus::Cancelled;
        self.phase = LoopRunPhase::Finalizing;
        self.terminal_reason = Some(reason);
        self.pause_requested = false;
        Ok(())
    }

    pub(crate) fn enforce_elapsed_limits(
        &mut self,
        total_elapsed_seconds: u64,
        phase_elapsed_seconds: u64,
        limits: &LoopLimits,
    ) -> Result<Option<LoopTerminalReason>, AgentRuntimeDomainError> {
        self.require_executing()?;
        let reason = if total_elapsed_seconds >= limits.total_timeout_seconds() {
            Some(LoopTerminalReason::TimeBudget)
        } else if phase_elapsed_seconds >= limits.step_timeout_seconds() {
            Some(LoopTerminalReason::PhaseTimeout)
        } else {
            None
        };
        if let Some(reason) = reason {
            self.fail(reason)?;
        }
        Ok(reason)
    }

    pub(crate) fn record_runtime_outcome(
        &mut self,
        failed: bool,
        limits: &LoopLimits,
    ) -> Result<bool, AgentRuntimeDomainError> {
        self.require_executing()?;
        self.consecutive_runtime_errors = if failed {
            self.consecutive_runtime_errors.saturating_add(1)
        } else {
            0
        };
        let limit_reached =
            self.consecutive_runtime_errors >= limits.max_consecutive_runtime_errors();
        if limit_reached {
            self.fail(LoopTerminalReason::RuntimeErrors)?;
        }
        Ok(limit_reached)
    }

    pub(crate) fn try_begin_revision(
        &mut self,
        limits: &LoopLimits,
    ) -> Result<bool, AgentRuntimeDomainError> {
        self.require_status(LoopRunStatus::Running)?;
        if self.phase != LoopRunPhase::Deciding {
            return Err(self.transition_error("revision"));
        }
        if self.current_iteration >= limits.max_iterations() {
            self.fail(LoopTerminalReason::MaxIterations)?;
            return Ok(false);
        }
        self.current_iteration += 1;
        self.phase = LoopRunPhase::Acting;
        Ok(true)
    }

    pub(crate) fn record_progress(&mut self, progressed: bool, limits: &LoopLimits) -> bool {
        self.consecutive_no_progress = if progressed {
            0
        } else {
            self.consecutive_no_progress.saturating_add(1)
        };
        self.consecutive_no_progress >= limits.max_consecutive_no_progress()
    }

    pub(crate) fn record_revision_progress(
        &mut self,
        progressed: bool,
        limits: &LoopLimits,
    ) -> Result<bool, AgentRuntimeDomainError> {
        self.require_status(LoopRunStatus::Running)?;
        if self.phase != LoopRunPhase::Deciding {
            return Err(self.transition_error("revision-progress"));
        }
        let limit_reached = self.record_progress(progressed, limits);
        if limit_reached {
            self.fail(LoopTerminalReason::NoProgress)?;
        }
        Ok(limit_reached)
    }

    pub(crate) fn id(&self) -> &str {
        &self.id
    }
    pub(crate) fn definition_id(&self) -> &str {
        &self.definition_id
    }
    pub(crate) fn status(&self) -> LoopRunStatus {
        self.status
    }
    pub(crate) fn phase(&self) -> LoopRunPhase {
        self.phase
    }
    pub(crate) fn terminal_reason(&self) -> Option<LoopTerminalReason> {
        self.terminal_reason
    }
    pub(crate) fn current_iteration(&self) -> u16 {
        self.current_iteration
    }
    pub(crate) fn consecutive_runtime_errors(&self) -> u16 {
        self.consecutive_runtime_errors
    }
    pub(crate) fn consecutive_no_progress(&self) -> u16 {
        self.consecutive_no_progress
    }
    pub(crate) fn pause_requested(&self) -> bool {
        self.pause_requested
    }

    fn require_status(&self, expected: LoopRunStatus) -> Result<(), AgentRuntimeDomainError> {
        if self.status == expected {
            Ok(())
        } else {
            Err(self.transition_error(expected.as_str()))
        }
    }

    fn require_executing(&self) -> Result<(), AgentRuntimeDomainError> {
        if matches!(self.status, LoopRunStatus::Queued | LoopRunStatus::Running) {
            Ok(())
        } else {
            Err(self.transition_error("limit-check"))
        }
    }

    fn is_valid_state(
        status: LoopRunStatus,
        phase: LoopRunPhase,
        terminal_reason: Option<LoopTerminalReason>,
        pause_requested: bool,
    ) -> bool {
        let phase_matches_status = match status {
            LoopRunStatus::Queued => phase == LoopRunPhase::Preparing,
            LoopRunStatus::Running => matches!(
                phase,
                LoopRunPhase::Acting | LoopRunPhase::Verifying | LoopRunPhase::Deciding
            ),
            LoopRunStatus::Paused => true,
            LoopRunStatus::AwaitingAcceptance
            | LoopRunStatus::Succeeded
            | LoopRunStatus::Failed
            | LoopRunStatus::Cancelled => phase == LoopRunPhase::Finalizing,
        };
        let reason_matches_status = match status {
            LoopRunStatus::Paused => matches!(
                terminal_reason,
                None | Some(LoopTerminalReason::RecoveryRequired)
            ),
            status if status.is_terminal() => terminal_reason.is_some(),
            _ => terminal_reason.is_none(),
        };
        phase_matches_status && reason_matches_status && (!pause_requested || status.is_active())
    }

    fn transition_error(&self, to: &str) -> AgentRuntimeDomainError {
        AgentRuntimeDomainError::InvalidLoopTransition {
            from: self.status.as_str().to_string(),
            to: to.to_string(),
        }
    }
}

fn required_text(value: String, label: &'static str) -> Result<String, AgentRuntimeDomainError> {
    let value = value.trim().to_string();
    if value.is_empty() {
        return Err(AgentRuntimeDomainError::RequiredValue(label));
    }
    if contains_control(&value) {
        return Err(AgentRuntimeDomainError::ControlCharacters(label));
    }
    Ok(value)
}

fn normalized_values(values: Vec<String>) -> Vec<String> {
    let mut values = values
        .into_iter()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .collect::<Vec<_>>();
    values.sort();
    values.dedup();
    values
}

fn contains_control(value: &str) -> bool {
    value.chars().any(char::is_control)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn limits() -> LoopLimits {
        LoopLimits::new(3, 60, 600, 2, 2).expect("limits")
    }

    #[test]
    fn limits_and_commands_reject_unsafe_values() {
        assert!(LoopLimits::new(0, 60, 600, 2, 2).is_err());
        assert!(LoopLimits::new(3, 600, 60, 2, 2).is_err());
        assert!(LoopVerificationCommand::new(
            "test".to_string(),
            "npm".to_string(),
            vec!["test".to_string()],
            Some("../outside".to_string()),
            60,
            true,
        )
        .is_err());
    }

    #[test]
    fn run_requires_explicit_valid_transitions() {
        let mut run = LoopRun::new("run-1".to_string(), "loop-1".to_string()).expect("run");
        assert!(run.move_to(LoopRunPhase::Verifying).is_err());
        run.begin().expect("begin");
        run.move_to(LoopRunPhase::Verifying).expect("verify");
        run.move_to(LoopRunPhase::Deciding).expect("decide");
        assert!(run.move_to(LoopRunPhase::Finalizing).is_err());
        assert!(run.await_acceptance(false).is_err());
        run.await_acceptance(true).expect("await");
        run.accept().expect("accept");
        assert_eq!(run.status(), LoopRunStatus::Succeeded);
        assert_eq!(run.terminal_reason(), Some(LoopTerminalReason::GoalMet));
        assert!(run.cancel(LoopTerminalReason::UserStopped).is_err());
    }

    #[test]
    fn rehydrate_rejects_impossible_status_phase_combinations() {
        let invalid_states = [
            (LoopRunStatus::Queued, LoopRunPhase::Acting, None),
            (LoopRunStatus::Running, LoopRunPhase::Preparing, None),
            (
                LoopRunStatus::AwaitingAcceptance,
                LoopRunPhase::Deciding,
                None,
            ),
            (LoopRunStatus::Succeeded, LoopRunPhase::Finalizing, None),
            (
                LoopRunStatus::Failed,
                LoopRunPhase::Acting,
                Some(LoopTerminalReason::RuntimeError),
            ),
            (
                LoopRunStatus::Running,
                LoopRunPhase::Acting,
                Some(LoopTerminalReason::RuntimeError),
            ),
        ];

        for (status, phase, reason) in invalid_states {
            assert!(LoopRun::rehydrate(
                "run-1".to_string(),
                "loop-1".to_string(),
                status,
                phase,
                reason,
                1,
                0,
                0,
                false,
            )
            .is_err());
        }
    }

    #[test]
    fn rehydrate_accepts_exactly_the_complete_status_phase_matrix() {
        let statuses = [
            LoopRunStatus::Queued,
            LoopRunStatus::Running,
            LoopRunStatus::Paused,
            LoopRunStatus::AwaitingAcceptance,
            LoopRunStatus::Succeeded,
            LoopRunStatus::Failed,
            LoopRunStatus::Cancelled,
        ];
        let phases = [
            LoopRunPhase::Preparing,
            LoopRunPhase::Acting,
            LoopRunPhase::Verifying,
            LoopRunPhase::Deciding,
            LoopRunPhase::Finalizing,
        ];

        for status in statuses {
            for phase in phases {
                let reason = match status {
                    LoopRunStatus::Succeeded => Some(LoopTerminalReason::GoalMet),
                    LoopRunStatus::Failed => Some(LoopTerminalReason::RuntimeError),
                    LoopRunStatus::Cancelled => Some(LoopTerminalReason::UserStopped),
                    _ => None,
                };
                let expected = match status {
                    LoopRunStatus::Queued => phase == LoopRunPhase::Preparing,
                    LoopRunStatus::Running => matches!(
                        phase,
                        LoopRunPhase::Acting | LoopRunPhase::Verifying | LoopRunPhase::Deciding
                    ),
                    LoopRunStatus::Paused => true,
                    LoopRunStatus::AwaitingAcceptance
                    | LoopRunStatus::Succeeded
                    | LoopRunStatus::Failed
                    | LoopRunStatus::Cancelled => phase == LoopRunPhase::Finalizing,
                };

                let result = LoopRun::rehydrate(
                    format!("{}-{}", status.as_str(), phase.as_str()),
                    "loop-1".to_string(),
                    status,
                    phase,
                    reason,
                    1,
                    0,
                    0,
                    false,
                );
                assert_eq!(
                    result.is_ok(),
                    expected,
                    "unexpected rehydrate result for {} / {}",
                    status.as_str(),
                    phase.as_str()
                );
            }
        }
    }

    #[test]
    fn terminal_transitions_always_finalize_and_clear_pause_requests() {
        let mut failed = LoopRun::new("run-1".to_string(), "loop-1".to_string()).expect("run");
        failed.request_pause().expect("request pause");
        failed.fail(LoopTerminalReason::RuntimeError).expect("fail");
        assert_eq!(failed.status(), LoopRunStatus::Failed);
        assert_eq!(failed.phase(), LoopRunPhase::Finalizing);
        assert!(!failed.pause_requested());

        let mut cancelled = LoopRun::new("run-2".to_string(), "loop-1".to_string()).expect("run");
        cancelled.begin().expect("begin");
        cancelled
            .cancel(LoopTerminalReason::UserStopped)
            .expect("cancel");
        assert_eq!(cancelled.status(), LoopRunStatus::Cancelled);
        assert_eq!(cancelled.phase(), LoopRunPhase::Finalizing);
    }

    #[test]
    fn pause_limits_and_progress_are_domain_controlled() {
        let mut run = LoopRun::new("run-1".to_string(), "loop-1".to_string()).expect("run");
        run.begin().expect("begin");
        run.request_pause().expect("request pause");
        run.pause_at_boundary().expect("pause");
        run.resume().expect("resume");
        assert!(!run.record_progress(false, &limits()));
        assert!(run.record_progress(false, &limits()));
        assert!(!run
            .record_runtime_outcome(true, &limits())
            .expect("runtime error"));
        assert!(run
            .record_runtime_outcome(true, &limits())
            .expect("runtime error limit"));
    }

    #[test]
    fn repeated_revision_state_terminates_at_no_progress_limit() {
        let mut run = LoopRun::new("run-1".to_string(), "loop-1".to_string()).expect("run");
        run.begin().expect("begin");
        assert!(run.record_revision_progress(false, &limits()).is_err());
        run.move_to(LoopRunPhase::Verifying).expect("verify");
        run.move_to(LoopRunPhase::Deciding).expect("decide");

        assert!(!run
            .record_revision_progress(false, &limits())
            .expect("first repeat"));
        assert_eq!(run.consecutive_no_progress(), 1);
        assert!(!run
            .record_revision_progress(true, &limits())
            .expect("objective progress"));
        assert_eq!(run.consecutive_no_progress(), 0);
        assert!(!run
            .record_revision_progress(false, &limits())
            .expect("repeat after reset"));
        assert!(run
            .record_revision_progress(false, &limits())
            .expect("limit reached"));
        assert_eq!(run.status(), LoopRunStatus::Failed);
        assert_eq!(run.phase(), LoopRunPhase::Finalizing);
        assert_eq!(run.terminal_reason(), Some(LoopTerminalReason::NoProgress));
        assert!(run.record_revision_progress(false, &limits()).is_err());
    }

    #[test]
    fn elapsed_limits_use_stable_reasons_and_total_budget_precedence() {
        let limits = limits();
        let mut phase_timeout =
            LoopRun::new("phase-run".to_string(), "loop-1".to_string()).expect("run");
        phase_timeout.begin().expect("begin");
        assert_eq!(
            phase_timeout
                .enforce_elapsed_limits(599, 59, &limits)
                .expect("below limits"),
            None
        );
        assert_eq!(
            phase_timeout
                .enforce_elapsed_limits(599, 60, &limits)
                .expect("phase timeout"),
            Some(LoopTerminalReason::PhaseTimeout)
        );
        assert_eq!(phase_timeout.status(), LoopRunStatus::Failed);

        let mut total_timeout =
            LoopRun::new("total-run".to_string(), "loop-1".to_string()).expect("run");
        total_timeout.begin().expect("begin");
        assert_eq!(
            total_timeout
                .enforce_elapsed_limits(600, 60, &limits)
                .expect("total timeout"),
            Some(LoopTerminalReason::TimeBudget)
        );
    }

    #[test]
    fn runtime_error_limit_resets_on_success_and_terminates_at_threshold() {
        let limits = limits();
        let mut run = LoopRun::new("run-1".to_string(), "loop-1".to_string()).expect("run");
        run.begin().expect("begin");
        assert!(!run
            .record_runtime_outcome(true, &limits)
            .expect("first error"));
        assert!(!run
            .record_runtime_outcome(false, &limits)
            .expect("successful step"));
        assert_eq!(run.consecutive_runtime_errors(), 0);
        assert!(!run
            .record_runtime_outcome(true, &limits)
            .expect("first consecutive error"));
        assert!(run
            .record_runtime_outcome(true, &limits)
            .expect("error limit"));
        assert_eq!(run.status(), LoopRunStatus::Failed);
        assert_eq!(
            run.terminal_reason(),
            Some(LoopTerminalReason::RuntimeErrors)
        );
    }

    #[test]
    fn revision_at_max_iteration_terminates_with_stable_reason() {
        let limits = limits();
        let mut run = LoopRun::rehydrate(
            "run-1".to_string(),
            "loop-1".to_string(),
            LoopRunStatus::Running,
            LoopRunPhase::Deciding,
            None,
            limits.max_iterations(),
            0,
            0,
            false,
        )
        .expect("run");

        assert!(!run.try_begin_revision(&limits).expect("iteration limit"));
        assert_eq!(run.status(), LoopRunStatus::Failed);
        assert_eq!(
            run.terminal_reason(),
            Some(LoopTerminalReason::MaxIterations)
        );

        let mut next = LoopRun::new("run-2".to_string(), "loop-1".to_string()).expect("run");
        next.begin().expect("begin");
        next.move_to(LoopRunPhase::Verifying).expect("verify");
        next.move_to(LoopRunPhase::Deciding).expect("decide");
        assert!(next.try_begin_revision(&limits).expect("next iteration"));
        assert_eq!(next.current_iteration(), 2);
        assert_eq!(next.phase(), LoopRunPhase::Acting);
    }

    #[test]
    fn orphan_recovery_resumes_from_the_persisted_durable_boundary() {
        let mut preparing = LoopRun::new("preparing".to_string(), "loop".to_string()).expect("run");
        preparing.recover_orphaned().expect("recover preparing");
        assert_eq!(preparing.status(), LoopRunStatus::Paused);
        assert_eq!(
            preparing.terminal_reason(),
            Some(LoopTerminalReason::RecoveryRequired)
        );
        preparing.resume().expect("resume preparing");
        assert_eq!(preparing.status(), LoopRunStatus::Queued);

        let mut acceptance =
            LoopRun::new("acceptance".to_string(), "loop".to_string()).expect("run");
        acceptance.begin().expect("begin");
        acceptance.move_to(LoopRunPhase::Verifying).expect("verify");
        acceptance.move_to(LoopRunPhase::Deciding).expect("decide");
        acceptance.await_acceptance(true).expect("acceptance");
        acceptance.recover_orphaned().expect("recover acceptance");
        acceptance.resume().expect("resume acceptance");
        assert_eq!(acceptance.status(), LoopRunStatus::AwaitingAcceptance);
        assert_eq!(acceptance.phase(), LoopRunPhase::Finalizing);
        assert_eq!(acceptance.terminal_reason(), None);
    }
}
