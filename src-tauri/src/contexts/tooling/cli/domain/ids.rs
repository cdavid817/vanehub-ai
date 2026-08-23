//! Validated identifier value objects for the CLI environment domain.
//!
//! Every identifier that crosses a boundary -- a wire DTO, a SQLite row, a plan reference -- is
//! constructed through one of these types. Deserializing a DTO or reading a row must not be able
//! to produce an identifier the domain would have rejected.
//!
//! Existing stable Agent ids (`claude-code`, `codex-cli`, ...) are unchanged; these wrap them.

use std::fmt;

/// Long enough for any real Agent id, source id, or generated plan id, short enough that a
/// malformed value cannot become an unbounded key in a map or a log line.
const MAX_ID_LENGTH: usize = 128;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CliIdError {
    Empty { label: &'static str },
    TooLong { label: &'static str, length: usize },
    ControlCharacter { label: &'static str },
    LeadingOrTrailingSpace { label: &'static str },
}

impl fmt::Display for CliIdError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty { label } => write!(formatter, "{label} must not be empty"),
            Self::TooLong { label, length } => write!(
                formatter,
                "{label} is {length} characters; the maximum is {MAX_ID_LENGTH}"
            ),
            Self::ControlCharacter { label } => {
                write!(formatter, "{label} must not contain control characters")
            }
            Self::LeadingOrTrailingSpace { label } => {
                write!(formatter, "{label} must not start or end with whitespace")
            }
        }
    }
}

fn validate(value: &str, label: &'static str) -> Result<(), CliIdError> {
    if value.is_empty() {
        return Err(CliIdError::Empty { label });
    }
    if value.chars().count() > MAX_ID_LENGTH {
        return Err(CliIdError::TooLong {
            label,
            length: value.chars().count(),
        });
    }
    if value.chars().any(char::is_control) {
        return Err(CliIdError::ControlCharacter { label });
    }
    // A surrounding space is almost always a copy-paste artifact, and silently trimming it would
    // make two different-looking ids compare equal in one place and not another.
    if value.trim() != value {
        return Err(CliIdError::LeadingOrTrailingSpace { label });
    }
    Ok(())
}

/// Defines a validated newtype over `String`. The five CLI identifiers differ only in their
/// diagnostic label, so the invariant lives in one place rather than five copies that can drift.
macro_rules! cli_identifier {
    ($name:ident, $label:literal) => {
        #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub(crate) struct $name(String);

        impl $name {
            pub(crate) fn new(value: impl Into<String>) -> Result<Self, CliIdError> {
                let value = value.into();
                validate(&value, $label)?;
                Ok(Self(value))
            }

            /// Builds an identifier from a value this codebase produced itself.
            ///
            /// Validation exists to reject what came from outside -- a wire field, a stored row, a
            /// user entry. A literal in this repository, or a string assembled here from ASCII
            /// pieces, is not one of those. `expect`ing on such a value would put a panic in a
            /// release binary to guard against a typo the test suite already catches, so the check
            /// is a debug assertion instead: it fires in every test run and costs a user nothing.
            ///
            /// **Never for external input.** A DTO field, a SQLite column, a PATH entry, a package
            /// manager's stdout, or anything off the network goes through `new` and is refused if
            /// it does not validate. The visibility below is the structural half of that rule: a
            /// command mapper cannot reach this at all, because it lives outside this context.
            /// `no_external_input_reaches_the_trusted_identifier_constructor` in the architecture
            /// suite is the other half, covering the call sites inside it.
            // Generated for all five identifiers; only the ones with an in-tree literal or a
            // generated value call it. `allow` rather than `expect` because which instantiations
            // are used is a property of the callers, not of this macro.
            #[allow(dead_code)]
            pub(in crate::contexts::tooling::cli) fn trusted(value: impl Into<String>) -> Self {
                let value = value.into();
                debug_assert!(
                    validate(&value, $label).is_ok(),
                    concat!($label, " built in-tree is invalid")
                );
                Self(value)
            }

            pub(crate) fn as_str(&self) -> &str {
                &self.0
            }

            pub(crate) fn into_inner(self) -> String {
                self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str(&self.0)
            }
        }

        impl PartialEq<str> for $name {
            fn eq(&self, other: &str) -> bool {
                self.0 == other
            }
        }
    };
}

cli_identifier!(CliToolId, "CLI tool id");
cli_identifier!(CliSourceId, "CLI source id");
cli_identifier!(CliInstallationId, "CLI installation id");
cli_identifier!(CliActionPlanId, "CLI action plan id");
cli_identifier!(CliBulkPlanId, "CLI bulk plan id");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_in_tree_identifier_would_also_pass_validation() {
        // `trusted` skips the check in release builds, so the values this repository actually
        // passes to it are asserted here instead. A literal that stopped being valid would
        // otherwise only fail as a debug assertion on whoever ran the app next.
        for source in ["npm", "winget", "vendor"] {
            assert_eq!(
                CliSourceId::trusted(source),
                CliSourceId::new(source).expect("valid")
            );
        }
        for installation in ["i-unknown", "legacy"] {
            assert_eq!(
                CliInstallationId::trusted(installation),
                CliInstallationId::new(installation).expect("valid")
            );
        }
        // The generated shape the id factory produces.
        let generated = format!("cli-plan-1-{}", uuid::Uuid::nil());
        assert_eq!(
            CliActionPlanId::trusted(generated.clone()),
            CliActionPlanId::new(generated).expect("valid")
        );
    }

    #[test]
    fn stable_agent_ids_are_accepted_unchanged() {
        for agent_id in [
            "claude-code",
            "codex-cli",
            "gemini-cli",
            "opencode",
            "antigravity-cli",
        ] {
            let id = CliToolId::new(agent_id).expect("stable agent id");
            assert_eq!(id.as_str(), agent_id);
            assert_eq!(id.to_string(), agent_id);
        }
    }

    #[test]
    fn empty_control_and_oversized_identifiers_are_rejected() {
        assert_eq!(
            CliToolId::new(""),
            Err(CliIdError::Empty {
                label: "CLI tool id"
            })
        );
        assert_eq!(
            CliSourceId::new("np\u{0}m"),
            Err(CliIdError::ControlCharacter {
                label: "CLI source id"
            })
        );
        assert_eq!(
            CliSourceId::new("npm\nglobal"),
            Err(CliIdError::ControlCharacter {
                label: "CLI source id"
            })
        );
        let oversized = "a".repeat(MAX_ID_LENGTH + 1);
        assert_eq!(
            CliActionPlanId::new(oversized),
            Err(CliIdError::TooLong {
                label: "CLI action plan id",
                length: MAX_ID_LENGTH + 1
            })
        );
        // The boundary itself is valid.
        assert!(CliActionPlanId::new("a".repeat(MAX_ID_LENGTH)).is_ok());
    }

    #[test]
    fn surrounding_whitespace_is_rejected_rather_than_trimmed() {
        // Trimming would make " npm" and "npm" equal here and unequal in SQLite, which is how a
        // plan ends up pointing at a source that no lookup can find.
        assert_eq!(
            CliSourceId::new(" npm"),
            Err(CliIdError::LeadingOrTrailingSpace {
                label: "CLI source id"
            })
        );
        assert_eq!(
            CliSourceId::new("npm "),
            Err(CliIdError::LeadingOrTrailingSpace {
                label: "CLI source id"
            })
        );
    }

    #[test]
    fn identifiers_of_different_kinds_do_not_compare_equal() {
        let tool = CliToolId::new("claude-code").expect("tool");
        let installation = CliInstallationId::new("claude-code").expect("installation");
        // Same text, different types: the compiler rejects `tool == installation`, which is the
        // point of separate newtypes rather than one shared `Id` alias.
        assert_eq!(tool.as_str(), installation.as_str());
        assert_eq!(tool.into_inner(), "claude-code");
    }

    #[test]
    fn all_five_identifier_kinds_share_the_same_invariant_and_accessors() {
        // The macro generates five types from one definition, so the invariant is exercised once
        // per type rather than trusted to have been copied correctly.
        macro_rules! assert_identifier_behaviour {
            ($ty:ident, $label:literal) => {
                let id = $ty::new("fixture-value").expect(concat!($label, " accepts a valid id"));
                assert_eq!(id.as_str(), "fixture-value");
                assert_eq!(id.to_string(), "fixture-value");
                assert!(id == *"fixture-value");
                assert!(!(id == *"other"));
                assert_eq!(id.clone().into_inner(), "fixture-value");
                assert_eq!($ty::new(""), Err(CliIdError::Empty { label: $label }));
                assert_eq!(
                    $ty::new("bad\u{7}"),
                    Err(CliIdError::ControlCharacter { label: $label })
                );
            };
        }

        assert_identifier_behaviour!(CliToolId, "CLI tool id");
        assert_identifier_behaviour!(CliSourceId, "CLI source id");
        assert_identifier_behaviour!(CliInstallationId, "CLI installation id");
        assert_identifier_behaviour!(CliActionPlanId, "CLI action plan id");
        assert_identifier_behaviour!(CliBulkPlanId, "CLI bulk plan id");
    }

    #[test]
    fn each_error_variant_renders_a_message_naming_its_field() {
        assert!(CliIdError::Empty {
            label: "CLI tool id"
        }
        .to_string()
        .contains("CLI tool id"));
        assert!(CliIdError::TooLong {
            label: "CLI source id",
            length: 999
        }
        .to_string()
        .contains("999"));
        assert!(CliIdError::ControlCharacter {
            label: "CLI action plan id"
        }
        .to_string()
        .contains("control"));
        assert!(CliIdError::LeadingOrTrailingSpace {
            label: "CLI bulk plan id"
        }
        .to_string()
        .contains("whitespace"));
    }

    #[test]
    fn identifiers_are_usable_as_ordered_map_keys() {
        use std::collections::BTreeSet;

        let mut ids = BTreeSet::new();
        assert!(ids.insert(CliToolId::new("opencode").expect("id")));
        assert!(ids.insert(CliToolId::new("claude-code").expect("id")));
        assert!(!ids.insert(CliToolId::new("opencode").expect("id")));
        assert_eq!(
            ids.iter().map(CliToolId::as_str).collect::<Vec<_>>(),
            vec!["claude-code", "opencode"]
        );
    }
}
