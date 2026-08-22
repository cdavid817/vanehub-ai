// No production caller yet; see `identity.rs`.
#![cfg_attr(not(test), allow(dead_code))]

//! Checks that need the whole manifest in view.
//!
//! The decoder validates each field where it sits, which is where a diagnostic is most useful. It
//! cannot see relationships: whether a mode's required hook exists, whether two declared paths are
//! the same file on a case-insensitive filesystem, whether a handler was declared for a runtime
//! that is not there. Those are answered here, once, over a decoded manifest.
//!
//! Duplicate contribution ids are absent from this list on purpose. The id is the mapping key and
//! the parser already rejects duplicate keys, so the check has no input to run on — that is the
//! reason the manifest format is keyed rather than a list of records.

use super::{
    ContributionGlobalId, ContributionKind, ContributionLocalId, ExtensionManifestV1,
    HookHandlerDeclaration, PortablePackagePath, RuntimeKind,
};
use std::collections::{BTreeMap, BTreeSet};

/// A relationship the manifest asserts and does not satisfy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct IntegrityViolation {
    /// Dotted field path, in the decoder's vocabulary, so both halves report locations the same
    /// way.
    pub(crate) field: String,
    pub(crate) reason: IntegrityReason,
    /// The offending value, already bounded by whatever validated it.
    pub(crate) value: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum IntegrityReason {
    /// A mode names a hook the manifest does not contribute.
    UnknownHookReference,
    /// A hook routes to an MCP tool whose server the manifest does not contribute.
    UnknownMcpReference,
    /// Two declared paths differ only by case, so a case-insensitive filesystem has one file where
    /// the manifest claims two.
    CaseInsensitivePathCollision { other: String },
    /// Two declared paths differ only by Unicode composition. `é` written as one code point and as
    /// `e` plus a combining accent are the same filename after normalization on macOS.
    UnicodePathCollision { other: String },
    /// A contribution declares an entry point into a runtime the manifest does not have.
    HandlerWithoutRuntime,
    /// An activation event names a tool, hook, or connector the manifest does not contribute, so
    /// nothing it declares could ever wake it.
    UnreachableActivationEvent,
}

impl IntegrityReason {
    pub(crate) const fn code(&self) -> &'static str {
        match self {
            Self::UnknownHookReference => "unknown_hook_reference",
            Self::UnknownMcpReference => "unknown_mcp_reference",
            Self::CaseInsensitivePathCollision { .. } => "case_insensitive_path_collision",
            Self::UnicodePathCollision { .. } => "unicode_path_collision",
            Self::HandlerWithoutRuntime => "handler_without_runtime",
            Self::UnreachableActivationEvent => "unreachable_activation_event",
        }
    }
}

/// Every relationship the manifest gets wrong, in a stable order.
///
/// Returns all of them rather than the first. A publisher fixing a manifest wants the list; making
/// them re-run the installer once per mistake is a worse experience for no safety gain, since
/// nothing here is executed to find the next one.
pub(crate) fn check_integrity(manifest: &ExtensionManifestV1) -> Vec<IntegrityViolation> {
    let mut violations = Vec::new();
    check_hook_references(manifest, &mut violations);
    check_mcp_references(manifest, &mut violations);
    check_handlers_against_runtime(manifest, &mut violations);
    check_activation_targets(manifest, &mut violations);
    check_path_collisions(manifest, &mut violations);
    violations
}

/// Global ids for everything the manifest contributes.
///
/// Built here rather than stored, because a contribution's global id is derived from the extension
/// id and only becomes meaningful once both are known.
pub(crate) fn global_ids(manifest: &ExtensionManifestV1) -> Vec<ContributionGlobalId> {
    manifest
        .contributes
        .declared_ids()
        .into_iter()
        .map(|(kind, local)| ContributionGlobalId::new(&manifest.id, kind, local))
        .collect()
}

fn local_ids(
    manifest: &ExtensionManifestV1,
    kind: ContributionKind,
) -> BTreeSet<&ContributionLocalId> {
    manifest
        .contributes
        .declared_ids()
        .into_iter()
        .filter(|(declared, _)| *declared == kind)
        .map(|(_, local)| local)
        .collect()
}

fn check_hook_references(manifest: &ExtensionManifestV1, violations: &mut Vec<IntegrityViolation>) {
    let hooks = local_ids(manifest, ContributionKind::Hook);
    for mode in &manifest.contributes.modes {
        for required in &mode.required_hooks {
            if !hooks.contains(required) {
                violations.push(IntegrityViolation {
                    field: format!("contributes.modes.{}.required_hooks", mode.id.as_str()),
                    reason: IntegrityReason::UnknownHookReference,
                    value: required.as_str().to_string(),
                });
            }
        }
    }
}

fn check_mcp_references(manifest: &ExtensionManifestV1, violations: &mut Vec<IntegrityViolation>) {
    let servers = local_ids(manifest, ContributionKind::Mcp);
    for hook in &manifest.contributes.hooks {
        let HookHandlerDeclaration::McpTool { tool } = &hook.handler else {
            continue;
        };
        // `<server>/<tool>`: the server half must be something this manifest contributes, since a
        // package cannot route a hook through a server it did not declare.
        let server = tool.split('/').next().unwrap_or(tool);
        let known = ContributionLocalId::parse(server)
            .is_ok_and(|parsed| servers.iter().any(|known| *known == &parsed));
        if !known {
            violations.push(IntegrityViolation {
                field: format!("contributes.hooks.{}.handler.tool", hook.id.as_str()),
                reason: IntegrityReason::UnknownMcpReference,
                value: tool.clone(),
            });
        }
    }
}

fn check_handlers_against_runtime(
    manifest: &ExtensionManifestV1,
    violations: &mut Vec<IntegrityViolation>,
) {
    if manifest.runtime.kind != RuntimeKind::None {
        return;
    }
    // A data-only extension has nothing to call. A tool or runtime hook declaring an entry point
    // would be installed, indexed, listed to the model, and then fail at the first invocation.
    for tool in &manifest.contributes.tools {
        violations.push(IntegrityViolation {
            field: format!("contributes.tools.{}.handler", tool.id.as_str()),
            reason: IntegrityReason::HandlerWithoutRuntime,
            value: tool.handler.clone(),
        });
    }
    for hook in &manifest.contributes.hooks {
        if let HookHandlerDeclaration::ExtensionRuntime { entry } = &hook.handler {
            violations.push(IntegrityViolation {
                field: format!("contributes.hooks.{}.handler.entry", hook.id.as_str()),
                reason: IntegrityReason::HandlerWithoutRuntime,
                value: entry.clone(),
            });
        }
    }
    for transform in &manifest.contributes.transforms {
        violations.push(IntegrityViolation {
            field: format!("contributes.transforms.{}.handler", transform.id.as_str()),
            reason: IntegrityReason::HandlerWithoutRuntime,
            value: transform.handler.clone(),
        });
    }
}

fn check_activation_targets(
    manifest: &ExtensionManifestV1,
    violations: &mut Vec<IntegrityViolation>,
) {
    use super::ActivationEvent;

    let tools = local_ids(manifest, ContributionKind::Tool);
    let connectors = local_ids(manifest, ContributionKind::Connector);

    for event in &manifest.activation_events {
        // Only events naming something this manifest owns are checkable. `onHook:<event-id>` and
        // `onAgentMode:<mode-id>` name host concepts, and `onCommand:` names a command the host
        // registers, so those are resolved against the application, not here.
        let (target, known) = match event {
            ActivationEvent::Tool(target) => (target, &tools),
            ActivationEvent::Connector(target) => (target, &connectors),
            _ => continue,
        };
        let declared = ContributionLocalId::parse(target.as_str())
            .is_ok_and(|parsed| known.iter().any(|item| *item == &parsed));
        if !declared {
            violations.push(IntegrityViolation {
                field: "activation_events".to_string(),
                reason: IntegrityReason::UnreachableActivationEvent,
                value: event.to_manifest_value(),
            });
        }
    }
}

fn check_path_collisions(manifest: &ExtensionManifestV1, violations: &mut Vec<IntegrityViolation>) {
    let mut declared: Vec<&PortablePackagePath> = manifest.contributes.declared_paths();
    if let Some(entry) = manifest.runtime.entry.as_ref() {
        declared.push(entry);
    }

    // Two passes over two different normalizations. A pair can collide under one and not the
    // other, and telling a publisher which one it is decides whether they rename a file or
    // re-type its accents.
    let mut by_case: BTreeMap<String, &PortablePackagePath> = BTreeMap::new();
    let mut by_composition: BTreeMap<String, &PortablePackagePath> = BTreeMap::new();

    for path in declared {
        if let Some(other) = by_case.insert(path.case_folded(), path) {
            if other.as_str() != path.as_str() {
                violations.push(IntegrityViolation {
                    field: "contributes".to_string(),
                    reason: IntegrityReason::CaseInsensitivePathCollision {
                        other: other.as_str().to_string(),
                    },
                    value: path.as_str().to_string(),
                });
            }
        }
        if let Some(other) = by_composition.insert(path.composition_folded(), path) {
            if other.as_str() != path.as_str() && other.case_folded() != path.case_folded() {
                violations.push(IntegrityViolation {
                    field: "contributes".to_string(),
                    reason: IntegrityReason::UnicodePathCollision {
                        other: other.as_str().to_string(),
                    },
                    value: path.as_str().to_string(),
                });
            }
        }
    }
}
