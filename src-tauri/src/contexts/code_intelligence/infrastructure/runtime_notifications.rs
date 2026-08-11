use super::diagnostics_cache::DiagnosticsCache;
use super::document_lease::DocumentLeaseManager;
use super::json_rpc_actor::JsonRpcNotification;
use lsp_types::PublishDiagnosticsParams;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::Mutex;
use url::Url;

#[derive(Clone)]
struct DiagnosticsTarget {
    documents: Arc<Mutex<DocumentLeaseManager>>,
    cache: Arc<DiagnosticsCache>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RuntimeNotificationObservation {
    pub(crate) diagnostic_count: usize,
}

#[derive(Clone, Default)]
pub(crate) struct RuntimeNotificationRouter {
    targets: Arc<RwLock<HashMap<u64, DiagnosticsTarget>>>,
}

impl RuntimeNotificationRouter {
    pub(crate) fn register_diagnostics(
        &self,
        process_id: u64,
        documents: Arc<Mutex<DocumentLeaseManager>>,
        cache: Arc<DiagnosticsCache>,
    ) {
        if let Ok(mut targets) = self.targets.write() {
            targets.insert(process_id, DiagnosticsTarget { documents, cache });
        }
    }

    pub(crate) fn process_exited(&self, process_id: u64) {
        let target = self
            .targets
            .write()
            .ok()
            .and_then(|mut targets| targets.remove(&process_id));
        if let Some(target) = target {
            target.cache.clear_after_process_exit();
        }
    }

    pub(crate) async fn handle(
        &self,
        process_id: u64,
        notification: JsonRpcNotification,
    ) -> Option<RuntimeNotificationObservation> {
        if notification.method != "textDocument/publishDiagnostics" {
            return None;
        }
        let params: PublishDiagnosticsParams = serde_json::from_value(notification.params).ok()?;
        let uri = Url::parse(params.uri.as_str()).ok()?;
        let target = self
            .targets
            .read()
            .ok()
            .and_then(|targets| targets.get(&process_id).cloned())?;
        let document = target.documents.lock().await.prepared_by_uri(&uri)?;
        target
            .cache
            .publish(&document, params, epoch_millis())
            .ok()?;
        Some(RuntimeNotificationObservation {
            diagnostic_count: target.cache.diagnostic_count(),
        })
    }
}

fn epoch_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| u64::try_from(duration.as_millis()).unwrap_or(u64::MAX))
        .unwrap_or(0)
}
