// No production caller yet; see `identity.rs`.
#![cfg_attr(not(test), allow(dead_code))]

//! The detached signature that travels next to a `.vhext` file.
//!
//! Everything else in this context is parsed *after* a signature is checked. The envelope cannot
//! be: it is what says which key to check against. So it is the one structure read from untrusted
//! bytes before anything has been verified, and it is deliberately tiny — nine scalar fields, a
//! 4 KiB ceiling, and the same bounded parser and unknown-field rejection the manifest uses. A
//! second hand-written parser for pre-verification input is exactly the surface not to add.
//!
//! ```text
//! envelope_version: 1
//! algorithm: ed25519
//! publisher: acme
//! extension: acme.linter
//! version: 1.4.2
//! package_sha256: <64 hex>
//! package_bytes: 1048576
//! manifest_sha256: <64 hex>
//! key_fingerprint: <64 hex>
//! signature: <base64 of 64 bytes>
//! ```
//!
//! `manifest_sha256` is a *claim*. The signature proves the publisher made it; only extraction can
//! prove the package honors it, which is why a verified signature is not yet a confirmed one.

use super::decode_reader::MappingReader;
use super::{
    identifier_at, DecodeReason, ExtensionId, ManifestDecodeError, ManifestDigest, PackageHash,
    PublisherId, PublisherKeyFingerprint,
};
use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use semver::Version;
use vanehub_bounded_yaml::{parse_block, BoundedYamlLimits};

/// An Ed25519 signature is 64 bytes. There is no other length.
pub(crate) const SIGNATURE_BYTES: usize = 64;

/// The only envelope shape this build understands.
pub(crate) const SUPPORTED_ENVELOPE_VERSION: u32 = 1;

/// Deliberately far tighter than the manifest's. An envelope is ten short lines; anything that
/// needs depth, sequences, or kilobytes of scalar is not one.
pub(crate) const ENVELOPE_YAML_LIMITS: BoundedYamlLimits = BoundedYamlLimits {
    max_bytes: 4 * 1024,
    max_depth: 1,
    max_nodes: 32,
    max_key_bytes: 32,
    max_scalar_characters: 256,
    max_sequence_items: 0,
    // `1.4.2` is a version, and `acme.linter` is an id: both are ordinary scalars, but a dotted
    // *key* has no meaning in an envelope.
    allow_dotted_keys: false,
};

/// Which signature scheme an envelope declares.
///
/// One variant, and an enum anyway: the alternative is a bare string compared in three places,
/// which is how a build ends up accepting an algorithm it cannot verify.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SignatureAlgorithm {
    Ed25519,
}

impl SignatureAlgorithm {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::Ed25519 => "ed25519",
        }
    }

    fn parse(value: &str) -> Option<Self> {
        match value {
            "ed25519" => Some(Self::Ed25519),
            _ => None,
        }
    }
}

/// The raw signature bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PackageSignature([u8; SIGNATURE_BYTES]);

impl PackageSignature {
    pub(crate) const fn from_bytes(bytes: [u8; SIGNATURE_BYTES]) -> Self {
        Self(bytes)
    }

    pub(crate) const fn as_bytes(&self) -> &[u8; SIGNATURE_BYTES] {
        &self.0
    }
}

/// A parsed, well-formed envelope. Says nothing yet about whether the signature is any good.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SignatureEnvelope {
    pub(crate) envelope_version: u32,
    pub(crate) algorithm: SignatureAlgorithm,
    pub(crate) publisher: PublisherId,
    pub(crate) extension: ExtensionId,
    pub(crate) version: Version,
    pub(crate) package_hash: PackageHash,
    pub(crate) package_bytes: u64,
    pub(crate) claimed_manifest_digest: ManifestDigest,
    pub(crate) key_fingerprint: PublisherKeyFingerprint,
    pub(crate) signature: PackageSignature,
}

pub(crate) fn parse_signature_envelope(
    bytes: &[u8],
) -> Result<SignatureEnvelope, ManifestDecodeError> {
    let text = std::str::from_utf8(bytes).map_err(|_| {
        ManifestDecodeError::new(
            "",
            DecodeReason::NotPermitted {
                detail: "an envelope must be UTF-8",
            },
        )
    })?;
    let document = parse_block(text, ENVELOPE_YAML_LIMITS).map_err(|error| {
        ManifestDecodeError::new("", DecodeReason::MalformedDocument { code: error.code() })
    })?;

    let mut reader = MappingReader::open("", &document)?;

    // Version first. Everything below is this version's shape, and reporting a field problem for a
    // shape this build never claimed to read would send an author chasing the wrong thing.
    let envelope_version = integer(&mut reader, "envelope_version")?;
    let declared = u32::try_from(envelope_version).unwrap_or(u32::MAX);
    if declared != SUPPORTED_ENVELOPE_VERSION {
        return Err(ManifestDecodeError::new(
            "envelope_version",
            DecodeReason::UnsupportedSchemaVersion { declared },
        ));
    }

    let algorithm_text = reader.required_scalar("algorithm")?;
    let algorithm = SignatureAlgorithm::parse(algorithm_text).ok_or_else(|| {
        ManifestDecodeError::new(
            "algorithm",
            DecodeReason::UnknownValue {
                expected: "ed25519",
            },
        )
    })?;

    let publisher = PublisherId::parse(reader.required_scalar("publisher")?)
        .map_err(|error| identifier_at("publisher", &error))?;
    let extension = ExtensionId::parse(reader.required_scalar("extension")?)
        .map_err(|error| identifier_at("extension", &error))?;
    let version = Version::parse(reader.required_scalar("version")?)
        .map_err(|_| ManifestDecodeError::new("version", DecodeReason::InvalidVersion))?;
    let package_hash = PackageHash::parse(reader.required_scalar("package_sha256")?)
        .map_err(|error| identifier_at("package_sha256", &error))?;
    let package_bytes = integer(&mut reader, "package_bytes")?;
    let claimed_manifest_digest = ManifestDigest::parse(reader.required_scalar("manifest_sha256")?)
        .ok_or_else(|| {
            ManifestDecodeError::new(
                "manifest_sha256",
                DecodeReason::NotPermitted {
                    detail: "a digest is 64 lowercase hexadecimal characters",
                },
            )
        })?;
    let key_fingerprint =
        PublisherKeyFingerprint::parse(reader.required_scalar("key_fingerprint")?)
            .map_err(|error| identifier_at("key_fingerprint", &error))?;
    let signature = decode_signature(reader.required_scalar("signature")?)?;

    reader.finish()?;

    Ok(SignatureEnvelope {
        envelope_version: declared,
        algorithm,
        publisher,
        extension,
        version,
        package_hash,
        package_bytes,
        claimed_manifest_digest,
        key_fingerprint,
        signature,
    })
}

fn integer(reader: &mut MappingReader<'_>, field: &str) -> Result<u64, ManifestDecodeError> {
    let value = reader.required_scalar(field)?;
    value.parse::<u64>().map_err(|_| {
        ManifestDecodeError::new(
            field,
            DecodeReason::NotPermitted {
                detail: "expects a non-negative whole number",
            },
        )
    })
}

/// Standard base64 with padding, decoded to exactly 64 bytes.
///
/// Length is checked rather than assumed: a short or long signature is a malformed envelope, and
/// the verifier should never be handed a slice it has to interpret.
fn decode_signature(value: &str) -> Result<PackageSignature, ManifestDecodeError> {
    let malformed = || {
        ManifestDecodeError::new(
            "signature",
            DecodeReason::NotPermitted {
                detail: "expects standard base64 of 64 bytes",
            },
        )
    };
    let decoded = STANDARD.decode(value).map_err(|_| malformed())?;
    let bytes: [u8; SIGNATURE_BYTES] = decoded.try_into().map_err(|_| malformed())?;
    Ok(PackageSignature::from_bytes(bytes))
}
