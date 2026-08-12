use super::{EffectiveSkillDerivedCache, EffectiveSkillRuntimeCacheInvalidator};
use crate::contexts::tooling::skills::application::{
    EffectiveSkill, EffectiveSkillCatalogPort, OverlayAppliedSkillSnapshot,
    OverlayAppliedSkillSnapshotPort, OverlayEffectivePackageSnapshot, OverlayKey,
    OverlayRuntimeCacheInvalidationPort, SkillApplicationError, SkillPackageDescriptor,
    SkillPackageMaterializer, SkillResourceDocument,
};
use crate::contexts::tooling::skills::domain::{
    replay_overlay_scope_chain, BaseSkillResource, OverlayBaseWitness, OverlayDocument,
    OverlayFile, OverlayPatch, OverlayScope, OverlayScopeReplayInput, OverlayTrust,
    SkillAvailability, SkillDelivery, SkillId, SkillLayer, SkillMetadata, SkillOrigin, SkillTrust,
    SkillType,
};
use crate::test_support::TempDirectory;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::path::Path;
use std::sync::{Arc, Mutex};

struct FixedSnapshots {
    snapshot: OverlayAppliedSkillSnapshot,
    resources: Mutex<BTreeMap<String, Vec<u8>>>,
}

impl OverlayAppliedSkillSnapshotPort for FixedSnapshots {
    fn read_overlay_applied_package(
        &self,
        canonical_skill_id: &SkillId,
        _workspace_identity: Option<&str>,
    ) -> Result<OverlayAppliedSkillSnapshot, SkillApplicationError> {
        if canonical_skill_id != &self.snapshot.base.canonical_skill_id {
            return Err(SkillApplicationError::NotFound(
                canonical_skill_id.as_str().to_string(),
            ));
        }
        Ok(self.snapshot.clone())
    }

    fn read_overlay_applied_resource(
        &self,
        canonical_skill_id: &SkillId,
        workspace_identity: Option<&str>,
        expected_revision: &str,
        logical_path: &str,
    ) -> Result<SkillResourceDocument, SkillApplicationError> {
        let bytes = self.read_overlay_applied_resource_bytes(
            canonical_skill_id,
            workspace_identity,
            expected_revision,
            logical_path,
        )?;
        let content =
            String::from_utf8(bytes).map_err(|_| SkillApplicationError::BinaryResource)?;
        Ok(SkillResourceDocument {
            size_bytes: content.len() as u64,
            content,
        })
    }

    fn read_overlay_applied_resource_bytes(
        &self,
        canonical_skill_id: &SkillId,
        _workspace_identity: Option<&str>,
        expected_revision: &str,
        logical_path: &str,
    ) -> Result<Vec<u8>, SkillApplicationError> {
        if canonical_skill_id != &self.snapshot.base.canonical_skill_id
            || expected_revision != self.snapshot.replay.effective().effective_hash()
        {
            return Err(SkillApplicationError::ConcurrentModification(
                canonical_skill_id.as_str().to_string(),
            ));
        }
        self.resources
            .lock()
            .expect("resource bytes")
            .get(logical_path)
            .cloned()
            .ok_or_else(|| SkillApplicationError::NotFound(logical_path.to_string()))
    }
}

#[test]
fn effective_cache_materializes_only_the_healthy_logical_package_and_is_immutable() {
    let temporary = TempDirectory::new("Overlay effective cache");
    let package = package();
    let reference = b"Overlay reference".to_vec();
    let image = vec![0x89, b'P', b'N', b'G'];
    let base_resources = vec![BaseSkillResource {
        logical_path: "references/team.md".to_string(),
        media_type: "text/markdown".to_string(),
        size_bytes: 8,
        content_hash: hash(b"Base ref"),
        source_layer: SkillLayer::User,
    }];
    let base_replay =
        replay_overlay_scope_chain("Base instructions", &base_resources, &[], None, 0);
    let base = OverlayEffectivePackageSnapshot {
        canonical_skill_id: package.metadata.id.clone(),
        base_identity: package.package_key.clone(),
        base_layer: package.layer,
        instructions: "Base instructions".to_string(),
        resources: base_resources.clone(),
        instruction_hash: base_replay.base().instruction_hash().to_string(),
        package_hash: base_replay.base().effective_hash().to_string(),
    };
    let mut overlay = OverlayDocument::new(
        package.metadata.id.clone(),
        OverlayScope::User,
        None,
        OverlayBaseWitness::new(
            &base.base_identity,
            &base.instruction_hash,
            &base.package_hash,
        )
        .expect("base witness"),
        OverlayTrust::trusted_local(1),
        "2026-08-11T00:00:00Z",
    )
    .expect("Overlay");
    overlay.patches.push(
        OverlayPatch::new(
            "effective-patch",
            "Base instructions",
            "Effective instructions",
            false,
            &base.instruction_hash,
            "2026-08-11T00:00:00Z",
        )
        .expect("patch"),
    );
    overlay.files.push(
        OverlayFile::new(
            "reference-file",
            "references/team.md",
            "text/markdown",
            reference.len() as u64,
            &hash(&reference),
            "D:/private/.payloads/reference",
            "2026-08-11T00:00:00Z",
        )
        .expect("reference"),
    );
    overlay.files.push(
        OverlayFile::new(
            "image-file",
            "assets/logo.png",
            "image/png",
            image.len() as u64,
            &hash(&image),
            "D:/private/.payloads/image",
            "2026-08-11T00:00:00Z",
        )
        .expect("image"),
    );
    let replay = replay_overlay_scope_chain(
        "Base instructions",
        &base_resources,
        &[OverlayScopeReplayInput::verified(&overlay)],
        None,
        8,
    );
    let revision = replay.effective().effective_hash().to_string();
    let snapshots = std::sync::Arc::new(FixedSnapshots {
        snapshot: OverlayAppliedSkillSnapshot { base, replay },
        resources: Mutex::new(BTreeMap::from([
            ("references/team.md".to_string(), reference.clone()),
            ("assets/logo.png".to_string(), image.clone()),
        ])),
    });
    let cache = EffectiveSkillDerivedCache::with_root(
        temporary.path().join(".vanehub/cache/skills/effective"),
        snapshots,
    );

    let first = cache.materialize(&package).expect("materialized cache");
    let second = cache.materialize(&package).expect("idempotent cache");
    assert_eq!(first, second);
    assert_eq!(first.content_hash, revision);
    assert!(first.skill_dir.ends_with(&revision));
    let root = Path::new(&first.skill_dir);
    let skill = std::fs::read_to_string(root.join("SKILL.md")).expect("Skill document");
    assert!(skill.contains("Effective instructions"));
    assert!(!skill.contains("Base instructions"));
    assert_eq!(
        std::fs::read(root.join("references/team.md")).expect("reference"),
        reference
    );
    assert_eq!(
        std::fs::read(root.join("assets/logo.png")).expect("image"),
        image
    );
    assert!(!root.join("overlay.json").exists());
    assert!(!root.join(".payloads").exists());
    assert!(!skill.contains("D:/private"));
    for path in [
        root.join("SKILL.md"),
        root.join("references/team.md"),
        root.join("assets/logo.png"),
    ] {
        assert!(
            path.metadata()
                .expect("cache metadata")
                .permissions()
                .readonly(),
            "{} must be read-only",
            path.display()
        );
    }
}

#[derive(Default)]
struct RecordingCatalog {
    invalidations: Mutex<Vec<Option<String>>>,
}

impl EffectiveSkillCatalogPort for RecordingCatalog {
    fn effective_catalog(
        &self,
        _workspace_path: Option<&str>,
    ) -> Result<Vec<EffectiveSkill>, SkillApplicationError> {
        Ok(Vec::new())
    }

    fn invalidate(&self, workspace_path: Option<&str>) {
        self.invalidations
            .lock()
            .expect("catalog invalidations")
            .push(workspace_path.map(str::to_string));
    }
}

#[test]
fn runtime_invalidator_scopes_catalog_and_derived_generations_without_deleting_snapshots() {
    let temporary = TempDirectory::new("Overlay cache invalidation");
    let package = package();
    let base_replay = replay_overlay_scope_chain("Base", &[], &[], None, 0);
    let base = OverlayEffectivePackageSnapshot {
        canonical_skill_id: package.metadata.id.clone(),
        base_identity: package.package_key.clone(),
        base_layer: package.layer,
        instructions: "Base".to_string(),
        resources: Vec::new(),
        instruction_hash: base_replay.base().instruction_hash().to_string(),
        package_hash: base_replay.base().effective_hash().to_string(),
    };
    let snapshots = Arc::new(FixedSnapshots {
        snapshot: OverlayAppliedSkillSnapshot {
            base,
            replay: base_replay,
        },
        resources: Mutex::new(BTreeMap::new()),
    });
    let derived = Arc::new(EffectiveSkillDerivedCache::with_root(
        temporary.path().join(".vanehub/cache/skills/effective"),
        snapshots,
    ));
    let catalog = Arc::new(RecordingCatalog::default());
    let invalidator = EffectiveSkillRuntimeCacheInvalidator::new(catalog.clone(), derived.clone());
    let user_key = OverlayKey {
        canonical_skill_id: package.metadata.id.clone(),
        scope: OverlayScope::User,
        workspace_identity: None,
    };
    let workspace = "D:/canonical/project";
    let project_key = OverlayKey {
        canonical_skill_id: package.metadata.id.clone(),
        scope: OverlayScope::Project,
        workspace_identity: Some(workspace.to_string()),
    };

    let materialized = derived.materialize(&package).expect("initial snapshot");
    invalidator.invalidate(&user_key);
    invalidator.invalidate(&project_key);

    assert_eq!(
        *catalog.invalidations.lock().expect("catalog invalidations"),
        vec![None, Some(workspace.to_string())]
    );
    assert_eq!(
        derived.invalidation_generation(package.metadata.id.as_str(), None),
        (1, 0)
    );
    assert_eq!(
        derived.invalidation_generation(package.metadata.id.as_str(), Some(workspace)),
        (1, 1)
    );
    assert!(Path::new(&materialized.skill_dir).is_dir());
    assert_eq!(
        derived
            .materialize(&package)
            .expect("rematerialized snapshot"),
        materialized
    );
}

fn package() -> SkillPackageDescriptor {
    SkillPackageDescriptor {
        package_key: "user:effective-cache-skill".to_string(),
        workspace_path: None,
        metadata: SkillMetadata::with_classification(
            "effective-cache-skill",
            "Effective Cache Skill",
            "Fixture",
            "testing",
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
        revision: hash(b"base-package"),
        source_path: Some("D:/workspace/.vanehub/skills/effective-cache-skill".to_string()),
    }
}

fn hash(content: &[u8]) -> String {
    Sha256::digest(content)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}
