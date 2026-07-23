use crate::contexts::operations::api::{DiagnosticLog, DiagnosticLogPort};
use crate::contexts::operations::application::ApplicationError;
use std::sync::Mutex;

#[derive(Default)]
pub(super) struct CapturingDiagnostics {
    pub(super) logs: Mutex<Vec<DiagnosticLog>>,
}

impl DiagnosticLogPort for CapturingDiagnostics {
    fn write_diagnostic(&self, log: DiagnosticLog) -> Result<(), ApplicationError> {
        self.logs.lock().expect("diagnostic lock").push(log);
        Ok(())
    }
}
