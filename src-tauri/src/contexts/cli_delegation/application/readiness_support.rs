use super::{DelegationMode, DelegationTarget};
use crate::contexts::tooling::api::compare_versions;
use std::cmp::Ordering;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum VersionClass {
    Unparseable,
    Below,
    Tested,
    AboveReviewed,
}

pub(super) fn version_class(target: DelegationTarget, output: &str) -> VersionClass {
    let Some(version) = output
        .split_whitespace()
        .find(|part| {
            part.chars()
                .next()
                .is_some_and(|value| value.is_ascii_digit())
        })
        .map(|value| {
            value.trim_matches(|character: char| {
                !character.is_ascii_alphanumeric() && character != '.'
            })
        })
    else {
        return VersionClass::Unparseable;
    };
    let (minimum, maximum) = match target {
        DelegationTarget::ClaudeCode => ("1.0.0", "2.999.999"),
        DelegationTarget::CodexCli => ("0.1.0", "0.999.999"),
    };
    match (
        compare_versions(version, minimum),
        compare_versions(version, maximum),
    ) {
        (Some(Ordering::Less), _) => VersionClass::Below,
        (Some(_), Some(Ordering::Greater)) => VersionClass::AboveReviewed,
        (Some(_), Some(_)) => VersionClass::Tested,
        _ => VersionClass::Unparseable,
    }
}

pub(super) fn required_flags(
    target: DelegationTarget,
    mode: DelegationMode,
) -> &'static [&'static str] {
    match (target, mode) {
        (DelegationTarget::ClaudeCode, DelegationMode::Analyze) => &[
            "--print",
            "--output-format",
            "--json-schema",
            "--strict-mcp-config",
            "--max-turns",
        ],
        (DelegationTarget::ClaudeCode, DelegationMode::Edit) => &[
            "--print",
            "--output-format",
            "--json-schema",
            "--permission-mode",
            "--tools",
            "--strict-mcp-config",
            "--max-turns",
        ],
        (DelegationTarget::CodexCli, DelegationMode::Analyze)
        | (DelegationTarget::CodexCli, DelegationMode::Edit) => &[
            "exec",
            "--json",
            "--output-schema",
            "--sandbox",
            "--ephemeral",
        ],
    }
}
