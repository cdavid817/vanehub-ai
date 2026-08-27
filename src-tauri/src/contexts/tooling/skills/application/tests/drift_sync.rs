use super::*;
use std::sync::atomic::{AtomicUsize, Ordering};

#[derive(Default)]
struct InvalidationCatalog {
    count: AtomicUsize,
}

impl EffectiveSkillCatalogPort for InvalidationCatalog {
    fn effective_catalog(
        &self,
        _workspace_path: Option<&str>,
    ) -> Result<Vec<EffectiveSkill>, SkillApplicationError> {
        Ok(Vec::new())
    }

    fn invalidate(&self, _workspace_path: Option<&str>) {
        self.count.fetch_add(1, Ordering::SeqCst);
    }
}

#[test]
fn drift_sync_invalidates_the_effective_catalog_before_the_overview_refreshes() {
    let fixture = Fixture::new();
    let catalog = Arc::new(InvalidationCatalog::default());
    let service = fixture
        .service
        .clone()
        .with_effective_catalog(catalog.clone());
    let existing = record("legacy-builtin", global(), SkillSource::User, true, &[]);
    fixture.repository.insert(existing.clone());
    *fixture
        .filesystem
        .inspection
        .lock()
        .expect("drift inspection") = Some(SkillDriftInspection {
        location: global(),
        registered: vec![RegisteredSkillInspection {
            id: existing.key.id.clone(),
            enabled: true,
            expected_content_hash: existing.managed_source.content_hash.clone(),
            source: SkillSourceInspection::Present {
                path: existing.managed_source.skill_md_path.clone(),
                content_hash: "changed-hash".to_string(),
            },
            bindings: Vec::new(),
        }],
        unregistered_sources: Vec::new(),
        deleted_builtin_ids: Vec::new(),
    });

    service
        .detect_skill_drift(SkillScopeQuery { location: global() })
        .expect("initial drift detection");
    catalog.count.store(0, Ordering::SeqCst);

    let result = service
        .sync_skill_drift(SkillScopeQuery { location: global() })
        .expect("drift sync");

    assert!(result.restored.contains(&"legacy-builtin".to_string()));
    assert_eq!(catalog.count.load(Ordering::SeqCst), 2);
}
