// The install flow that calls this lands with Task Group 4; see `identity.rs`.
#![cfg_attr(not(test), allow(dead_code))]

//! Whether a package may be installed at all, given what is known about its provenance.
//!
//! One rule, stated three ways because each is a thing people get wrong:
//!
//! * **Unsigned is refused by default.** Not warned about, not installed disabled — refused. A
//!   default that admits and warns is a default that admits.
//! * **Developer Mode admits unsigned, never forged.** A package whose signature is present and
//!   wrong is not an unsigned package; it is a package someone tried to make look signed.
//!   Developer Mode is for content that has no signature yet, and it has nothing to say about
//!   content whose signature failed.
//! * **Developer Mode changes admission and nothing else.** The result carries no limits, no
//!   permissions, and no ceilings, because there is no field here that could carry one. Archive,
//!   path, compatibility, Permissions, Hook, rule, connector, logging, and runtime limits are
//!   decided elsewhere and cannot be reached from this decision.
//!
//! What an admitted unsigned package gets is fixed and not negotiable: installed disabled, the
//! Strict profile, a warning that stays attached to it, no automatic updates, and no activation at
//! startup.

use super::{SignatureRejection, SignatureState, TrustProfile};

/// Whether an operator has explicitly turned on installation of unsigned content.
///
/// Two states and no default parameter anywhere: every caller has to say which it has, so "we
/// forgot to pass it" cannot resolve to on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DeveloperMode {
    Off,
    On,
}

impl DeveloperMode {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::On => "on",
        }
    }

    pub(crate) const fn from_enabled(enabled: bool) -> Self {
        if enabled {
            Self::On
        } else {
            Self::Off
        }
    }

    pub(crate) const fn is_on(self) -> bool {
        matches!(self, Self::On)
    }
}

/// A warning that has to keep being shown, not one that is dismissed once.
///
/// An enum rather than a message: the text belongs to the locale files, and a warning identified
/// by its rendered English would be untranslatable and unmatchable in a log.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PersistentWarning {
    /// This extension's bytes are not attributable to any publisher.
    UnsignedContent,
}

impl PersistentWarning {
    pub(crate) const fn code(self) -> &'static str {
        match self {
            Self::UnsignedContent => "unsigned_content",
        }
    }
}

/// The containment an admitted package starts under.
///
/// Every field is a restriction. There is deliberately no field that grants anything, so no future
/// edit to this struct can turn admission into authority without that being obvious in the diff.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AdmittedPackage {
    /// Always false at install. Nothing is enabled by being installed, signed or not.
    pub(crate) enabled_on_install: bool,
    /// `None` when the operator chooses within what the runtime kind permits; `Some` when the
    /// admission itself pins the profile, which is what unsigned content gets.
    pub(crate) forced_trust_profile: Option<TrustProfile>,
    pub(crate) persistent_warning: Option<PersistentWarning>,
    pub(crate) automatic_updates: bool,
    pub(crate) activate_at_startup: bool,
}

impl AdmittedPackage {
    /// What a package with verified provenance gets: installed disabled, like everything else, and
    /// otherwise unconstrained by *this* decision.
    ///
    /// A valid signature grants nothing. It says the bytes came from a publisher the operator
    /// trusts, and the authority the extension then receives is decided separately.
    const fn signed() -> Self {
        Self {
            enabled_on_install: false,
            forced_trust_profile: None,
            persistent_warning: None,
            automatic_updates: true,
            activate_at_startup: true,
        }
    }

    /// What unsigned content gets under Developer Mode. Fixed, and not a starting point for
    /// negotiation.
    const fn unsigned_under_developer_mode() -> Self {
        Self {
            enabled_on_install: false,
            forced_trust_profile: Some(TrustProfile::Strict),
            persistent_warning: Some(PersistentWarning::UnsignedContent),
            automatic_updates: false,
            activate_at_startup: false,
        }
    }
}

/// Why a package was not admitted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AdmissionRefusal {
    /// No signature at all, and Developer Mode is off.
    UnsignedWithoutDeveloperMode,
    /// A signature was offered and could not be read as one. Developer Mode does not apply: this
    /// is not unsigned content.
    SignatureUnreadable,
    /// A signature was offered and did not establish provenance.
    SignatureRejected(SignatureRejection),
}

impl AdmissionRefusal {
    pub(crate) const fn code(self) -> &'static str {
        match self {
            Self::UnsignedWithoutDeveloperMode => "unsigned_package_refused",
            Self::SignatureUnreadable => "signature_unreadable",
            Self::SignatureRejected(rejection) => rejection.code(),
        }
    }
}

pub(crate) const ALL_ADMISSION_REFUSALS: [AdmissionRefusal; 2] = [
    AdmissionRefusal::UnsignedWithoutDeveloperMode,
    AdmissionRefusal::SignatureUnreadable,
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum PackageAdmission {
    Admitted(AdmittedPackage),
    Refused(AdmissionRefusal),
}

impl PackageAdmission {
    pub(crate) const fn admitted(&self) -> Option<&AdmittedPackage> {
        match self {
            Self::Admitted(package) => Some(package),
            Self::Refused(_) => None,
        }
    }
}

/// Decides whether a package may be installed.
pub(crate) fn admit_package(
    signature: &SignatureState,
    developer_mode: DeveloperMode,
) -> PackageAdmission {
    match signature {
        SignatureState::Verified(_) => PackageAdmission::Admitted(AdmittedPackage::signed()),
        SignatureState::Unsigned if developer_mode.is_on() => {
            PackageAdmission::Admitted(AdmittedPackage::unsigned_under_developer_mode())
        }
        SignatureState::Unsigned => {
            PackageAdmission::Refused(AdmissionRefusal::UnsignedWithoutDeveloperMode)
        }
        // Deliberately not gated on Developer Mode. A broken or forged signature is not unsigned
        // content, and admitting it under a switch meant for unsigned content would make that
        // switch far more dangerous than it reads.
        SignatureState::Unreadable(_) => {
            PackageAdmission::Refused(AdmissionRefusal::SignatureUnreadable)
        }
        SignatureState::Rejected(rejection) => {
            PackageAdmission::Refused(AdmissionRefusal::SignatureRejected(*rejection))
        }
    }
}

/// Whether an already-installed unsigned extension may still be activated.
///
/// Turning Developer Mode off does not uninstall anything and deletes no evidence. It makes
/// unsigned extensions ineligible for *new* activation until they are signed and trusted, or until
/// Developer Mode is explicitly turned on again.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ActivationEligibility {
    Eligible,
    /// Installed, retained, and not activatable right now.
    IneligibleUnsignedWithoutDeveloperMode,
    IneligibleSignatureRejected(SignatureRejection),
    IneligibleSignatureUnreadable,
}

impl ActivationEligibility {
    pub(crate) const fn code(self) -> &'static str {
        match self {
            Self::Eligible => "eligible",
            Self::IneligibleUnsignedWithoutDeveloperMode => "ineligible_unsigned",
            Self::IneligibleSignatureRejected(rejection) => rejection.code(),
            Self::IneligibleSignatureUnreadable => "signature_unreadable",
        }
    }
}

pub(crate) fn activation_eligibility(
    signature: &SignatureState,
    developer_mode: DeveloperMode,
) -> ActivationEligibility {
    match signature {
        SignatureState::Verified(_) => ActivationEligibility::Eligible,
        SignatureState::Unsigned if developer_mode.is_on() => ActivationEligibility::Eligible,
        SignatureState::Unsigned => ActivationEligibility::IneligibleUnsignedWithoutDeveloperMode,
        SignatureState::Unreadable(_) => ActivationEligibility::IneligibleSignatureUnreadable,
        SignatureState::Rejected(rejection) => {
            ActivationEligibility::IneligibleSignatureRejected(*rejection)
        }
    }
}

/// Why reading or changing the Developer Mode switch did not happen.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum DeveloperModeError {
    /// Someone else changed the switch since the caller last read it. Refused rather than
    /// overwritten: a toggle that silently wins a race is a toggle whose state nobody can rely on.
    StaleRevision {
        expected: i64,
        actual: i64,
    },
    Storage(String),
}

impl DeveloperModeError {
    pub(crate) const fn code(&self) -> &'static str {
        match self {
            Self::StaleRevision { .. } => "developer_mode_stale_revision",
            Self::Storage(_) => "developer_mode_storage_failure",
        }
    }
}

pub(crate) fn all_developer_mode_errors() -> Vec<DeveloperModeError> {
    vec![
        DeveloperModeError::StaleRevision {
            expected: 0,
            actual: 0,
        },
        DeveloperModeError::Storage(String::new()),
    ]
}
