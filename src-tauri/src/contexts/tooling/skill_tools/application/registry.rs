use super::{
    project_contextual_catalog, SkillToolApplicationError, SkillToolCatalogCandidate,
    SkillToolCatalogContext, SkillToolCatalogPort, SkillToolCatalogSnapshot,
    SkillToolCompiledArtifactPort,
};
use crate::contexts::tooling::skill_tools::domain::SkillToolKey;
use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, RwLock, Weak};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SkillToolRegistryRefreshCause {
    Enablement,
    Archive,
    Delete,
    Replacement,
    Restore,
    EffectiveScope,
    Trust,
    Validation,
    Quarantine,
    GlobalKillSwitch,
    SkillKillSwitch,
}

#[derive(Debug, Clone)]
pub(crate) struct SkillToolRegistrySnapshot {
    pub(crate) generation: u64,
    pub(crate) cause: SkillToolRegistryRefreshCause,
    candidates: Vec<SkillToolCatalogCandidate>,
}

impl SkillToolRegistrySnapshot {
    fn build(
        generation: u64,
        cause: SkillToolRegistryRefreshCause,
        candidates: Vec<SkillToolCatalogCandidate>,
    ) -> Result<Self, SkillToolApplicationError> {
        let mut names = HashSet::new();
        let mut keys = HashSet::<SkillToolKey>::new();
        for candidate in &candidates {
            let expected = candidate
                .entry
                .key
                .canonical_name()
                .map_err(SkillToolApplicationError::Domain)?;
            if expected != candidate.entry.canonical_name
                || !names.insert(candidate.entry.canonical_name.clone())
                || !keys.insert(candidate.entry.key.clone())
            {
                return Err(SkillToolApplicationError::HostDenied(
                    "invalid-registry-snapshot".to_string(),
                ));
            }
        }
        Ok(Self {
            generation,
            cause,
            candidates,
        })
    }

    pub(crate) fn candidates(&self) -> &[SkillToolCatalogCandidate] {
        &self.candidates
    }
}

pub(crate) struct SkillToolRegistry {
    current: RwLock<Arc<SkillToolRegistrySnapshot>>,
    next_generation: AtomicU64,
    in_flight: Mutex<HashMap<SkillToolKey, Vec<Weak<AtomicBool>>>>,
    pinned_snapshots: Mutex<Vec<Weak<SkillToolRegistrySnapshot>>>,
    artifacts: Option<Arc<dyn SkillToolCompiledArtifactPort>>,
    global_enabled: AtomicBool,
    suppressed: Mutex<HashMap<String, Vec<SkillToolCatalogCandidate>>>,
}

pub(crate) struct SkillToolInvocationPin {
    pub(crate) snapshot: Arc<SkillToolRegistrySnapshot>,
    pub(crate) cancelled: Arc<AtomicBool>,
}

impl SkillToolRegistry {
    pub(crate) fn empty() -> Self {
        let snapshot = SkillToolRegistrySnapshot {
            generation: 0,
            cause: SkillToolRegistryRefreshCause::Restore,
            candidates: Vec::new(),
        };
        Self {
            current: RwLock::new(Arc::new(snapshot)),
            next_generation: AtomicU64::new(1),
            in_flight: Mutex::new(HashMap::new()),
            pinned_snapshots: Mutex::new(Vec::new()),
            artifacts: None,
            global_enabled: AtomicBool::new(true),
            suppressed: Mutex::new(HashMap::new()),
        }
    }

    pub(crate) fn with_artifacts(
        mut self,
        artifacts: Arc<dyn SkillToolCompiledArtifactPort>,
    ) -> Self {
        self.artifacts = Some(artifacts);
        self
    }

    pub(crate) fn snapshot(&self) -> Arc<SkillToolRegistrySnapshot> {
        self.current
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone()
    }

    pub(crate) fn refresh(
        &self,
        cause: SkillToolRegistryRefreshCause,
        candidates: Vec<SkillToolCatalogCandidate>,
    ) -> Result<Arc<SkillToolRegistrySnapshot>, SkillToolApplicationError> {
        let generation = self.next_generation.fetch_add(1, Ordering::SeqCst);
        let candidates = if self.global_enabled.load(Ordering::Acquire) {
            candidates
        } else {
            if !candidates.is_empty() {
                if let Ok(mut suppressed) = self.suppressed.lock() {
                    suppressed.insert("*".into(), candidates);
                }
            }
            Vec::new()
        };
        let next = Arc::new(SkillToolRegistrySnapshot::build(
            generation, cause, candidates,
        )?);
        let prior = self.snapshot();
        *self
            .current
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = next.clone();
        if cause == SkillToolRegistryRefreshCause::Quarantine {
            self.cancel_newly_quarantined(&prior, &next);
        }
        self.retire_unreferenced_artifacts(&next);
        Ok(next)
    }

    pub(crate) fn replace_owner(
        &self,
        cause: SkillToolRegistryRefreshCause,
        owner: &crate::contexts::tooling::skill_tools::domain::SkillToolOwnerId,
        mut candidates: Vec<SkillToolCatalogCandidate>,
    ) -> Result<Arc<SkillToolRegistrySnapshot>, SkillToolApplicationError> {
        candidates.extend(
            self.snapshot()
                .candidates
                .iter()
                .filter(|candidate| &candidate.entry.key.owner != owner)
                .cloned(),
        );
        self.refresh(cause, candidates)
    }

    pub(crate) fn remove_owner(
        &self,
        cause: SkillToolRegistryRefreshCause,
        owner: &crate::contexts::tooling::skill_tools::domain::SkillToolOwnerId,
    ) -> Result<Arc<SkillToolRegistrySnapshot>, SkillToolApplicationError> {
        let candidates = self
            .snapshot()
            .candidates
            .iter()
            .filter(|candidate| &candidate.entry.key.owner != owner)
            .cloned()
            .collect();
        self.refresh(cause, candidates)
    }

    pub(crate) fn set_global_execution_enabled(
        &self,
        enabled: bool,
    ) -> Result<Arc<SkillToolRegistrySnapshot>, SkillToolApplicationError> {
        self.global_enabled.store(enabled, Ordering::Release);
        let candidates = if enabled {
            self.suppressed
                .lock()
                .map_err(|_| SkillToolApplicationError::Storage("registry-kill-switch".into()))?
                .remove("*")
                .unwrap_or_default()
        } else {
            let candidates = self.snapshot().candidates.clone();
            self.suppressed
                .lock()
                .map_err(|_| SkillToolApplicationError::Storage("registry-kill-switch".into()))?
                .insert("*".into(), candidates);
            Vec::new()
        };
        self.refresh(SkillToolRegistryRefreshCause::GlobalKillSwitch, candidates)
    }

    pub(crate) fn set_owner_execution_enabled(
        &self,
        owner: &crate::contexts::tooling::skill_tools::domain::SkillToolOwnerId,
        enabled: bool,
    ) -> Result<Arc<SkillToolRegistrySnapshot>, SkillToolApplicationError> {
        let owner_key = owner.as_str().to_string();
        let mut candidates = self.snapshot().candidates.clone();
        if enabled {
            candidates.extend(
                self.suppressed
                    .lock()
                    .map_err(|_| SkillToolApplicationError::Storage("registry-kill-switch".into()))?
                    .remove(&owner_key)
                    .unwrap_or_default(),
            );
        } else {
            let (removed, retained): (Vec<_>, Vec<_>) = candidates
                .into_iter()
                .partition(|candidate| candidate.entry.key.owner == *owner);
            candidates = retained;
            self.suppressed
                .lock()
                .map_err(|_| SkillToolApplicationError::Storage("registry-kill-switch".into()))?
                .insert(owner_key, removed);
        }
        self.refresh(SkillToolRegistryRefreshCause::SkillKillSwitch, candidates)
    }

    pub(crate) fn pin_invocation(
        &self,
        key: &SkillToolKey,
    ) -> Result<SkillToolInvocationPin, SkillToolApplicationError> {
        let snapshot = self.snapshot();
        if !snapshot
            .candidates
            .iter()
            .any(|candidate| &candidate.entry.key == key)
        {
            return Err(SkillToolApplicationError::StaleRevision);
        }
        let cancelled = Arc::new(AtomicBool::new(false));
        self.pinned_snapshots
            .lock()
            .map_err(|_| SkillToolApplicationError::Storage("registry-pins".to_string()))?
            .push(Arc::downgrade(&snapshot));
        self.in_flight
            .lock()
            .map_err(|_| SkillToolApplicationError::Storage("registry-in-flight".to_string()))?
            .entry(key.clone())
            .or_default()
            .push(Arc::downgrade(&cancelled));
        Ok(SkillToolInvocationPin {
            snapshot,
            cancelled,
        })
    }

    fn retire_unreferenced_artifacts(&self, current: &Arc<SkillToolRegistrySnapshot>) {
        let Some(artifacts) = &self.artifacts else {
            return;
        };
        let mut revisions = current
            .candidates
            .iter()
            .map(|candidate| candidate.entry.key.revision.clone())
            .collect::<HashSet<_>>();
        if let Ok(mut pins) = self.pinned_snapshots.lock() {
            pins.retain(|pin| {
                if let Some(snapshot) = pin.upgrade() {
                    revisions.extend(
                        snapshot
                            .candidates
                            .iter()
                            .map(|item| item.entry.key.revision.clone()),
                    );
                    true
                } else {
                    false
                }
            });
        }
        artifacts.retain_revisions(&revisions);
    }

    fn cancel_newly_quarantined(
        &self,
        prior: &SkillToolRegistrySnapshot,
        next: &SkillToolRegistrySnapshot,
    ) {
        let quarantined = prior
            .candidates
            .iter()
            .filter(|candidate| {
                next.candidates.iter().any(|next| {
                    next.entry.key == candidate.entry.key
                        && next.lifecycle.quarantine.is_quarantined()
                        && !candidate.lifecycle.quarantine.is_quarantined()
                })
            })
            .map(|candidate| candidate.entry.key.clone())
            .collect::<Vec<_>>();
        if let Ok(mut active) = self.in_flight.lock() {
            for key in quarantined {
                if let Some(tokens) = active.remove(&key) {
                    for token in tokens.into_iter().filter_map(|token| token.upgrade()) {
                        token.store(true, Ordering::Release);
                    }
                }
            }
            active.retain(|_, tokens| {
                tokens.retain(|token| token.strong_count() > 0);
                !tokens.is_empty()
            });
        }
    }
}

impl SkillToolCatalogPort for SkillToolRegistry {
    fn catalog_for(
        &self,
        context: &SkillToolCatalogContext,
    ) -> Result<SkillToolCatalogSnapshot, SkillToolApplicationError> {
        let snapshot = self.snapshot();
        Ok(SkillToolCatalogSnapshot {
            generation: snapshot.generation,
            entries: project_contextual_catalog(snapshot.candidates(), context),
            lease: snapshot,
        })
    }
}

#[cfg(test)]
#[path = "registry_tests.rs"]
mod tests;
