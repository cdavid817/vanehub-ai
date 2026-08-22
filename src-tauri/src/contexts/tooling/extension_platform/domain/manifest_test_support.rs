//! Decoded manifests for tests that need one but are not about decoding.
//!
//! Built from manifest *text* through the real parser and decoder, so a fixture cannot describe a
//! manifest the production path is unable to produce.

use super::{ExtensionManifestV1, ExtensionManifestV1Decoder, VersionedExtensionManifest};
use semver::Version;
use vanehub_bounded_yaml::parse_block;

/// The smallest complete manifest, plus whatever the caller appends.
pub(super) fn manifest(extra: &str) -> ExtensionManifestV1 {
    let text = format!(
        "\
schema_version: 1
id: acme.git-guardian
display_name: Git Guardian
publisher: acme
version: 1.2.0
min_vanehub_version: \">=0.9.0\"
{extra}"
    );
    let document = parse_block(&text, super::EXTENSION_MANIFEST_YAML_LIMITS)
        .unwrap_or_else(|error| panic!("fixture should parse: {error:?}\n---\n{text}"));
    let decoded = ExtensionManifestV1Decoder::new(Version::parse("1.0.0").expect("version"))
        .decode(&document)
        .unwrap_or_else(|error| panic!("fixture should decode: {error}\n---\n{text}"));
    match decoded {
        VersionedExtensionManifest::V1(manifest) => manifest,
    }
}

/// A WASM extension whose runtime entry is `entry`.
pub(super) fn manifest_with_runtime_entry(entry: &str) -> ExtensionManifestV1 {
    manifest(&format!(
        "\
runtime:
  kind: wasm-module
  entry: {entry}
  trust_profile: strict
"
    ))
}
