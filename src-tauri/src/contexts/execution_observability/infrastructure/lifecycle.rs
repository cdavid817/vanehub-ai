use crate::contexts::execution_observability::application::{
    ExecutionTelemetryError, ExecutionTelemetryPort,
};
use std::sync::Arc;
use std::time::Duration;

#[derive(Clone)]
pub(crate) struct ExecutionTelemetryLifecycle {
    telemetry: Arc<dyn ExecutionTelemetryPort>,
    flush_timeout: Duration,
}

impl ExecutionTelemetryLifecycle {
    pub(crate) fn new(telemetry: Arc<dyn ExecutionTelemetryPort>, flush_timeout: Duration) -> Self {
        Self {
            telemetry,
            flush_timeout,
        }
    }

    pub(crate) fn shutdown(&self) -> Result<(), ExecutionTelemetryError> {
        self.telemetry.shutdown(self.flush_timeout)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contexts::execution_observability::application::ExecutionTelemetryPort;
    use crate::contexts::execution_observability::domain::{
        ExecutionEvent, ExecutionRun, ExecutionRunId, ExecutionSpan, ExecutionStatus, SpanId,
    };
    use std::sync::Mutex;

    #[derive(Default)]
    struct CapturingShutdown {
        timeout: Mutex<Option<Duration>>,
    }

    impl ExecutionTelemetryPort for CapturingShutdown {
        fn start_run(&self, _run: &ExecutionRun) -> Result<(), ExecutionTelemetryError> {
            Ok(())
        }
        fn finish_run(
            &self,
            _run_id: &ExecutionRunId,
            _status: ExecutionStatus,
            _ended_at: &str,
            _error_classification: Option<&str>,
        ) -> Result<(), ExecutionTelemetryError> {
            Ok(())
        }
        fn start_span(&self, _span: &ExecutionSpan) -> Result<(), ExecutionTelemetryError> {
            Ok(())
        }
        fn finish_span(
            &self,
            _run_id: &ExecutionRunId,
            _span_id: &SpanId,
            _status: ExecutionStatus,
            _ended_at: &str,
            _error_classification: Option<&str>,
        ) -> Result<(), ExecutionTelemetryError> {
            Ok(())
        }
        fn record_event(&self, _event: &ExecutionEvent) -> Result<(), ExecutionTelemetryError> {
            Ok(())
        }
        fn add_metric(
            &self,
            _name: &'static str,
            _value: u64,
            _dimensions: &[(&'static str, &'static str)],
        ) -> Result<(), ExecutionTelemetryError> {
            Ok(())
        }
        fn shutdown(&self, timeout: Duration) -> Result<(), ExecutionTelemetryError> {
            *self.timeout.lock().expect("timeout lock") = Some(timeout);
            Ok(())
        }
    }

    #[test]
    fn forwards_the_configured_bounded_shutdown_timeout() {
        let telemetry = Arc::new(CapturingShutdown::default());
        let lifecycle = ExecutionTelemetryLifecycle::new(telemetry.clone(), Duration::from_secs(3));
        lifecycle.shutdown().expect("shutdown");
        assert_eq!(
            *telemetry.timeout.lock().expect("timeout lock"),
            Some(Duration::from_secs(3))
        );
    }
}
