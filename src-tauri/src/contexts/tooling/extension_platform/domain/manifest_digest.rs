// No production caller yet; see `identity.rs`.
#![cfg_attr(not(test), allow(dead_code))]

//! A canonical digest of a decoded manifest.
//!
//! The install witness binds a confirmation to what was previewed. That needs an identity which
//! changes when the manifest *means* something different and does not change when it is merely
//! written differently — reindenting a file or reordering two tools must not invalidate a preview
//! the user is halfway through approving, and adding a network origin must.
//!
//! So the digest is taken over the decoded manifest, not the source bytes. Two rules make that
//! safe:
//!
//! * **Unambiguous encoding.** Every string is length-prefixed. Concatenating fields without that
//!   would let `["ab", "c"]` and `["a", "bc"]` produce identical bytes, and two different
//!   manifests sharing a digest is precisely the failure a witness exists to prevent.
//! * **Sets are sorted, sequences are not.** Source order is meaningless for a set of origins and
//!   load-bearing for a command line. Sorting the second would make `--force` and `--dry-run`
//!   interchangeable.

use super::{
    ContributionManifest, HookHandlerDeclaration, McpTransportDeclaration,
    VersionedExtensionManifest,
};
use sha2::{Digest, Sha256};

/// SHA-256 over the canonical encoding, lower-case hex.
///
/// Only ever produced by `manifest_digest`; `parse` exists to read one back from storage.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct ManifestDigest(String);

impl ManifestDigest {
    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }

    pub(crate) fn parse(value: &str) -> Option<Self> {
        let valid = value.len() == 64
            && value
                .chars()
                .all(|character| character.is_ascii_digit() || ('a'..='f').contains(&character));
        valid.then(|| Self(value.to_string()))
    }
}

pub(crate) fn manifest_digest(manifest: &VersionedExtensionManifest) -> ManifestDigest {
    let mut canonical = Canonical::default();
    match manifest {
        VersionedExtensionManifest::V1(inner) => {
            canonical.tag("v1");
            write_v1(&mut canonical, inner);
        }
    }
    ManifestDigest(hex(&Sha256::digest(&canonical.buffer)))
}

fn write_v1(canonical: &mut Canonical, manifest: &super::ExtensionManifestV1) {
    canonical.tag("id");
    canonical.text(manifest.id.as_str());
    canonical.tag("display_name");
    canonical.text(&manifest.display_name);
    canonical.tag("publisher");
    canonical.text(manifest.publisher.as_str());
    canonical.tag("version");
    canonical.text(&manifest.version.to_string());
    canonical.tag("description");
    canonical.optional(manifest.description.as_deref());
    canonical.tag("license");
    canonical.optional(manifest.license.as_deref());
    canonical.tag("min_vanehub_version");
    canonical.text(&manifest.min_vanehub_version.to_string());

    canonical.tag("runtime");
    canonical.text(manifest.runtime.kind.as_str());
    canonical.optional(manifest.runtime.entry.as_ref().map(|entry| entry.as_str()));
    canonical.text(manifest.runtime.trust_profile.as_str());

    // A set of triggers: which ones, not in what order.
    canonical.tag("activation_events");
    canonical.set(
        manifest
            .activation_events
            .iter()
            .map(super::ActivationEvent::to_manifest_value),
    );

    canonical.tag("requires.extensions");
    canonical.set(manifest.requires.extensions.iter().map(|dependency| {
        join(&[
            dependency.id.as_str(),
            &dependency.version.to_string(),
            bool_text(dependency.optional),
        ])
    }));
    canonical.tag("requires.skills");
    canonical.set(manifest.requires.skills.iter().map(|dependency| {
        join(&[
            &dependency.id,
            &dependency.version.to_string(),
            bool_text(dependency.optional),
        ])
    }));

    let permissions = &manifest.permissions;
    canonical.tag("permissions.filesystem.read");
    canonical.set(permissions.filesystem_read.iter().cloned());
    canonical.tag("permissions.filesystem.write");
    canonical.set(permissions.filesystem_write.iter().cloned());
    canonical.tag("permissions.network.origins");
    canonical.set(
        permissions
            .network_origins
            .iter()
            .map(|origin| origin.as_str().to_string()),
    );
    canonical.tag("permissions.process");
    canonical.set(permissions.process_commands.iter().cloned());
    canonical.tag("permissions.secrets");
    canonical.set(permissions.secret_ids.iter().cloned());

    write_contributions(canonical, &manifest.contributes);
}

fn write_contributions(canonical: &mut Canonical, contributes: &ContributionManifest) {
    canonical.tag("contributes.tools");
    canonical.set(contributes.tools.iter().map(|tool| {
        join(&[
            tool.id.as_str(),
            &tool.display_name,
            tool.description.as_deref().unwrap_or_default(),
            tool.input_schema
                .as_ref()
                .map_or("", super::PortablePackagePath::as_str),
            tool.output_schema
                .as_ref()
                .map_or("", super::PortablePackagePath::as_str),
            &tool.handler,
        ])
    }));

    canonical.tag("contributes.skills");
    canonical.set(
        contributes
            .skills
            .iter()
            .map(|skill| join(&[skill.id.as_str(), skill.path.as_str()])),
    );

    canonical.tag("contributes.mcp_definitions");
    canonical.set(contributes.mcp_definitions.iter().map(|definition| {
        let transport = match &definition.transport {
            McpTransportDeclaration::Stdio {
                command,
                args,
                env_keys,
            } => join(&[
                "stdio",
                command,
                // Order-significant: a command line is not a set.
                &sequence(args),
                &sorted(env_keys),
            ]),
            McpTransportDeclaration::Http { url, header_keys } => {
                join(&["http", url, &sorted(header_keys)])
            }
        };
        join(&[definition.id.as_str(), &definition.display_name, &transport])
    }));

    canonical.tag("contributes.modes");
    canonical.set(contributes.modes.iter().map(|mode| {
        join(&[
            mode.id.as_str(),
            &mode.display_name,
            &mode.strategy,
            mode.default_policy_template.as_deref().unwrap_or_default(),
            &sorted(&mode.required_tool_groups),
            &sorted(&mode.required_skills),
            &sorted(
                &mode
                    .required_hooks
                    .iter()
                    .map(|hook| hook.as_str().to_string())
                    .collect::<Vec<_>>(),
            ),
        ])
    }));

    canonical.tag("contributes.hooks");
    canonical.set(contributes.hooks.iter().map(|hook| {
        let handler = match &hook.handler {
            HookHandlerDeclaration::ExtensionRuntime { entry } => {
                join(&["extension-runtime", entry])
            }
            HookHandlerDeclaration::McpTool { tool } => join(&["mcp-tool", tool]),
        };
        join(&[
            hook.id.as_str(),
            &hook.event,
            &matcher_text(&hook.matcher),
            &handler,
            hook.failure_mode.as_str(),
            &hook.priority.to_string(),
        ])
    }));

    canonical.tag("contributes.authorization_rules");
    canonical.set(contributes.authorization_rules.iter().map(|rule| {
        join(&[
            rule.id.as_str(),
            &rule.operation,
            &matcher_text(&rule.matcher),
            rule.effect.as_str(),
            &rule.risk,
            &sorted(&rule.allowed_scopes),
        ])
    }));

    canonical.tag("contributes.connectors");
    canonical.set(contributes.connectors.iter().map(|connector| {
        join(&[
            connector.id.as_str(),
            &connector.display_name,
            &connector.connector_type,
            &connector.driver,
            &connector.auth_strategy,
            &sorted(&connector.capabilities),
        ])
    }));

    canonical.tag("contributes.configuration");
    canonical.set(
        contributes
            .configuration
            .iter()
            .map(|entry| join(&[entry.id.as_str(), entry.schema.as_str()])),
    );

    canonical.tag("contributes.transforms");
    canonical.set(
        contributes
            .transforms
            .iter()
            .map(|transform| join(&[transform.id.as_str(), &transform.event, &transform.handler])),
    );
}

/// Predicate name to values. The predicate set is unordered and each value list is a set too:
/// a matcher listing two tool ids means the same thing in either order.
fn matcher_text(matcher: &[(String, Vec<String>)]) -> String {
    sorted(
        &matcher
            .iter()
            .map(|(name, values)| join(&[name, &sorted(values)]))
            .collect::<Vec<_>>(),
    )
}

#[derive(Default)]
struct Canonical {
    buffer: Vec<u8>,
}

impl Canonical {
    /// A field marker. Length-prefixed like everything else so a tag cannot be confused with the
    /// value before it.
    fn tag(&mut self, tag: &str) {
        self.text(tag);
    }

    fn text(&mut self, value: &str) {
        self.buffer
            .extend_from_slice(value.len().to_string().as_bytes());
        self.buffer.push(b':');
        self.buffer.extend_from_slice(value.as_bytes());
        self.buffer.push(b';');
    }

    /// Absent and present-but-empty are different states, so they encode differently.
    fn optional(&mut self, value: Option<&str>) {
        match value {
            Some(value) => {
                self.text("some");
                self.text(value);
            }
            None => self.text("none"),
        }
    }

    /// Order-insensitive. Sorted after rendering so the encoding, not the source, decides.
    fn set<I: IntoIterator<Item = String>>(&mut self, items: I) {
        let mut rendered: Vec<String> = items.into_iter().collect();
        rendered.sort_unstable();
        self.text(&rendered.len().to_string());
        for item in rendered {
            self.text(&item);
        }
    }
}

/// Joins parts unambiguously, for a value that is itself a record inside a set.
fn join(parts: &[&str]) -> String {
    let mut joined = String::new();
    for part in parts {
        joined.push_str(&part.len().to_string());
        joined.push(':');
        joined.push_str(part);
        joined.push(';');
    }
    joined
}

/// Order-significant rendering.
fn sequence(items: &[String]) -> String {
    join(&items.iter().map(String::as_str).collect::<Vec<_>>())
}

/// Order-insensitive rendering.
fn sorted(items: &[String]) -> String {
    let mut ordered: Vec<&str> = items.iter().map(String::as_str).collect();
    ordered.sort_unstable();
    join(&ordered)
}

const fn bool_text(value: bool) -> &'static str {
    if value {
        "true"
    } else {
        "false"
    }
}

fn hex(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>()
}
