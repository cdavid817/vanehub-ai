use super::ApplicationError;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LogSeverity {
    Error,
    Warn,
    Info,
    Debug,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DiagnosticLog {
    pub(crate) severity: LogSeverity,
    pub(crate) category: String,
    pub(crate) message: String,
    pub(crate) context: BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct OperationLog {
    pub(crate) operation_id: String,
    pub(crate) severity: LogSeverity,
    pub(crate) category: String,
    pub(crate) message: String,
    pub(crate) context: BTreeMap<String, String>,
}

pub(crate) trait DiagnosticLogPort: Send + Sync {
    fn write_diagnostic(&self, log: DiagnosticLog) -> Result<(), ApplicationError>;
}

pub(crate) trait OperationLogPort: Send + Sync {
    fn write_operation(&self, log: OperationLog) -> Result<(), ApplicationError>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    #[derive(Default)]
    struct CapturingLogPort {
        diagnostics: Mutex<Vec<DiagnosticLog>>,
        operations: Mutex<Vec<OperationLog>>,
    }

    impl DiagnosticLogPort for CapturingLogPort {
        fn write_diagnostic(&self, log: DiagnosticLog) -> Result<(), ApplicationError> {
            self.diagnostics.lock().expect("diagnostics").push(log);
            Ok(())
        }
    }

    impl OperationLogPort for CapturingLogPort {
        fn write_operation(&self, log: OperationLog) -> Result<(), ApplicationError> {
            self.operations.lock().expect("operations").push(log);
            Ok(())
        }
    }

    #[test]
    fn diagnostic_and_operation_contracts_remain_distinct() {
        let port = CapturingLogPort::default();
        port.write_diagnostic(DiagnosticLog {
            severity: LogSeverity::Warn,
            category: "runtime.health".to_string(),
            message: "degraded".to_string(),
            context: BTreeMap::new(),
        })
        .expect("diagnostic");
        port.write_operation(OperationLog {
            operation_id: "op-17".to_string(),
            severity: LogSeverity::Info,
            category: "sdk.operation".to_string(),
            message: "installed".to_string(),
            context: BTreeMap::new(),
        })
        .expect("operation");

        assert_eq!(port.diagnostics.lock().expect("diagnostics").len(), 1);
        assert_eq!(
            port.operations.lock().expect("operations")[0].operation_id,
            "op-17"
        );
    }
}
