use std::{
    collections::BTreeMap,
    sync::{mpsc, Arc, Mutex},
    thread,
    time::Duration,
};

use chrono::Utc;

use crate::{
    contexts::{
        operations::api::{DiagnosticLog, DiagnosticLogPort, LogSeverity},
        skill_evolution_orchestration::application::{
            AuthoritativeTriggerSourceV1, EvolutionTriggerIngressService,
        },
    },
    platform::database::NativeDatabase,
};

#[derive(Clone)]
pub(crate) struct EvolutionBackgroundLifecycle {
    database: NativeDatabase,
    ingress: EvolutionTriggerIngressService,
    logging: Arc<dyn DiagnosticLogPort>,
    state: Arc<Mutex<BackgroundState>>,
}

#[derive(Default)]
struct BackgroundState {
    stop: Option<mpsc::Sender<()>>,
    worker: Option<thread::JoinHandle<()>>,
}

impl EvolutionBackgroundLifecycle {
    pub(crate) fn new(
        database: NativeDatabase,
        ingress: EvolutionTriggerIngressService,
        logging: Arc<dyn DiagnosticLogPort>,
    ) -> Self {
        Self {
            database,
            ingress,
            logging,
            state: Arc::new(Mutex::new(BackgroundState::default())),
        }
    }

    pub(crate) fn start(&self, interval: Duration) -> Result<(), String> {
        if interval.is_zero() {
            return Err("invalid-background-interval".into());
        }
        let mut state = self.state.lock().map_err(|_| "background-state-poisoned")?;
        if state.worker.is_some() {
            return Ok(());
        }
        let (stop, stopped) = mpsc::channel();
        let lifecycle = self.clone();
        state.stop = Some(stop);
        state.worker = Some(
            thread::Builder::new()
                .name("skill-evolution-maintenance".into())
                .spawn(move || lifecycle.run(stopped, interval))
                .map_err(|_| "background-start-failed")?,
        );
        Ok(())
    }

    pub(crate) fn shutdown(&self) -> Result<(), String> {
        let worker = {
            let mut state = self.state.lock().map_err(|_| "background-state-poisoned")?;
            if let Some(stop) = state.stop.take() {
                let _ = stop.send(());
            }
            state.worker.take()
        };
        worker
            .map(thread::JoinHandle::join)
            .transpose()
            .map_err(|_| "background-stop-failed")?;
        Ok(())
    }

    #[cfg(test)]
    pub(crate) fn is_running(&self) -> bool {
        self.state.lock().is_ok_and(|state| state.worker.is_some())
    }

    fn run(&self, stopped: mpsc::Receiver<()>, interval: Duration) {
        self.publish(workspaces(&self.database, true), true);
        while stopped.recv_timeout(interval) == Err(mpsc::RecvTimeoutError::Timeout) {
            self.publish(workspaces(&self.database, false), false);
        }
    }

    fn publish(&self, workspaces: Result<Vec<String>, String>, startup: bool) {
        let Ok(workspaces) = workspaces else {
            self.warn("workspace-scan-unavailable");
            return;
        };
        let now_ms = Utc::now().timestamp_millis();
        for workspace_id in workspaces {
            let source_id = if startup {
                format!("desktop-startup-{now_ms}")
            } else {
                format!("desktop-periodic-{}", now_ms / (15 * 60 * 1_000))
            };
            let source = AuthoritativeTriggerSourceV1 {
                workspace_id,
                source_id,
                source_revision: 1,
                occurred_at_ms: now_ms,
            };
            let failed = if startup {
                self.ingress.startup_recovery(source, now_ms).is_err()
            } else {
                self.ingress.periodic_maintenance(source, now_ms).is_err()
            };
            if failed {
                self.warn("trigger-unavailable");
            }
        }
    }

    fn warn(&self, reason: &'static str) {
        let _ = self.logging.write_diagnostic(DiagnosticLog {
            severity: LogSeverity::Warn,
            category: "skill-evolution.orchestration.background".into(),
            message: "Skill evolution background maintenance did not enqueue work".into(),
            context: BTreeMap::from([("reason".into(), reason.into())]),
        });
    }
}

fn workspaces(database: &NativeDatabase, include_recovery: bool) -> Result<Vec<String>, String> {
    let connection = database.connection().map_err(|_| "storage-unavailable")?;
    let sql = if include_recovery {
        "SELECT workspace_id FROM evolution_orchestration_policy UNION SELECT workspace_id FROM \
         evolution_runs WHERE status IN ('requested','waiting_idle','running','partial',\
         'cancel_requested','recovered') ORDER BY workspace_id"
    } else {
        "SELECT workspace_id FROM evolution_orchestration_policy WHERE mode!='off' ORDER BY workspace_id"
    };
    let mut statement = connection.prepare(sql).map_err(|_| "storage-unavailable")?;
    let rows = statement
        .query_map([], |row| row.get(0))
        .map_err(|_| "storage-unavailable")?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| "storage-unavailable")?;
    Ok(rows)
}

impl Drop for EvolutionBackgroundLifecycle {
    fn drop(&mut self) {
        if Arc::strong_count(&self.state) == 1 {
            let _ = self.shutdown();
        }
    }
}
