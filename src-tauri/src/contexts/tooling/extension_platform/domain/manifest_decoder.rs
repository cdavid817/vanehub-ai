// Landed with its consumer in the same commit; the contributions half lives in
// `manifest_decoder_contributions.rs`.
#![cfg_attr(not(test), allow(dead_code))]

//! `BoundedYamlValue` to `VersionedExtensionManifest`.
//!
//! The second half of the two-stage pipeline. The first stage asked "is this well-formed YAML
//! within our limits?" and knew nothing about extensions; this one asks "does it describe a valid
//! extension?" and never touches bytes or indentation. Keeping them apart is what stops a manifest
//! rule change from quietly moving a resource bound.
//!
//! Reading is explicit, field by field, and `MappingReader::finish` refuses anything left over. A
//! key this build does not read is a key the author wrote intending something; for a
//! security-relevant field, ignoring it is the difference between refusing a package and running
//! it with intent nobody reviewed.

use super::decode_reader::{bound, MappingReader};
use super::{
    identifier_at, origin_at, path_at, ActivationEvent, CapabilityRequest, DecodeReason,
    ExtensionDependency, ExtensionId, ExtensionManifestV1, ExtensionRequirements,
    ManifestDecodeError, NetworkOrigin, PortablePackagePath, PublisherId, RuntimeDeclaration,
    RuntimeKind, SkillDependency, TrustProfile, VersionedExtensionManifest,
    SUPPORTED_SCHEMA_VERSIONS,
};
use semver::{Version, VersionReq};
use vanehub_bounded_yaml::{BoundedYamlLimits, BoundedYamlValue};

/// The manifest's own parser profile.
///
/// Separate from Skills' by construction, per `design.md`. A manifest describes more than a Skill
/// config so it gets more room — and because the profile is a parameter, taking that room here
/// cannot widen what a Skill config may contain.
pub(crate) const EXTENSION_MANIFEST_YAML_LIMITS: BoundedYamlLimits = BoundedYamlLimits {
    max_bytes: 64 * 1_024,
    max_depth: 8,
    max_nodes: 2_048,
    max_key_bytes: 128,
    max_scalar_characters: 1_024,
    max_sequence_items: 64,
};

pub(crate) const MAX_ACTIVATION_EVENTS: usize = 32;
pub(crate) const MAX_DEPENDENCIES: usize = 32;
pub(crate) const MAX_CAPABILITY_ENTRIES: usize = 32;
pub(crate) const MAX_CONTRIBUTIONS_PER_KIND: usize = 64;

/// Decodes a parsed manifest against the version of the application that will run it.
///
/// The application version is a constructor parameter rather than a global so that compatibility
/// is testable without a running app, and so that "which build was this checked against?" has one
/// answer per decoder.
pub(crate) struct ExtensionManifestV1Decoder {
    application_version: Version,
}

impl ExtensionManifestV1Decoder {
    pub(crate) fn new(application_version: Version) -> Self {
        Self {
            application_version,
        }
    }

    pub(crate) fn decode(
        &self,
        document: &BoundedYamlValue,
    ) -> Result<VersionedExtensionManifest, ManifestDecodeError> {
        let mut root = MappingReader::open(String::new(), document)?;

        // Schema version first: everything below is read according to it, so a version this build
        // does not implement must stop the decode rather than be discovered halfway through.
        let declared = parse_schema_version(&mut root)?;
        if !SUPPORTED_SCHEMA_VERSIONS.contains(&declared) {
            return Err(ManifestDecodeError::new(
                "schema_version",
                DecodeReason::UnsupportedSchemaVersion { declared },
            ));
        }

        let id_text = root.required_scalar("id")?;
        let id = ExtensionId::parse(id_text).map_err(|error| identifier_at("id", &error))?;

        let publisher_text = root.required_scalar("publisher")?;
        let publisher = PublisherId::parse(publisher_text)
            .map_err(|error| identifier_at("publisher", &error))?;

        let version = parse_version(&mut root, "version")?;
        let min_vanehub_version = parse_version_requirement(&mut root, "min_vanehub_version")?;
        if !min_vanehub_version.matches(&self.application_version) {
            return Err(ManifestDecodeError::new(
                "min_vanehub_version",
                DecodeReason::IncompatibleApplicationVersion {
                    required: min_vanehub_version.to_string(),
                    running: self.application_version.to_string(),
                },
            ));
        }

        let display_name = root.required_scalar("display_name")?.to_string();
        let description = root.optional_scalar("description")?.map(str::to_string);
        let license = root.optional_scalar("license")?.map(str::to_string);

        let runtime = decode_runtime(&mut root)?;
        let activation_events = decode_activation_events(&mut root)?;
        let requires = decode_requirements(&mut root)?;
        let permissions = decode_permissions(&mut root)?;
        let contributes = super::manifest_decoder_contributions::decode_contributions(&mut root)?;

        root.finish()?;

        Ok(VersionedExtensionManifest::V1(ExtensionManifestV1 {
            id,
            display_name,
            publisher,
            version,
            description,
            license,
            min_vanehub_version,
            runtime,
            activation_events,
            requires,
            permissions,
            contributes,
        }))
    }
}

fn parse_schema_version(root: &mut MappingReader<'_>) -> Result<u32, ManifestDecodeError> {
    let text = root.required_scalar("schema_version")?;
    text.parse::<u32>().map_err(|_| {
        // Not `UnsupportedSchemaVersion`: that claims a number was read. This is "not a number".
        ManifestDecodeError::new("schema_version", DecodeReason::ExpectedScalar)
    })
}

fn parse_version(
    reader: &mut MappingReader<'_>,
    field: &str,
) -> Result<Version, ManifestDecodeError> {
    let path = reader.child_path(field);
    let text = reader.required_scalar(field)?;
    Version::parse(text).map_err(|_| ManifestDecodeError::new(path, DecodeReason::InvalidVersion))
}

fn parse_version_requirement(
    reader: &mut MappingReader<'_>,
    field: &str,
) -> Result<VersionReq, ManifestDecodeError> {
    let path = reader.child_path(field);
    let text = reader.required_scalar(field)?;
    VersionReq::parse(text)
        .map_err(|_| ManifestDecodeError::new(path, DecodeReason::InvalidVersionRequirement))
}

fn decode_runtime(root: &mut MappingReader<'_>) -> Result<RuntimeDeclaration, ManifestDecodeError> {
    let path = root.child_path("runtime");
    let Some(value) = root.optional_value("runtime") else {
        // No runtime is a data-only extension: contributions with nothing to activate.
        return Ok(RuntimeDeclaration {
            kind: RuntimeKind::None,
            entry: None,
            trust_profile: TrustProfile::Strict,
        });
    };
    let mut reader = MappingReader::open(path.clone(), value)?;

    let kind_text = reader.required_scalar("kind")?;
    let kind = RuntimeKind::parse(kind_text).ok_or_else(|| {
        ManifestDecodeError::new(
            reader.child_path("kind"),
            DecodeReason::UnknownValue {
                expected: "wasm-module, sidecar, none",
            },
        )
    })?;
    if !kind.is_selectable_by_external_package() {
        return Err(ManifestDecodeError::new(
            reader.child_path("kind"),
            DecodeReason::NotPermitted {
                detail: "names a runtime reserved for reviewed built-in extensions",
            },
        ));
    }
    if kind == RuntimeKind::WasmComponentReserved {
        // Named rather than treated as a malformed module: the pinned engine has no
        // component-model support, and an author deserves to know that is why.
        return Err(ManifestDecodeError::new(
            reader.child_path("kind"),
            DecodeReason::NotPermitted {
                detail: "names the WebAssembly component model, which this build does not \
                         implement; use wasm-module",
            },
        ));
    }

    let entry_path = reader.child_path("entry");
    let entry = match reader.optional_scalar("entry")? {
        Some(text) => {
            Some(PortablePackagePath::parse(text).map_err(|error| path_at(&entry_path, &error))?)
        }
        None => None,
    };
    if kind.requires_entry() && entry.is_none() {
        return Err(ManifestDecodeError::new(entry_path, DecodeReason::Missing));
    }
    if !kind.requires_entry() && entry.is_some() {
        return Err(ManifestDecodeError::new(
            entry_path,
            DecodeReason::NotPermitted {
                detail: "declares an entry point for a runtime that has none",
            },
        ));
    }

    let trust_path = reader.child_path("trust_profile");
    let trust_profile = match reader.optional_scalar("trust_profile")? {
        Some(text) => TrustProfile::parse(text).ok_or_else(|| {
            ManifestDecodeError::new(
                trust_path,
                DecodeReason::UnknownValue {
                    expected: "strict, standard, trusted",
                },
            )
        })?,
        // Absent means the tightest profile. A default that granted more than the author asked
        // for would be an authority increase nobody wrote down.
        None => TrustProfile::Strict,
    };

    reader.finish()?;
    Ok(RuntimeDeclaration {
        kind,
        entry,
        trust_profile,
    })
}

fn decode_activation_events(
    root: &mut MappingReader<'_>,
) -> Result<Vec<ActivationEvent>, ManifestDecodeError> {
    let path = root.child_path("activation_events");
    let raw = bound(
        &path,
        root.scalar_sequence("activation_events")?,
        MAX_ACTIVATION_EVENTS,
    )?;
    raw.into_iter()
        .map(|text| ActivationEvent::parse(text).map_err(|error| identifier_at(&path, &error)))
        .collect()
}

fn decode_requirements(
    root: &mut MappingReader<'_>,
) -> Result<ExtensionRequirements, ManifestDecodeError> {
    let path = root.child_path("requires");
    let Some(value) = root.optional_value("requires") else {
        return Ok(ExtensionRequirements::default());
    };
    let mut reader = MappingReader::open(path, value)?;

    let extensions_path = reader.child_path("extensions");
    let mut extensions = Vec::new();
    for (id_text, entry) in reader.keyed_collection("extensions")? {
        let field = format!("{extensions_path}.{id_text}");
        let id = ExtensionId::parse(id_text).map_err(|error| identifier_at(&field, &error))?;
        let mut entry_reader = MappingReader::open(field, entry)?;
        let version = parse_version_requirement(&mut entry_reader, "version")?;
        let optional = decode_optional_flag(&mut entry_reader)?;
        entry_reader.finish()?;
        extensions.push(ExtensionDependency {
            id,
            version,
            optional,
        });
    }
    let extensions = bound(&extensions_path, extensions, MAX_DEPENDENCIES)?;

    let skills_path = reader.child_path("skills");
    let mut skills = Vec::new();
    for (id_text, entry) in reader.keyed_collection("skills")? {
        let field = format!("{skills_path}.{id_text}");
        let mut entry_reader = MappingReader::open(field, entry)?;
        let version = parse_version_requirement(&mut entry_reader, "version")?;
        let optional = decode_optional_flag(&mut entry_reader)?;
        entry_reader.finish()?;
        skills.push(SkillDependency {
            // Skill ids belong to the Skills context; this one only carries the text through.
            id: id_text.to_string(),
            version,
            optional,
        });
    }
    let skills = bound(&skills_path, skills, MAX_DEPENDENCIES)?;

    reader.finish()?;
    Ok(ExtensionRequirements { extensions, skills })
}

/// `optional: true|false`, defaulting to required.
///
/// A dependency whose absence blocks activation is the safe reading of silence: the alternative
/// silently ships an extension with a piece missing.
fn decode_optional_flag(reader: &mut MappingReader<'_>) -> Result<bool, ManifestDecodeError> {
    let path = reader.child_path("optional");
    match reader.optional_scalar("optional")? {
        Some("true") => Ok(true),
        Some("false") | None => Ok(false),
        Some(_) => Err(ManifestDecodeError::new(
            path,
            DecodeReason::UnknownValue {
                expected: "true, false",
            },
        )),
    }
}

fn decode_permissions(
    root: &mut MappingReader<'_>,
) -> Result<CapabilityRequest, ManifestDecodeError> {
    let path = root.child_path("permissions");
    let Some(value) = root.optional_value("permissions") else {
        return Ok(CapabilityRequest::default());
    };
    let mut reader = MappingReader::open(path, value)?;

    let (filesystem_read, filesystem_write) = match reader.optional_value("filesystem") {
        Some(filesystem) => {
            let filesystem_path = reader.child_path("filesystem");
            let mut filesystem_reader = MappingReader::open(filesystem_path, filesystem)?;
            let read_path = filesystem_reader.child_path("read");
            let read = bound(
                &read_path,
                filesystem_reader.scalar_sequence("read")?,
                MAX_CAPABILITY_ENTRIES,
            )?;
            let write_path = filesystem_reader.child_path("write");
            let write = bound(
                &write_path,
                filesystem_reader.scalar_sequence("write")?,
                MAX_CAPABILITY_ENTRIES,
            )?;
            filesystem_reader.finish()?;
            (owned(read), owned(write))
        }
        None => (Vec::new(), Vec::new()),
    };

    let network_origins = match reader.optional_value("network") {
        Some(network) => {
            let network_path = reader.child_path("network");
            let mut network_reader = MappingReader::open(network_path, network)?;
            let origins_path = network_reader.child_path("origins");
            let raw = bound(
                &origins_path,
                network_reader.scalar_sequence("origins")?,
                MAX_CAPABILITY_ENTRIES,
            )?;
            network_reader.finish()?;
            raw.into_iter()
                .map(|text| {
                    NetworkOrigin::parse(text).map_err(|error| origin_at(&origins_path, &error))
                })
                .collect::<Result<Vec<_>, _>>()?
        }
        None => Vec::new(),
    };

    let process_path = reader.child_path("process");
    let process_commands = owned(bound(
        &process_path,
        reader.scalar_sequence("process")?,
        MAX_CAPABILITY_ENTRIES,
    )?);

    let secrets_path = reader.child_path("secrets");
    let secret_ids = owned(bound(
        &secrets_path,
        reader.scalar_sequence("secrets")?,
        MAX_CAPABILITY_ENTRIES,
    )?);

    reader.finish()?;
    Ok(CapabilityRequest {
        filesystem_read,
        filesystem_write,
        network_origins,
        process_commands,
        secret_ids,
    })
}

fn owned(values: Vec<&str>) -> Vec<String> {
    values.into_iter().map(str::to_string).collect()
}
