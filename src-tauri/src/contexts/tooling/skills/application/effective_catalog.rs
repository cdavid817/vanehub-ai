use super::{
    EffectiveSkill, SkillApplicationError, SkillPackageDescriptor, SkillPackagePreview,
    SkillPackageReader,
};
use crate::contexts::tooling::skills::domain::{SkillAvailability, SkillLayer, SkillType};
use std::collections::BTreeMap;

pub(crate) fn resolve_effective_catalog(
    workspace_path: Option<&str>,
    mut definitions: Vec<SkillPackageDescriptor>,
) -> Vec<EffectiveSkill> {
    definitions.retain(|definition| {
        definition.layer != SkillLayer::Project
            || workspace_path
                .is_some_and(|workspace| definition.workspace_path.as_deref() == Some(workspace))
    });
    definitions.sort_by(|left, right| {
        left.metadata
            .id
            .cmp(&right.metadata.id)
            .then_with(|| left.layer.rank().cmp(&right.layer.rank()))
            .then_with(|| left.package_key.cmp(&right.package_key))
    });

    let mut grouped = BTreeMap::new();
    for definition in definitions {
        grouped
            .entry(definition.metadata.id.clone())
            .or_insert_with(Vec::new)
            .push(definition);
    }
    grouped
        .into_values()
        .filter_map(resolve_canonical_group)
        .collect()
}

fn resolve_canonical_group(mut definitions: Vec<SkillPackageDescriptor>) -> Option<EffectiveSkill> {
    definitions.sort_by(|left, right| {
        left.layer
            .rank()
            .cmp(&right.layer.rank())
            .then_with(|| left.package_key.cmp(&right.package_key))
    });
    for layer in [
        SkillLayer::Project,
        SkillLayer::User,
        SkillLayer::Registry,
        SkillLayer::System,
    ] {
        let indexes = definitions
            .iter()
            .enumerate()
            .filter(|(_, definition)| {
                definition.layer == layer && definition.availability == SkillAvailability::Available
            })
            .map(|(index, _)| index)
            .collect::<Vec<_>>();
        if indexes.len() > 1 {
            for index in indexes {
                definitions[index].availability = SkillAvailability::Conflicting;
            }
            continue;
        }
        if let Some(index) = indexes.first().copied() {
            let effective = definitions.remove(index);
            return Some(EffectiveSkill {
                effective,
                shadowed: definitions,
            });
        }
    }
    definitions
        .into_iter()
        .next()
        .map(|effective| EffectiveSkill {
            effective,
            shadowed: Vec::new(),
        })
}

pub(crate) fn normalize_utility_availability(
    definition: &mut SkillPackageDescriptor,
    utility_execution_available: bool,
) {
    if definition.metadata.skill_type == SkillType::Utility
        && !utility_execution_available
        && definition.availability == SkillAvailability::Available
    {
        definition.availability = SkillAvailability::Unsupported;
    }
}

pub(crate) fn preview_package(
    package: &SkillPackageDescriptor,
    reader: &dyn SkillPackageReader,
) -> Result<SkillPackagePreview, SkillApplicationError> {
    Ok(SkillPackagePreview {
        id: package.metadata.id.clone(),
        document: reader.read_document(package)?,
        layer: package.layer,
        revision: package.revision.clone(),
        immutable: !package.layer.content_is_mutable(),
    })
}

#[cfg(test)]
fn ensure_package_content_mutable(
    package: &SkillPackageDescriptor,
) -> Result<(), SkillApplicationError> {
    if package.layer.content_is_mutable() {
        Ok(())
    } else {
        Err(SkillApplicationError::ImmutablePackage(
            package.metadata.id.as_str().to_string(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contexts::tooling::skills::application::SkillDocument;
    use crate::contexts::tooling::skills::domain::{
        SkillDelivery, SkillMetadata, SkillOrigin, SkillTrust,
    };

    fn definition(
        id: &str,
        layer: SkillLayer,
        package_key: &str,
        availability: SkillAvailability,
        workspace: Option<&str>,
    ) -> SkillPackageDescriptor {
        SkillPackageDescriptor {
            package_key: package_key.to_string(),
            workspace_path: workspace.map(str::to_string),
            metadata: SkillMetadata::with_classification(
                id,
                id,
                "Description",
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
            availability,
            revision: format!("revision-{package_key}"),
            source_path: None,
        }
    }

    struct StubReader;

    impl SkillPackageReader for StubReader {
        fn read_document(
            &self,
            package: &SkillPackageDescriptor,
        ) -> Result<SkillDocument, SkillApplicationError> {
            Ok(SkillDocument {
                metadata: package.metadata.clone(),
                body: "System instructions".to_string(),
            })
        }
    }

    #[test]
    fn precedence_is_project_user_registry_system_and_shadowing_is_ordered() {
        let catalog = resolve_effective_catalog(
            Some("D:/work"),
            vec![
                definition(
                    "developer",
                    SkillLayer::System,
                    "system",
                    SkillAvailability::Available,
                    None,
                ),
                definition(
                    "developer",
                    SkillLayer::Registry,
                    "registry",
                    SkillAvailability::Available,
                    None,
                ),
                definition(
                    "developer",
                    SkillLayer::User,
                    "user",
                    SkillAvailability::Available,
                    None,
                ),
                definition(
                    "developer",
                    SkillLayer::Project,
                    "project",
                    SkillAvailability::Available,
                    Some("D:/work"),
                ),
            ],
        );
        assert_eq!(catalog[0].effective.layer, SkillLayer::Project);
        assert_eq!(
            catalog[0]
                .shadowed
                .iter()
                .map(|definition| definition.layer)
                .collect::<Vec<_>>(),
            vec![SkillLayer::User, SkillLayer::Registry, SkillLayer::System]
        );
    }

    #[test]
    fn workspace_omission_and_invalid_higher_layer_select_the_next_valid_definition() {
        let catalog = resolve_effective_catalog(
            None,
            vec![
                definition(
                    "review",
                    SkillLayer::Project,
                    "project",
                    SkillAvailability::Available,
                    Some("D:/work"),
                ),
                definition(
                    "review",
                    SkillLayer::User,
                    "user",
                    SkillAvailability::Invalid,
                    None,
                ),
                definition(
                    "review",
                    SkillLayer::System,
                    "system",
                    SkillAvailability::Available,
                    None,
                ),
            ],
        );
        assert_eq!(catalog[0].effective.layer, SkillLayer::System);
        assert_eq!(
            catalog[0].shadowed[0].availability,
            SkillAvailability::Invalid
        );
    }

    #[test]
    fn same_layer_collision_fails_closed_and_order_is_deterministic() {
        let catalog = resolve_effective_catalog(
            None,
            vec![
                definition(
                    "review",
                    SkillLayer::User,
                    "z-user",
                    SkillAvailability::Available,
                    None,
                ),
                definition(
                    "review",
                    SkillLayer::User,
                    "a-user",
                    SkillAvailability::Available,
                    None,
                ),
                definition(
                    "review",
                    SkillLayer::System,
                    "system",
                    SkillAvailability::Available,
                    None,
                ),
            ],
        );
        assert_eq!(catalog[0].effective.layer, SkillLayer::System);
        assert_eq!(catalog[0].shadowed[0].package_key, "a-user");
        assert!(catalog[0]
            .shadowed
            .iter()
            .all(|definition| definition.availability == SkillAvailability::Conflicting));
    }

    #[test]
    fn utility_is_visible_but_unsupported_until_delegation_exists() {
        let mut utility = definition(
            "explorer",
            SkillLayer::User,
            "user",
            SkillAvailability::Available,
            None,
        );
        utility.metadata.skill_type = SkillType::Utility;
        normalize_utility_availability(&mut utility, false);
        assert_eq!(utility.availability, SkillAvailability::Unsupported);
    }

    #[test]
    fn system_package_is_previewable_but_content_mutations_are_refused() {
        let package = definition(
            "code-review",
            SkillLayer::System,
            "system:1:code-review",
            SkillAvailability::Available,
            None,
        );
        let preview = preview_package(&package, &StubReader).expect("preview");
        assert_eq!(preview.document.body, "System instructions");
        assert_eq!(preview.layer, SkillLayer::System);
        assert!(preview.immutable);
        assert_eq!(
            ensure_package_content_mutable(&package),
            Err(SkillApplicationError::ImmutablePackage(
                "code-review".to_string()
            ))
        );

        let user = definition(
            "code-review",
            SkillLayer::User,
            "user:code-review",
            SkillAvailability::Available,
            None,
        );
        assert_eq!(ensure_package_content_mutable(&user), Ok(()));
    }
}
