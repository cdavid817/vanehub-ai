#![cfg_attr(not(test), allow(dead_code))]

use super::{validate_overlay_path, OverlayFile, OverlayMutationState, OverlayScope, SkillLayer};
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct BaseSkillResource {
    pub(crate) logical_path: String,
    pub(crate) media_type: String,
    pub(crate) size_bytes: u64,
    pub(crate) content_hash: String,
    pub(crate) source_layer: SkillLayer,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum EffectiveResourceSource {
    Base {
        layer: SkillLayer,
    },
    Overlay {
        scope: OverlayScope,
        workspace_identity: Option<String>,
        mutation_id: String,
        payload_ref: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ResourceShadowSummary {
    pub(crate) source: EffectiveResourceSource,
    pub(crate) media_type: String,
    pub(crate) size_bytes: u64,
    pub(crate) content_hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct EffectiveSkillResource {
    pub(crate) logical_path: String,
    pub(crate) media_type: String,
    pub(crate) size_bytes: u64,
    pub(crate) content_hash: String,
    pub(crate) source: EffectiveResourceSource,
    pub(crate) shadowed: Vec<ResourceShadowSummary>,
    pub(crate) shadowed_truncated: bool,
}

#[derive(Clone, Copy)]
pub(crate) struct ScopedOverlayFiles<'a> {
    scope: OverlayScope,
    workspace_identity: Option<&'a str>,
    files: &'a [OverlayFile],
}

impl<'a> ScopedOverlayFiles<'a> {
    pub(crate) fn new(
        scope: OverlayScope,
        workspace_identity: Option<&'a str>,
        files: &'a [OverlayFile],
    ) -> Self {
        Self {
            scope,
            workspace_identity,
            files,
        }
    }

    fn applies_to(self, active_workspace: Option<&str>) -> bool {
        self.scope != OverlayScope::Project
            || active_workspace.is_some_and(|workspace| self.workspace_identity == Some(workspace))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct OverlayResourceReplay {
    resources: Vec<EffectiveSkillResource>,
}

impl OverlayResourceReplay {
    pub(crate) fn resources(&self) -> &[EffectiveSkillResource] {
        &self.resources
    }

    pub(crate) fn entry(&self, logical_path: &str) -> Option<&EffectiveSkillResource> {
        self.resources
            .binary_search_by(|resource| resource.logical_path.as_str().cmp(logical_path))
            .ok()
            .map(|index| &self.resources[index])
    }
}

pub(crate) fn merge_overlay_resources(
    base_resources: &[BaseSkillResource],
    scoped_files: &[ScopedOverlayFiles<'_>],
    active_workspace: Option<&str>,
    maximum_shadow_summaries: usize,
) -> OverlayResourceReplay {
    let resources = base_resources
        .iter()
        .map(|resource| {
            (
                resource.logical_path.clone(),
                EffectiveSkillResource {
                    logical_path: resource.logical_path.clone(),
                    media_type: resource.media_type.clone(),
                    size_bytes: resource.size_bytes,
                    content_hash: resource.content_hash.clone(),
                    source: EffectiveResourceSource::Base {
                        layer: resource.source_layer,
                    },
                    shadowed: Vec::new(),
                    shadowed_truncated: false,
                },
            )
        })
        .collect::<BTreeMap<_, _>>()
        .into_values()
        .collect::<Vec<_>>();

    let mut ordered_scopes = scoped_files
        .iter()
        .copied()
        .filter(|scope| scope.applies_to(active_workspace))
        .collect::<Vec<_>>();
    ordered_scopes.sort_by_key(|scope| scope.scope);
    let mut replay = OverlayResourceReplay { resources };
    for scope in ordered_scopes {
        replay = apply_overlay_resource_scope(
            replay.resources(),
            scope.scope,
            scope.workspace_identity,
            scope.files,
            maximum_shadow_summaries,
        );
    }
    replay
}

pub(crate) fn apply_overlay_resource_scope(
    current_resources: &[EffectiveSkillResource],
    scope: OverlayScope,
    workspace_identity: Option<&str>,
    files: &[OverlayFile],
    maximum_shadow_summaries: usize,
) -> OverlayResourceReplay {
    let mut resources = current_resources
        .iter()
        .cloned()
        .map(|resource| (resource.logical_path.clone(), resource))
        .collect::<BTreeMap<_, _>>();
    for file in files {
        if file.state() != OverlayMutationState::Active {
            continue;
        }
        // Assembly replays *persisted* Overlay rows, which may predate a validation rule or have
        // been written straight into storage. Re-checking the path here means a row that should
        // never have existed cannot shadow a base resource — in particular a Skill tool manifest,
        // module, or hash — just because it survived long enough to be read back.
        if validate_overlay_path(&file.logical_path).is_err() {
            continue;
        }
        let previous = resources.remove(&file.logical_path);
        let (shadowed, shadowed_truncated) =
            bounded_shadows(previous.as_ref(), maximum_shadow_summaries);
        resources.insert(
            file.logical_path.clone(),
            EffectiveSkillResource {
                logical_path: file.logical_path.clone(),
                media_type: file.media_type.clone(),
                size_bytes: file.size,
                content_hash: file.content_hash.clone(),
                source: EffectiveResourceSource::Overlay {
                    scope,
                    workspace_identity: workspace_identity.map(str::to_string),
                    mutation_id: file.id.clone(),
                    payload_ref: file.payload_ref.clone(),
                },
                shadowed,
                shadowed_truncated,
            },
        );
    }
    OverlayResourceReplay {
        resources: resources.into_values().collect(),
    }
}

fn bounded_shadows(
    previous: Option<&EffectiveSkillResource>,
    maximum: usize,
) -> (Vec<ResourceShadowSummary>, bool) {
    let Some(previous) = previous else {
        return (Vec::new(), false);
    };
    let mut shadowed = Vec::with_capacity(previous.shadowed.len().saturating_add(1));
    shadowed.push(ResourceShadowSummary {
        source: previous.source.clone(),
        media_type: previous.media_type.clone(),
        size_bytes: previous.size_bytes,
        content_hash: previous.content_hash.clone(),
    });
    shadowed.extend(previous.shadowed.iter().cloned());
    let shadowed_truncated = previous.shadowed_truncated || shadowed.len() > maximum;
    shadowed.truncate(maximum);
    (shadowed, shadowed_truncated)
}
