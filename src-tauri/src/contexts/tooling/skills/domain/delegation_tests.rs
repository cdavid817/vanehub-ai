use super::delegation::{
    evaluate_delegated_assignment, evaluate_delegation_eligibility, RawSkillDelegation,
    SkillDelegationAgentRuntime, SkillDelegationCapabilityId, SkillDelegationDeclaration,
    SkillDelegationEligibility, SkillDelegationLimitField, SkillDelegationLimits,
    SkillDelegationUnavailableReason, DEFAULT_DELEGATION_CAPABILITIES,
};
use super::{SkillAvailability, SkillTrust, SkillType};
use std::collections::BTreeMap;

fn declaration(tools: &[&str], fields: &[(&str, &str)]) -> SkillDelegationDeclaration {
    SkillDelegationDeclaration::declared(RawSkillDelegation {
        tools: tools.iter().map(|tool| (*tool).to_string()).collect(),
        fields: fields
            .iter()
            .map(|(key, value)| ((*key).to_string(), (*value).to_string()))
            .collect::<BTreeMap<_, _>>(),
    })
}

fn utility(declaration: &SkillDelegationDeclaration) -> SkillDelegationEligibility {
    evaluate_delegation_eligibility(
        SkillType::Utility,
        SkillTrust::Trusted,
        SkillAvailability::Unsupported,
        declaration,
    )
}

#[test]
fn valid_contract_keeps_declared_capabilities_and_lower_limits() {
    let declaration = declaration(
        &["file-read", "content-search", "filename-search"],
        &[
            ("max_rounds", "6"),
            ("timeout_seconds", "90"),
            ("max_context_chars", "12000"),
            ("max_output_chars", "8000"),
        ],
    );

    let SkillDelegationEligibility::Available(contract) = utility(&declaration) else {
        panic!("expected an available delegation contract");
    };
    assert_eq!(
        contract.declared_capabilities,
        vec![
            SkillDelegationCapabilityId::FileRead,
            SkillDelegationCapabilityId::ContentSearch,
            SkillDelegationCapabilityId::FilenameSearch,
        ]
    );
    assert_eq!(
        contract.effective_capabilities,
        contract.declared_capabilities
    );
    assert_eq!(
        contract.effective_limits,
        SkillDelegationLimits {
            max_rounds: 6,
            timeout_seconds: 90,
            max_context_chars: 12_000,
            max_output_chars: 8_000,
        }
    );
    assert!(contract.capped_limits.is_empty());
    assert!(contract.is_read_only());
    assert!(!contract.uses_platform_default);
}

#[test]
fn missing_contract_falls_back_to_the_read_only_platform_default() {
    let SkillDelegationEligibility::Available(contract) =
        utility(&SkillDelegationDeclaration::default())
    else {
        panic!("expected an available delegation contract");
    };
    assert_eq!(
        contract.declared_capabilities,
        DEFAULT_DELEGATION_CAPABILITIES.to_vec()
    );
    assert!(contract.is_read_only());
    assert!(contract.uses_platform_default);
    assert_eq!(contract.effective_limits, SkillDelegationLimits::PLATFORM);
    assert_eq!(contract.requested_limits.max_rounds, None);
}

#[test]
fn declared_limits_above_platform_ceilings_are_capped_and_reported() {
    let declaration = declaration(
        &["file-read", "file-write"],
        &[
            ("max_rounds", "64"),
            ("timeout_seconds", "3600"),
            ("max_context_chars", "500000"),
            ("max_output_chars", "5000"),
        ],
    );

    let SkillDelegationEligibility::Available(contract) = utility(&declaration) else {
        panic!("expected an available delegation contract");
    };
    assert_eq!(contract.effective_limits.max_rounds, 8);
    assert_eq!(contract.effective_limits.timeout_seconds, 120);
    assert_eq!(contract.effective_limits.max_context_chars, 16_000);
    assert_eq!(contract.effective_limits.max_output_chars, 5_000);
    assert_eq!(
        contract.capped_limits,
        vec![
            SkillDelegationLimitField::MaxRounds,
            SkillDelegationLimitField::TimeoutSeconds,
            SkillDelegationLimitField::MaxContextChars,
        ]
    );
    assert_eq!(contract.requested_limits.max_rounds, Some(64));
    assert!(!contract.is_read_only());
}

#[test]
fn unknown_and_prohibited_capability_ids_make_delegation_unavailable() {
    for (tools, expected) in [
        (
            vec!["file-read", "launch-rockets"],
            SkillDelegationUnavailableReason::UnknownCapability,
        ),
        (
            vec!["file-read", "delegate-skill"],
            SkillDelegationUnavailableReason::ProhibitedCapability,
        ),
        (
            vec!["mcp"],
            SkillDelegationUnavailableReason::ProhibitedCapability,
        ),
        (
            vec!["file-read", "file-read"],
            SkillDelegationUnavailableReason::DuplicateCapability,
        ),
        (
            Vec::new(),
            SkillDelegationUnavailableReason::EmptyCapabilityList,
        ),
    ] {
        assert_eq!(
            utility(&declaration(&tools, &[])),
            SkillDelegationEligibility::Unavailable {
                reason: expected,
                contract: None,
            }
        );
    }
}

#[test]
fn invalid_limits_and_unknown_contract_fields_make_delegation_unavailable() {
    for (fields, expected) in [
        (
            vec![("max_rounds", "0")],
            SkillDelegationUnavailableReason::InvalidLimit,
        ),
        (
            vec![("timeout_seconds", "soon")],
            SkillDelegationUnavailableReason::InvalidLimit,
        ),
        (
            vec![("max_context_chars", "-1")],
            SkillDelegationUnavailableReason::InvalidLimit,
        ),
        (
            vec![("maxRounds", "4")],
            SkillDelegationUnavailableReason::UnknownContractField,
        ),
    ] {
        assert_eq!(
            utility(&declaration(&["file-read"], &fields)),
            SkillDelegationEligibility::Unavailable {
                reason: expected,
                contract: None,
            }
        );
    }
}

#[test]
fn role_skills_have_no_delegation_fields_and_untrusted_utilities_fail_closed() {
    assert_eq!(
        evaluate_delegation_eligibility(
            SkillType::Role,
            SkillTrust::Trusted,
            SkillAvailability::Available,
            &declaration(&["file-read"], &[]),
        ),
        SkillDelegationEligibility::NotApplicable
    );

    let untrusted = evaluate_delegation_eligibility(
        SkillType::Utility,
        SkillTrust::Untrusted,
        SkillAvailability::Available,
        &SkillDelegationDeclaration::default(),
    );
    let SkillDelegationEligibility::Unavailable { reason, contract } = untrusted else {
        panic!("expected an untrusted Utility to be unavailable");
    };
    assert_eq!(reason, SkillDelegationUnavailableReason::Untrusted);
    assert!(contract.is_some(), "declared capabilities stay visible");

    for availability in [
        SkillAvailability::Disabled,
        SkillAvailability::Invalid,
        SkillAvailability::Conflicting,
    ] {
        assert!(matches!(
            evaluate_delegation_eligibility(
                SkillType::Utility,
                SkillTrust::Trusted,
                availability,
                &SkillDelegationDeclaration::default(),
            ),
            SkillDelegationEligibility::Unavailable {
                reason: SkillDelegationUnavailableReason::SkillUnavailable,
                ..
            }
        ));
    }
}

#[test]
fn unavailable_reasons_are_stable_content_free_identifiers() {
    for (reason, expected) in [
        (
            SkillDelegationUnavailableReason::UnknownCapability,
            "unknown-capability",
        ),
        (
            SkillDelegationUnavailableReason::SkillUnavailable,
            "skill-unavailable",
        ),
        (
            SkillDelegationUnavailableReason::UnsupportedCliAgent,
            "unsupported-cli-agent",
        ),
        (
            SkillDelegationUnavailableReason::UnsupportedApiRuntime,
            "unsupported-api-runtime",
        ),
    ] {
        assert_eq!(reason.as_str(), expected);
    }
    assert_eq!(
        SkillDelegationCapabilityId::ScopedEdit.as_str(),
        "scoped-edit"
    );
    assert_eq!(
        SkillDelegationLimitField::MaxContextChars.as_str(),
        "max-context-chars"
    );
}

#[test]
fn only_native_api_runtimes_accept_a_delegated_utility_assignment() {
    assert_eq!(
        SkillDelegationAgentRuntime::classify(true, Some("anthropic")),
        SkillDelegationAgentRuntime::NativeApi
    );
    assert_eq!(
        SkillDelegationAgentRuntime::classify(true, Some("openai-compatible")),
        SkillDelegationAgentRuntime::NativeApi
    );
    assert_eq!(
        SkillDelegationAgentRuntime::classify(true, None),
        SkillDelegationAgentRuntime::UnsupportedApi
    );
    assert_eq!(
        SkillDelegationAgentRuntime::classify(false, None),
        SkillDelegationAgentRuntime::Cli
    );

    assert_eq!(
        evaluate_delegated_assignment(SkillDelegationAgentRuntime::NativeApi),
        Ok(())
    );
    assert_eq!(
        evaluate_delegated_assignment(SkillDelegationAgentRuntime::Cli),
        Err(SkillDelegationUnavailableReason::UnsupportedCliAgent)
    );
    assert_eq!(
        evaluate_delegated_assignment(SkillDelegationAgentRuntime::UnsupportedApi),
        Err(SkillDelegationUnavailableReason::UnsupportedApiRuntime)
    );
    assert!(!SkillDelegationAgentRuntime::Cli.supports_delegation());
}
