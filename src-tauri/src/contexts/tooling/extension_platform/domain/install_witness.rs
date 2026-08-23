// The preview and confirm operations that use this land with Task Group 4; see `identity.rs`.
#![cfg_attr(not(test), allow(dead_code))]

//! What a confirmation is bound to.
//!
//! A user approves an install after reading a screen. Between reading it and pressing confirm, a
//! publisher key can be revoked, a dependency can be uninstalled, and the file on disk can be
//! swapped. The witness is the record of what they were shown, and confirming re-derives the same
//! facts and refuses if any of them moved. "Silently proceed with different facts" is the failure
//! this exists to prevent, and it is prevented by comparison rather than by locking, because
//! nothing here can hold a lock across a human decision.
//!
//! The comparison reports *which* facts changed. "This preview is stale" tells a user to try
//! again; "the publisher key was revoked" tells them not to.

use super::canonical::{hex, join, sorted, Canonical};
use super::{
    global_ids, CapabilityRequest, ContributionGlobalId, ExtensionId, ExtensionManifestV1,
    ManifestDigest, PackageHash, PublisherKeyFingerprint, SignatureState, TrustProfile,
};
use semver::{Version, VersionReq};
use sha2::{Digest, Sha256};

/// The provenance answer, reduced to what a witness may keep.
///
/// A stable code and, where there is one, the key. Not the error: a decode error carries text
/// derived from the package, and a witness is compared for equality, so a diagnostic that varies
/// with wording would make two identical situations look different.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SignatureSummary {
    pub(crate) state: &'static str,
    pub(crate) key_fingerprint: Option<PublisherKeyFingerprint>,
}

impl SignatureSummary {
    pub(crate) fn of(state: &SignatureState) -> Self {
        Self {
            state: state.code(),
            key_fingerprint: state
                .verified()
                .map(|verified| verified.key_fingerprint().clone()),
        }
    }
}

/// What is installed right now, if anything.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct InstalledSummary {
    pub(crate) version: Version,
    pub(crate) package_hash: PackageHash,
    pub(crate) enabled: bool,
}

/// Whether this build can run the package.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum CompatibilityOutcome {
    Compatible,
    Incompatible {
        required: VersionReq,
        running: Version,
    },
}

/// One dependency and whether it is satisfied.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DependencySummary {
    pub(crate) id: String,
    pub(crate) requirement: VersionReq,
    pub(crate) optional: bool,
    pub(crate) satisfied: bool,
}

/// What authority this version asks for that the installed one did not, and the reverse.
///
/// Rendered as canonical `kind:value` strings rather than as typed sets, because the comparison is
/// over what a reviewer reads. Two origins that differ only in spelling were already canonicalized
/// at decode, so equal strings here mean equal authority.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct CapabilityDiff {
    pub(crate) added: Vec<String>,
    pub(crate) removed: Vec<String>,
    pub(crate) unchanged: Vec<String>,
}

impl CapabilityDiff {
    /// Whether this update asks for anything the installed version did not have.
    ///
    /// The question the install wizard turns into a fresh confirmation. Removals do not count:
    /// giving authority back is not something to re-approve.
    pub(crate) fn broadens_authority(&self) -> bool {
        !self.added.is_empty()
    }
}

/// Every capability a request contains, as canonical strings.
pub(crate) fn capability_lines(request: &CapabilityRequest) -> Vec<String> {
    let mut lines = Vec::new();
    lines.extend(
        request
            .filesystem_read
            .iter()
            .map(|value| format!("filesystem.read:{value}")),
    );
    lines.extend(
        request
            .filesystem_write
            .iter()
            .map(|value| format!("filesystem.write:{value}")),
    );
    lines.extend(
        request
            .network_origins
            .iter()
            .map(|origin| format!("network:{}", origin.as_str())),
    );
    lines.extend(
        request
            .process_commands
            .iter()
            .map(|value| format!("process:{value}")),
    );
    lines.extend(
        request
            .secret_ids
            .iter()
            .map(|value| format!("secret:{value}")),
    );
    lines.sort();
    lines.dedup();
    lines
}

/// Compares what is requested against what the installed version already had.
///
/// `previous` is `None` for a first install, which makes everything requested an addition. That is
/// the honest reading: nothing was previously approved.
pub(crate) fn capability_diff(
    previous: Option<&CapabilityRequest>,
    requested: &CapabilityRequest,
) -> CapabilityDiff {
    let before = previous.map(capability_lines).unwrap_or_default();
    let after = capability_lines(requested);

    CapabilityDiff {
        added: after
            .iter()
            .filter(|line| !before.contains(line))
            .cloned()
            .collect(),
        removed: before
            .iter()
            .filter(|line| !after.contains(line))
            .cloned()
            .collect(),
        unchanged: after
            .iter()
            .filter(|line| before.contains(line))
            .cloned()
            .collect(),
    }
}

/// Everything a confirmation is bound to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct InstallWitnessSubject {
    pub(crate) extension: ExtensionId,
    pub(crate) version: Version,
    pub(crate) package_hash: PackageHash,
    pub(crate) manifest_digest: ManifestDigest,
    pub(crate) signature: SignatureSummary,
    pub(crate) installed: Option<InstalledSummary>,
    pub(crate) compatibility: CompatibilityOutcome,
    pub(crate) trust_profile: TrustProfile,
    pub(crate) dependencies: Vec<DependencySummary>,
    pub(crate) capabilities: CapabilityDiff,
    pub(crate) contributions: Vec<ContributionGlobalId>,
}

impl InstallWitnessSubject {
    /// The contributions a manifest declares, in the shape a witness records them.
    pub(crate) fn contributions_of(manifest: &ExtensionManifestV1) -> Vec<ContributionGlobalId> {
        let mut ids = global_ids(manifest);
        ids.sort();
        ids
    }
}

/// Which bound fact moved between preview and confirm.
///
/// A closed set, because a caller decides what to tell a user from it: a changed dependency is
/// "try again", and a changed signature is "do not".
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum WitnessField {
    Extension,
    Version,
    PackageHash,
    ManifestDigest,
    Signature,
    Installed,
    Compatibility,
    TrustProfile,
    Dependencies,
    Capabilities,
    Contributions,
}

impl WitnessField {
    pub(crate) const fn code(self) -> &'static str {
        match self {
            Self::Extension => "extension",
            Self::Version => "version",
            Self::PackageHash => "package_hash",
            Self::ManifestDigest => "manifest_digest",
            Self::Signature => "signature",
            Self::Installed => "installed_state",
            Self::Compatibility => "compatibility",
            Self::TrustProfile => "trust_profile",
            Self::Dependencies => "dependencies",
            Self::Capabilities => "capabilities",
            Self::Contributions => "contributions",
        }
    }
}

pub(crate) const ALL_WITNESS_FIELDS: [WitnessField; 11] = [
    WitnessField::Extension,
    WitnessField::Version,
    WitnessField::PackageHash,
    WitnessField::ManifestDigest,
    WitnessField::Signature,
    WitnessField::Installed,
    WitnessField::Compatibility,
    WitnessField::TrustProfile,
    WitnessField::Dependencies,
    WitnessField::Capabilities,
    WitnessField::Contributions,
];

/// The preview no longer describes the world.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StaleWitness {
    pub(crate) changed: Vec<WitnessField>,
}

impl StaleWitness {
    pub(crate) const fn code(&self) -> &'static str {
        "stale_install_witness"
    }
}

/// A preview, and the state it was taken against.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ExtensionInstallWitness {
    subject: InstallWitnessSubject,
    digest: String,
}

impl ExtensionInstallWitness {
    pub(crate) fn issue(subject: InstallWitnessSubject) -> Self {
        let digest = witness_digest(&subject);
        Self { subject, digest }
    }

    pub(crate) fn subject(&self) -> &InstallWitnessSubject {
        &self.subject
    }

    /// The identity a caller stores and compares. Derived, so a witness read back from storage
    /// whose subject was edited no longer matches its own digest.
    pub(crate) fn digest(&self) -> &str {
        &self.digest
    }

    /// Whether the witness still describes its own subject.
    ///
    /// The guard against a witness that arrived from somewhere it should not have: a caller can
    /// reconstruct the struct, but not a digest that agrees with it.
    pub(crate) fn is_self_consistent(&self) -> bool {
        witness_digest(&self.subject) == self.digest
    }

    /// Confirms the world still looks the way it did when this was issued.
    pub(crate) fn confirm(&self, current: &InstallWitnessSubject) -> Result<(), StaleWitness> {
        let changed = changed_fields(&self.subject, current);
        if changed.is_empty() {
            Ok(())
        } else {
            Err(StaleWitness { changed })
        }
    }
}

fn changed_fields(
    previewed: &InstallWitnessSubject,
    current: &InstallWitnessSubject,
) -> Vec<WitnessField> {
    let mut changed = Vec::new();
    let mut note = |field: WitnessField, differs: bool| {
        if differs {
            changed.push(field);
        }
    };

    note(
        WitnessField::Extension,
        previewed.extension != current.extension,
    );
    note(WitnessField::Version, previewed.version != current.version);
    note(
        WitnessField::PackageHash,
        previewed.package_hash != current.package_hash,
    );
    note(
        WitnessField::ManifestDigest,
        previewed.manifest_digest != current.manifest_digest,
    );
    note(
        WitnessField::Signature,
        previewed.signature != current.signature,
    );
    note(
        WitnessField::Installed,
        previewed.installed != current.installed,
    );
    note(
        WitnessField::Compatibility,
        previewed.compatibility != current.compatibility,
    );
    note(
        WitnessField::TrustProfile,
        previewed.trust_profile != current.trust_profile,
    );
    note(
        WitnessField::Dependencies,
        previewed.dependencies != current.dependencies,
    );
    note(
        WitnessField::Capabilities,
        previewed.capabilities != current.capabilities,
    );
    note(
        WitnessField::Contributions,
        previewed.contributions != current.contributions,
    );
    changed
}

/// SHA-256 over the shared canonical encoding of every bound fact.
fn witness_digest(subject: &InstallWitnessSubject) -> String {
    hex(&Sha256::digest(canonical_witness_bytes(subject)))
}

/// The bytes the digest is taken over.
///
/// Exposed so the storage bound can be measured against the same encoding the identity is derived
/// from. Measuring anything else -- the sum of field lengths, say -- would let the two disagree,
/// and the bound that matters is on what actually gets written.
pub(crate) fn canonical_witness_bytes(subject: &InstallWitnessSubject) -> Vec<u8> {
    let mut canonical = Canonical::default();
    canonical.tag("vanehub.extension-platform.install-witness.v1");

    canonical.tag("extension");
    canonical.text(subject.extension.as_str());
    canonical.tag("version");
    canonical.text(&subject.version.to_string());
    canonical.tag("package_hash");
    canonical.text(subject.package_hash.as_str());
    canonical.tag("manifest_digest");
    canonical.text(subject.manifest_digest.as_str());

    canonical.tag("signature");
    canonical.text(subject.signature.state);
    canonical.optional(
        subject
            .signature
            .key_fingerprint
            .as_ref()
            .map(PublisherKeyFingerprint::as_str),
    );

    canonical.tag("installed");
    match &subject.installed {
        Some(installed) => {
            canonical.text("some");
            canonical.text(&installed.version.to_string());
            canonical.text(installed.package_hash.as_str());
            canonical.text(if installed.enabled {
                "enabled"
            } else {
                "disabled"
            });
        }
        None => canonical.text("none"),
    }

    canonical.tag("compatibility");
    match &subject.compatibility {
        CompatibilityOutcome::Compatible => canonical.text("compatible"),
        CompatibilityOutcome::Incompatible { required, running } => {
            canonical.text("incompatible");
            canonical.text(&required.to_string());
            canonical.text(&running.to_string());
        }
    }

    canonical.tag("trust_profile");
    canonical.text(subject.trust_profile.as_str());

    // Order-significant: a resolution plan is a sequence, and two plans differing only in order
    // are two different plans.
    canonical.tag("dependencies");
    canonical.text(&subject.dependencies.len().to_string());
    for dependency in &subject.dependencies {
        canonical.text(&join(&[
            &dependency.id,
            &dependency.requirement.to_string(),
            if dependency.optional {
                "optional"
            } else {
                "required"
            },
            if dependency.satisfied {
                "satisfied"
            } else {
                "unsatisfied"
            },
        ]));
    }

    canonical.tag("capabilities.added");
    canonical.text(&sorted(&subject.capabilities.added));
    canonical.tag("capabilities.removed");
    canonical.text(&sorted(&subject.capabilities.removed));
    canonical.tag("capabilities.unchanged");
    canonical.text(&sorted(&subject.capabilities.unchanged));

    canonical.tag("contributions");
    canonical.set(
        subject
            .contributions
            .iter()
            .map(|id| id.as_str().to_string()),
    );

    canonical.bytes().to_vec()
}
