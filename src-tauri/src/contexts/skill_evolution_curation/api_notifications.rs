use super::api::SkillEvolutionCurationApi;
use super::api_action_support::storage;
use super::api_models::{CuratorApiError, CuratorApiResult};
use super::application::{CuratorNotificationDispatchReport, CuratorNotificationService};
use super::infrastructure::SqliteCuratorNotificationStore;
use crate::contexts::operations::application::{DiagnosticLog, DiagnosticLogPort, LogSeverity};
use serde_json::to_value;
use std::collections::BTreeMap;

impl SkillEvolutionCurationApi {
    pub(crate) fn dispatch_notifications(&self, now_ms: i64) -> CuratorApiResult {
        let connection = self.database.connection().map_err(|_| storage())?;
        let mut store = SqliteCuratorNotificationStore::new(&connection);
        match CuratorNotificationService::new(&mut store, self.notifications.as_ref())
            .dispatch(now_ms)
        {
            Ok(report) => {
                if report.failed > 0 {
                    notification_diagnostic(
                        self.logging.as_ref(),
                        LogSeverity::Warn,
                        "skill_curator.notification.delivery_failed",
                        report.failed,
                    );
                }
                to_value(report).map_err(|_| storage())
            }
            Err(_) => {
                notification_diagnostic(
                    self.logging.as_ref(),
                    LogSeverity::Warn,
                    "skill_curator.notification.dispatch_failed",
                    0,
                );
                Err(CuratorApiError::new("storage_unavailable"))
            }
        }
    }

    pub(super) fn dispatch_after(&self, result: CuratorApiResult, now_ms: i64) -> CuratorApiResult {
        if result.is_ok() {
            let _ = self.dispatch_notifications(now_ms);
        }
        result
    }
}

fn notification_diagnostic(
    logging: &dyn DiagnosticLogPort,
    severity: LogSeverity,
    category: &str,
    failed_count: usize,
) {
    let context = if failed_count == 0 {
        BTreeMap::new()
    } else {
        BTreeMap::from([("failedCount".to_string(), failed_count.to_string())])
    };
    let _ = logging.write_diagnostic(DiagnosticLog {
        severity,
        category: category.to_string(),
        message: "Curator notification delivery did not complete".to_string(),
        context,
    });
}
