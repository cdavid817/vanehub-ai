use crate::contexts::communications::domain::{builtin_descriptors, ConnectorKind};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{Mutex, OwnedMutexGuard};

#[derive(Clone)]
pub(super) struct ConnectorLifecycleCoordinator {
    lanes: Arc<HashMap<ConnectorKind, Arc<Mutex<()>>>>,
}

impl Default for ConnectorLifecycleCoordinator {
    fn default() -> Self {
        Self {
            lanes: Arc::new(
                builtin_descriptors()
                    .into_iter()
                    .map(|descriptor| (descriptor.kind, Arc::new(Mutex::new(()))))
                    .collect(),
            ),
        }
    }
}

impl ConnectorLifecycleCoordinator {
    pub(super) async fn lock(&self, kind: ConnectorKind) -> OwnedMutexGuard<()> {
        self.lanes
            .get(&kind)
            .cloned()
            .unwrap_or_else(|| Arc::new(Mutex::new(())))
            .lock_owned()
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn same_connector_serializes_while_different_connectors_remain_responsive() {
        let coordinator = ConnectorLifecycleCoordinator::default();
        let telegram = coordinator.lock(ConnectorKind::Telegram).await;
        let same = {
            let coordinator = coordinator.clone();
            tokio::spawn(async move {
                let _guard = coordinator.lock(ConnectorKind::Telegram).await;
            })
        };
        let other = {
            let coordinator = coordinator.clone();
            tokio::spawn(async move {
                let _guard = coordinator.lock(ConnectorKind::Feishu).await;
            })
        };
        tokio::task::yield_now().await;
        assert!(!same.is_finished());
        other.await.expect("unrelated connector lock");

        drop(telegram);
        same.await.expect("serialized connector lock");
    }
}
