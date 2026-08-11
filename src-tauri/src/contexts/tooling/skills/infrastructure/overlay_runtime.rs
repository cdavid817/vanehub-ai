use crate::contexts::tooling::skills::application::{
    EffectiveSkillCatalogPort, OverlayEffectivePackageSnapshot, OverlayEffectiveSnapshotPort,
    SkillApplicationError, SkillPackageDescriptor, SkillPackageReader, SkillResourceDocument,
};
use crate::contexts::tooling::skills::domain::{
    replay_overlay_scope_chain, BaseSkillResource, SkillId,
};
use std::sync::Arc;

pub(crate) struct CatalogOverlayEffectiveSnapshot {
    catalog: Arc<dyn EffectiveSkillCatalogPort>,
    reader: Arc<dyn SkillPackageReader>,
}

impl CatalogOverlayEffectiveSnapshot {
    pub(crate) fn new(
        catalog: Arc<dyn EffectiveSkillCatalogPort>,
        reader: Arc<dyn SkillPackageReader>,
    ) -> Self {
        Self { catalog, reader }
    }

    fn effective_package(
        &self,
        canonical_skill_id: &SkillId,
        workspace_identity: Option<&str>,
    ) -> Result<SkillPackageDescriptor, SkillApplicationError> {
        self.catalog
            .effective_catalog(workspace_identity)?
            .into_iter()
            .find(|skill| skill.effective.metadata.id == *canonical_skill_id)
            .map(|skill| skill.effective)
            .ok_or_else(|| SkillApplicationError::NotFound(canonical_skill_id.as_str().to_string()))
    }
}

impl OverlayEffectiveSnapshotPort for CatalogOverlayEffectiveSnapshot {
    fn read_effective_package(
        &self,
        canonical_skill_id: &SkillId,
        workspace_identity: Option<&str>,
    ) -> Result<OverlayEffectivePackageSnapshot, SkillApplicationError> {
        let package = self.effective_package(canonical_skill_id, workspace_identity)?;
        let document = self.reader.read_document(&package)?;
        let resources = self
            .reader
            .list_resources(&package)?
            .into_iter()
            .map(|resource| BaseSkillResource {
                logical_path: resource.relative_path,
                media_type: resource.media_type,
                size_bytes: resource.size_bytes,
                content_hash: resource.content_hash,
                source_layer: package.layer,
            })
            .collect::<Vec<_>>();
        let base_replay =
            replay_overlay_scope_chain(&document.body, &resources, &[], workspace_identity, 0);
        Ok(OverlayEffectivePackageSnapshot {
            canonical_skill_id: canonical_skill_id.clone(),
            base_identity: package.package_key,
            base_layer: package.layer,
            instructions: document.body,
            resources,
            instruction_hash: base_replay.base().instruction_hash().to_string(),
            package_hash: base_replay.base().effective_hash().to_string(),
        })
    }

    fn read_effective_resource(
        &self,
        canonical_skill_id: &SkillId,
        workspace_identity: Option<&str>,
        logical_path: &str,
    ) -> Result<SkillResourceDocument, SkillApplicationError> {
        let package = self.effective_package(canonical_skill_id, workspace_identity)?;
        self.reader.read_resource(&package, logical_path)
    }

    fn read_effective_resource_bytes(
        &self,
        canonical_skill_id: &SkillId,
        workspace_identity: Option<&str>,
        logical_path: &str,
    ) -> Result<Vec<u8>, SkillApplicationError> {
        let package = self.effective_package(canonical_skill_id, workspace_identity)?;
        self.reader.read_resource_bytes(&package, logical_path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contexts::tooling::skills::application::{
        SkillDocument, SkillLayerProvider, SkillPackageDescriptor, SkillPackageResource,
        SkillResourceDocument,
    };
    use crate::contexts::tooling::skills::domain::{
        SkillAvailability, SkillDelivery, SkillLayer, SkillMetadata, SkillOrigin, SkillTrust,
        SkillType,
    };
    use crate::contexts::tooling::skills::infrastructure::CachedEffectiveSkillCatalog;
    use crate::test_support::TempDirectory;

    struct FourLayerProvider;

    impl SkillLayerProvider for FourLayerProvider {
        fn inventory(
            &self,
            workspace_path: Option<&str>,
        ) -> Result<Vec<SkillPackageDescriptor>, SkillApplicationError> {
            Ok(vec![
                package(SkillLayer::System, None),
                package(SkillLayer::Registry, None),
                package(SkillLayer::User, None),
                package(SkillLayer::Project, workspace_path),
            ])
        }
    }

    struct LayerReader;

    impl SkillPackageReader for LayerReader {
        fn read_document(
            &self,
            package: &SkillPackageDescriptor,
        ) -> Result<SkillDocument, SkillApplicationError> {
            Ok(SkillDocument {
                metadata: package.metadata.clone(),
                body: package.layer.as_str().to_string(),
            })
        }

        fn list_resources(
            &self,
            _package: &SkillPackageDescriptor,
        ) -> Result<Vec<SkillPackageResource>, SkillApplicationError> {
            Ok(Vec::new())
        }

        fn read_resource(
            &self,
            package: &SkillPackageDescriptor,
            _relative_path: &str,
        ) -> Result<SkillResourceDocument, SkillApplicationError> {
            Err(SkillApplicationError::NotFound(
                package.metadata.id.as_str().to_string(),
            ))
        }
    }

    #[test]
    fn base_snapshot_is_built_from_the_four_layer_winner() {
        let workspace = TempDirectory::new("overlay-four-layer-base");
        let catalog = Arc::new(CachedEffectiveSkillCatalog::new(vec![Arc::new(
            FourLayerProvider,
        )]));
        let snapshots = CatalogOverlayEffectiveSnapshot::new(catalog, Arc::new(LayerReader));
        let workspace_path = workspace.path().to_string_lossy();
        let snapshot = snapshots
            .read_effective_package(
                &SkillId::parse("overlay-layered").expect("Skill id"),
                Some(&workspace_path),
            )
            .expect("effective base snapshot");

        assert_eq!(snapshot.base_layer, SkillLayer::Project);
        assert_eq!(snapshot.base_identity, "project:overlay-layered");
        assert_eq!(snapshot.instructions, "project");
        assert!(!snapshot.instruction_hash.is_empty());
        assert!(!snapshot.package_hash.is_empty());
    }

    fn package(layer: SkillLayer, workspace_path: Option<&str>) -> SkillPackageDescriptor {
        SkillPackageDescriptor {
            package_key: format!("{}:overlay-layered", layer.as_str()),
            workspace_path: workspace_path.map(str::to_string),
            metadata: SkillMetadata::with_classification(
                "overlay-layered",
                "Overlay Layered",
                "Four-layer fixture",
                "testing",
                "1.0.0",
                Vec::new(),
                Vec::new(),
                Some(SkillType::Role),
                Some(SkillDelivery::OnDemand),
            )
            .expect("metadata"),
            layer,
            origin: SkillOrigin::Created,
            trust: SkillTrust::Trusted,
            availability: SkillAvailability::Available,
            revision: format!("{}-revision", layer.as_str()),
            source_path: None,
        }
    }
}
