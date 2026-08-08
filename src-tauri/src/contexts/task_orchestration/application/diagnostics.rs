#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PlanDiagnosticLevel {
    Error,
    Warn,
    Info,
    Debug,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PlanDiagnostic {
    pub(crate) level: PlanDiagnosticLevel,
    pub(crate) event: &'static str,
    pub(crate) plan_run_id: Option<String>,
    pub(crate) subtask_run_id: Option<String>,
    pub(crate) attempt_id: Option<String>,
    pub(crate) session_id: Option<String>,
    pub(crate) operation_id: Option<String>,
    pub(crate) execution_run_id: Option<String>,
    pub(crate) state: Option<&'static str>,
    pub(crate) error_class: Option<&'static str>,
}

impl PlanDiagnostic {
    pub(crate) fn lifecycle(
        level: PlanDiagnosticLevel,
        event: &'static str,
        plan_run_id: Option<&str>,
        state: Option<&'static str>,
    ) -> Self {
        Self {
            level,
            event,
            plan_run_id: plan_run_id.map(str::to_string),
            subtask_run_id: None,
            attempt_id: None,
            session_id: None,
            operation_id: None,
            execution_run_id: None,
            state,
            error_class: None,
        }
    }
}

pub(crate) trait PlanDiagnosticsPort: Send + Sync {
    fn record(&self, diagnostic: PlanDiagnostic);
}
