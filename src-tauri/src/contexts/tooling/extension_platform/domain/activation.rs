// No production caller yet; see `identity.rs`.
#![cfg_attr(not(test), allow(dead_code))]

//! What wakes an extension's runtime.
//!
//! Contributions are indexed from the manifest without running anything; a runtime starts only
//! when one of these fires or a user asks for it. The set is closed — an unrecognised event would
//! index a contribution that nothing could ever activate, which reads in the UI as "installed and
//! idle" rather than as the misconfiguration it is.

use super::{ExtensionDomainError, IdentifierKind};

const MAX_EVENT_CHARACTERS: usize = 160;
const MAX_TARGET_CHARACTERS: usize = 128;

/// An activation trigger, as written in `activation_events`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum ActivationEvent {
    /// After the desktop runtime has finished starting. The most expensive trigger, and the one a
    /// review should question.
    StartupFinished,
    SessionStart,
    AgentMode(ActivationTarget),
    Tool(ActivationTarget),
    Hook(ActivationTarget),
    Connector(ActivationTarget),
    Command(ActivationTarget),
    /// Only an explicit user action activates it.
    Manual,
}

/// The `<target>` half of a parameterised event.
///
/// Deliberately permissive about *shape* and strict about *size*: it names a tool, hook, mode,
/// connector, or command owned by some other subsystem, and this type must not encode another
/// subsystem's id rule. Whether the target exists is resolved later, against the registry.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct ActivationTarget(String);

impl ActivationTarget {
    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

impl ActivationEvent {
    pub(crate) fn parse(value: &str) -> Result<Self, ExtensionDomainError> {
        let invalid = || {
            ExtensionDomainError::new(
                IdentifierKind::ActivationEvent,
                value.chars().take(MAX_EVENT_CHARACTERS).collect(),
            )
        };
        if value.len() > MAX_EVENT_CHARACTERS {
            return Err(invalid());
        }
        match value {
            "onStartupFinished" => return Ok(Self::StartupFinished),
            "onSessionStart" => return Ok(Self::SessionStart),
            "manual" => return Ok(Self::Manual),
            _ => {}
        }
        let (prefix, target) = value.split_once(':').ok_or_else(invalid)?;
        let target = parse_target(target).ok_or_else(invalid)?;
        match prefix {
            "onAgentMode" => Ok(Self::AgentMode(target)),
            "onTool" => Ok(Self::Tool(target)),
            "onHook" => Ok(Self::Hook(target)),
            "onConnector" => Ok(Self::Connector(target)),
            "onCommand" => Ok(Self::Command(target)),
            _ => Err(invalid()),
        }
    }

    /// The manifest spelling. Round-trips with `parse`.
    pub(crate) fn to_manifest_value(&self) -> String {
        match self {
            Self::StartupFinished => "onStartupFinished".to_string(),
            Self::SessionStart => "onSessionStart".to_string(),
            Self::Manual => "manual".to_string(),
            Self::AgentMode(target) => format!("onAgentMode:{}", target.as_str()),
            Self::Tool(target) => format!("onTool:{}", target.as_str()),
            Self::Hook(target) => format!("onHook:{}", target.as_str()),
            Self::Connector(target) => format!("onConnector:{}", target.as_str()),
            Self::Command(target) => format!("onCommand:{}", target.as_str()),
        }
    }

    /// Whether activation happens without any user action. Reviewed at install time: an extension
    /// that wakes on startup runs before the user has done anything with it.
    pub(crate) fn is_automatic(&self) -> bool {
        !matches!(self, Self::Manual)
    }

    pub(crate) fn target(&self) -> Option<&ActivationTarget> {
        match self {
            Self::StartupFinished | Self::SessionStart | Self::Manual => None,
            Self::AgentMode(target)
            | Self::Tool(target)
            | Self::Hook(target)
            | Self::Connector(target)
            | Self::Command(target) => Some(target),
        }
    }
}

/// Targets exclude whitespace, control characters, and a further `:` so that an event round-trips
/// through its manifest spelling unambiguously.
fn parse_target(value: &str) -> Option<ActivationTarget> {
    let valid = !value.is_empty()
        && value.len() <= MAX_TARGET_CHARACTERS
        && value.chars().all(|character| {
            !character.is_whitespace() && !character.is_control() && character != ':'
        });
    valid.then(|| ActivationTarget(value.to_string()))
}
