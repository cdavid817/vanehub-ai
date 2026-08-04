use super::ConnectorRuntimeError;
use std::future::Future;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;

struct CachedToken {
    value: String,
    refresh_at: Instant,
}

pub(super) struct AccessTokenCache {
    value: Mutex<Option<CachedToken>>,
    safety_skew: Duration,
}

impl Default for AccessTokenCache {
    fn default() -> Self {
        Self {
            value: Mutex::new(None),
            safety_skew: Duration::from_secs(60),
        }
    }
}

impl AccessTokenCache {
    pub(super) async fn get_or_refresh<F, Fut>(
        &self,
        refresh: F,
    ) -> Result<String, ConnectorRuntimeError>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = Result<(String, Duration), ConnectorRuntimeError>>,
    {
        let mut cached = self.value.lock().await;
        if let Some(value) = cached
            .as_ref()
            .filter(|value| Instant::now() < value.refresh_at)
        {
            return Ok(value.value.clone());
        }
        let (value, lifetime) = refresh().await?;
        let usable = lifetime
            .saturating_sub(self.safety_skew)
            .max(Duration::from_secs(1));
        *cached = Some(CachedToken {
            value: value.clone(),
            refresh_at: Instant::now() + usable,
        });
        Ok(value)
    }

    pub(super) async fn invalidate(&self) {
        *self.value.lock().await = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    #[tokio::test]
    async fn concurrent_reads_single_flight_and_invalidation_forces_refresh() {
        let cache = Arc::new(AccessTokenCache::default());
        let refreshes = Arc::new(AtomicUsize::new(0));
        let calls = (0..16).map(|_| {
            let cache = cache.clone();
            let refreshes = refreshes.clone();
            async move {
                cache
                    .get_or_refresh(|| async move {
                        refreshes.fetch_add(1, Ordering::AcqRel);
                        tokio::task::yield_now().await;
                        Ok(("token".to_string(), Duration::from_secs(3_600)))
                    })
                    .await
            }
        });
        let results = futures_util::future::join_all(calls).await;
        assert!(results
            .into_iter()
            .all(|result| result.as_deref() == Ok("token")));
        assert_eq!(refreshes.load(Ordering::Acquire), 1);

        cache.invalidate().await;
        cache
            .get_or_refresh(|| async {
                refreshes.fetch_add(1, Ordering::AcqRel);
                Ok(("new-token".to_string(), Duration::from_secs(3_600)))
            })
            .await
            .expect("refresh");
        assert_eq!(refreshes.load(Ordering::Acquire), 2);
    }
}
