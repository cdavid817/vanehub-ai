use crate::contexts::tooling::skills::application::{
    resolve_effective_catalog, EffectiveCatalogCacheKey, EffectiveSkill, EffectiveSkillCatalogPort,
    SkillApplicationError, SkillLayerProvider, SkillPackageDescriptor,
};
use std::collections::BTreeMap;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::path::Path;
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc, Mutex,
};

pub(crate) struct CachedEffectiveSkillCatalog {
    providers: Vec<Arc<dyn SkillLayerProvider>>,
    entries: Mutex<BTreeMap<EffectiveCatalogCacheKey, Vec<EffectiveSkill>>>,
    state_revision: AtomicU64,
}

impl CachedEffectiveSkillCatalog {
    pub(crate) fn new(providers: Vec<Arc<dyn SkillLayerProvider>>) -> Self {
        Self {
            providers,
            entries: Mutex::new(BTreeMap::new()),
            state_revision: AtomicU64::new(0),
        }
    }

    fn inventory(
        &self,
        workspace_path: Option<&str>,
    ) -> Result<Vec<SkillPackageDescriptor>, SkillApplicationError> {
        let mut packages = Vec::new();
        for provider in &self.providers {
            packages.extend(provider.inventory(workspace_path)?);
        }
        packages.sort_by(|left, right| {
            left.layer
                .rank()
                .cmp(&right.layer.rank())
                .then_with(|| left.package_key.cmp(&right.package_key))
        });
        Ok(packages)
    }
}

impl EffectiveSkillCatalogPort for CachedEffectiveSkillCatalog {
    fn effective_catalog(
        &self,
        workspace_path: Option<&str>,
    ) -> Result<Vec<EffectiveSkill>, SkillApplicationError> {
        let workspace_path = canonical_workspace(workspace_path)?;
        let inventory = self.inventory(workspace_path.as_deref())?;
        let key = EffectiveCatalogCacheKey {
            workspace_path: workspace_path.clone(),
            inventory_fingerprint: inventory_fingerprint(&inventory),
            state_revision: self.state_revision.load(Ordering::Acquire),
        };
        if let Some(catalog) = self.entries.lock().map_err(cache_error)?.get(&key).cloned() {
            return Ok(catalog);
        }
        let catalog = resolve_effective_catalog(workspace_path.as_deref(), inventory);
        self.entries
            .lock()
            .map_err(cache_error)?
            .insert(key, catalog.clone());
        Ok(catalog)
    }

    fn invalidate(&self, workspace_path: Option<&str>) {
        self.state_revision.fetch_add(1, Ordering::AcqRel);
        let Ok(mut entries) = self.entries.lock() else {
            return;
        };
        match canonical_workspace(workspace_path) {
            Ok(Some(workspace)) => {
                entries.retain(|key, _| key.workspace_path.as_deref() != Some(workspace.as_str()))
            }
            Ok(None) | Err(_) => entries.clear(),
        }
    }
}

fn canonical_workspace(
    workspace_path: Option<&str>,
) -> Result<Option<String>, SkillApplicationError> {
    workspace_path
        .map(|workspace| {
            Path::new(workspace)
                .canonicalize()
                .map(|path| normalize_path(&path))
                .map_err(|error| SkillApplicationError::Filesystem(error.to_string()))
        })
        .transpose()
}

fn inventory_fingerprint(packages: &[SkillPackageDescriptor]) -> String {
    let mut hasher = DefaultHasher::new();
    for package in packages {
        package.package_key.hash(&mut hasher);
        package.workspace_path.hash(&mut hasher);
        package.metadata.id.hash(&mut hasher);
        package.metadata.skill_type.hash(&mut hasher);
        package.metadata.delivery.hash(&mut hasher);
        package.layer.hash(&mut hasher);
        package.origin.hash(&mut hasher);
        package.trust.hash(&mut hasher);
        package.availability.hash(&mut hasher);
        package.revision.hash(&mut hasher);
    }
    format!("{:016x}", hasher.finish())
}

fn normalize_path(path: &Path) -> String {
    let value = path.to_string_lossy();
    value.strip_prefix(r"\\?\").unwrap_or(&value).to_string()
}

fn cache_error<T>(error: std::sync::PoisonError<T>) -> SkillApplicationError {
    SkillApplicationError::Repository(format!("Effective Skill cache is unavailable: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contexts::tooling::skills::domain::{
        SkillAvailability, SkillDelivery, SkillLayer, SkillMetadata, SkillOrigin, SkillTrust,
        SkillType,
    };
    use crate::test_support::TempDirectory;
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct CountingProvider {
        revision: AtomicUsize,
        scans: AtomicUsize,
    }

    impl CountingProvider {
        fn new() -> Self {
            Self {
                revision: AtomicUsize::new(1),
                scans: AtomicUsize::new(0),
            }
        }
    }

    impl SkillLayerProvider for CountingProvider {
        fn inventory(
            &self,
            _workspace_path: Option<&str>,
        ) -> Result<Vec<SkillPackageDescriptor>, SkillApplicationError> {
            self.scans.fetch_add(1, Ordering::Relaxed);
            let revision = self.revision.load(Ordering::Relaxed);
            Ok(vec![descriptor(revision)])
        }
    }

    fn descriptor(revision: usize) -> SkillPackageDescriptor {
        SkillPackageDescriptor {
            package_key: "user/developer".to_string(),
            workspace_path: None,
            metadata: SkillMetadata::with_classification(
                "developer",
                "Developer",
                "Description",
                "development",
                "1.0.0",
                Vec::new(),
                Vec::new(),
                Some(SkillType::Role),
                Some(SkillDelivery::OnDemand),
            )
            .expect("metadata"),
            layer: SkillLayer::User,
            origin: SkillOrigin::Created,
            trust: SkillTrust::Trusted,
            availability: SkillAvailability::Available,
            revision: revision.to_string(),
            source_path: None,
        }
    }

    #[test]
    fn cache_key_tracks_inventory_and_persisted_state_revision() {
        let provider = Arc::new(CountingProvider::new());
        let catalog = CachedEffectiveSkillCatalog::new(vec![provider.clone()]);
        let first = catalog.effective_catalog(None).expect("catalog");
        let second = catalog.effective_catalog(None).expect("cached catalog");
        assert_eq!(first, second);
        assert_eq!(catalog.entries.lock().expect("entries").len(), 1);

        provider.revision.store(2, Ordering::Relaxed);
        catalog.effective_catalog(None).expect("changed inventory");
        assert_eq!(catalog.entries.lock().expect("entries").len(), 2);

        catalog.invalidate(None);
        assert!(catalog.entries.lock().expect("entries").is_empty());
        catalog
            .effective_catalog(None)
            .expect("invalidated catalog");
        assert_eq!(catalog.entries.lock().expect("entries").len(), 1);
    }

    #[test]
    fn workspace_invalidation_preserves_other_workspace_entries() {
        let first = TempDirectory::new("effective-cache-first");
        let second = TempDirectory::new("effective-cache-second");
        let provider = Arc::new(CountingProvider::new());
        let catalog = CachedEffectiveSkillCatalog::new(vec![provider]);
        let first_path = first.path().to_string_lossy();
        let second_path = second.path().to_string_lossy();
        catalog
            .effective_catalog(Some(&first_path))
            .expect("first catalog");
        catalog
            .effective_catalog(Some(&second_path))
            .expect("second catalog");
        catalog.invalidate(Some(&first_path));
        let entries = catalog.entries.lock().expect("entries");
        assert_eq!(entries.len(), 1);
        assert_eq!(
            entries
                .keys()
                .next()
                .and_then(|key| key.workspace_path.as_deref()),
            canonical_workspace(Some(&second_path))
                .expect("canonical")
                .as_deref()
        );
    }
}
