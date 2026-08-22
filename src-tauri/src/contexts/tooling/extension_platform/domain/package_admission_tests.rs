//! Admission: what is refused by default, what Developer Mode changes, and what it does not.

use super::DecodeReason;
use super::{
    activation_eligibility, admit_package, ActivationEligibility, AdmissionRefusal,
    AdmittedPackage, DeveloperMode, ManifestDecodeError, PackageAdmission, PersistentWarning,
    SignatureRejection, SignatureState, TrustProfile, ALL_ADMISSION_REFUSALS,
    ALL_SIGNATURE_REJECTIONS,
};

/// Named so a change in what admission hands back cannot pass unnoticed here.
fn admitted(signature: &SignatureState, mode: DeveloperMode) -> AdmittedPackage {
    admit_package(signature, mode)
        .admitted()
        .cloned()
        .expect("admitted")
}

fn unreadable() -> SignatureState {
    SignatureState::Unreadable(ManifestDecodeError::new("signature", DecodeReason::Missing))
}

fn verified() -> SignatureState {
    // Built through the real verifier, so this test cannot assert about a state the production
    // path is unable to produce.
    SignatureState::Verified(super::signature_test_support::verified_signature())
}

#[test]
fn an_unsigned_package_is_refused_by_default() {
    // Refused, not "installed disabled with a warning". A default that admits and warns is a
    // default that admits.
    assert_eq!(
        admit_package(&SignatureState::Unsigned, DeveloperMode::Off),
        PackageAdmission::Refused(AdmissionRefusal::UnsignedWithoutDeveloperMode)
    );
}

#[test]
fn developer_mode_admits_unsigned_content_only_as_disabled_strict_and_warned() {
    let admitted = admitted(&SignatureState::Unsigned, DeveloperMode::On);

    assert!(!admitted.enabled_on_install);
    assert_eq!(admitted.forced_trust_profile, Some(TrustProfile::Strict));
    assert_eq!(
        admitted.persistent_warning,
        Some(PersistentWarning::UnsignedContent)
    );
    assert!(!admitted.automatic_updates);
    assert!(!admitted.activate_at_startup);
}

#[test]
fn developer_mode_does_not_admit_a_signature_that_is_present_and_wrong() {
    // The rule most likely to be got wrong. Content whose signature failed is not unsigned
    // content, and a switch meant for the latter has nothing to say about the former.
    for mode in [DeveloperMode::Off, DeveloperMode::On] {
        assert_eq!(
            admit_package(&unreadable(), mode),
            PackageAdmission::Refused(AdmissionRefusal::SignatureUnreadable),
            "{mode:?}"
        );
        for rejection in ALL_SIGNATURE_REJECTIONS {
            assert_eq!(
                admit_package(&SignatureState::Rejected(rejection), mode),
                PackageAdmission::Refused(AdmissionRefusal::SignatureRejected(rejection)),
                "{mode:?} {rejection:?}"
            );
        }
    }
}

#[test]
fn a_verified_signature_admits_without_enabling_anything() {
    let admitted = admitted(&verified(), DeveloperMode::Off);

    assert!(
        !admitted.enabled_on_install,
        "a valid signature is provenance, not permission"
    );
    assert_eq!(
        admitted.forced_trust_profile, None,
        "a signature must not select a less restrictive profile"
    );
    assert_eq!(admitted.persistent_warning, None);
}

#[test]
fn developer_mode_makes_no_difference_to_a_signed_package() {
    assert_eq!(
        admit_package(&verified(), DeveloperMode::On),
        admit_package(&verified(), DeveloperMode::Off)
    );
}

#[test]
fn turning_developer_mode_off_stops_new_activation_without_touching_what_is_installed() {
    assert_eq!(
        activation_eligibility(&SignatureState::Unsigned, DeveloperMode::On),
        ActivationEligibility::Eligible
    );
    assert_eq!(
        activation_eligibility(&SignatureState::Unsigned, DeveloperMode::Off),
        ActivationEligibility::IneligibleUnsignedWithoutDeveloperMode
    );
    // Turning it back on restores eligibility. Nothing had to be reinstalled, which is the point:
    // the switch governs activation, not the bytes on disk.
    assert_eq!(
        activation_eligibility(&SignatureState::Unsigned, DeveloperMode::On),
        ActivationEligibility::Eligible
    );
}

#[test]
fn a_rejected_signature_is_ineligible_for_activation_under_either_mode() {
    for mode in [DeveloperMode::Off, DeveloperMode::On] {
        assert_eq!(
            activation_eligibility(&unreadable(), mode),
            ActivationEligibility::IneligibleSignatureUnreadable
        );
        assert_eq!(
            activation_eligibility(
                &SignatureState::Rejected(SignatureRejection::RevokedPublisherKey),
                mode
            ),
            ActivationEligibility::IneligibleSignatureRejected(
                SignatureRejection::RevokedPublisherKey
            )
        );
    }
}

#[test]
fn a_verified_package_is_eligible_regardless_of_developer_mode() {
    for mode in [DeveloperMode::Off, DeveloperMode::On] {
        assert_eq!(
            activation_eligibility(&verified(), mode),
            ActivationEligibility::Eligible
        );
    }
}

#[test]
fn every_refusal_and_eligibility_reason_has_a_distinct_stable_code() {
    let mut codes: Vec<&str> = ALL_ADMISSION_REFUSALS
        .iter()
        .map(|refusal| refusal.code())
        .collect();
    codes.extend(
        ALL_SIGNATURE_REJECTIONS
            .iter()
            .map(|rejection| AdmissionRefusal::SignatureRejected(*rejection).code()),
    );
    let total = codes.len();
    codes.sort_unstable();
    codes.dedup();
    assert_eq!(codes.len(), total);

    assert_eq!(DeveloperMode::On.as_str(), "on");
    assert_eq!(DeveloperMode::Off.as_str(), "off");
    assert_eq!(DeveloperMode::from_enabled(true), DeveloperMode::On);
    assert_eq!(DeveloperMode::from_enabled(false), DeveloperMode::Off);
    assert_eq!(
        PersistentWarning::UnsignedContent.code(),
        "unsigned_content"
    );
    assert_eq!(ActivationEligibility::Eligible.code(), "eligible");
}
