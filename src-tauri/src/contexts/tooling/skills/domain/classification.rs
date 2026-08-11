use super::{SkillDomainError, SkillId};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum SkillType {
    Role,
    Utility,
}

impl SkillType {
    pub(crate) fn parse(value: &str) -> Result<Self, SkillDomainError> {
        match value {
            "role" => Ok(Self::Role),
            "utility" => Ok(Self::Utility),
            _ => Err(SkillDomainError::InvalidMetadataValue(format!(
                "Unknown Skill type: {value}"
            ))),
        }
    }

    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Role => "role",
            Self::Utility => "utility",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum SkillDelivery {
    Eager,
    OnDemand,
}

impl SkillDelivery {
    pub(crate) fn parse(value: &str) -> Result<Self, SkillDomainError> {
        match value {
            "eager" => Ok(Self::Eager),
            "on-demand" => Ok(Self::OnDemand),
            _ => Err(SkillDomainError::InvalidMetadataValue(format!(
                "Unknown Skill delivery: {value}"
            ))),
        }
    }

    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Eager => "eager",
            Self::OnDemand => "on-demand",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum SkillLayer {
    Project,
    User,
    Registry,
    System,
}

impl SkillLayer {
    pub(crate) fn parse(value: &str) -> Option<Self> {
        match value {
            "project" => Some(Self::Project),
            "user" => Some(Self::User),
            "registry" => Some(Self::Registry),
            "system" => Some(Self::System),
            _ => None,
        }
    }

    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Project => "project",
            Self::User => "user",
            Self::Registry => "registry",
            Self::System => "system",
        }
    }

    pub(crate) fn rank(self) -> u8 {
        match self {
            Self::Project => 0,
            Self::User => 1,
            Self::Registry => 2,
            Self::System => 3,
        }
    }

    pub(crate) fn content_is_mutable(self) -> bool {
        matches!(self, Self::Project | Self::User)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum SkillOrigin {
    Created,
    Imported,
    Installed,
    Shipped,
    Migrated,
}

impl SkillOrigin {
    pub(crate) fn parse(value: &str) -> Option<Self> {
        match value {
            "created" => Some(Self::Created),
            "imported" => Some(Self::Imported),
            "installed" => Some(Self::Installed),
            "shipped" => Some(Self::Shipped),
            "migrated" => Some(Self::Migrated),
            _ => None,
        }
    }

    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Created => "created",
            Self::Imported => "imported",
            Self::Installed => "installed",
            Self::Shipped => "shipped",
            Self::Migrated => "migrated",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum SkillTrust {
    Trusted,
    Untrusted,
}

impl SkillTrust {
    #[cfg(test)]
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Trusted => "trusted",
            Self::Untrusted => "untrusted",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum SkillAvailability {
    Available,
    Disabled,
    Invalid,
    Conflicting,
    Unsupported,
}

impl SkillAvailability {
    pub(crate) fn parse(value: &str) -> Option<Self> {
        match value {
            "available" => Some(Self::Available),
            "disabled" => Some(Self::Disabled),
            "invalid" => Some(Self::Invalid),
            "conflicting" => Some(Self::Conflicting),
            "unsupported" => Some(Self::Unsupported),
            _ => None,
        }
    }

    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Available => "available",
            Self::Disabled => "disabled",
            Self::Invalid => "invalid",
            Self::Conflicting => "conflicting",
            Self::Unsupported => "unsupported",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct SkillCompatibilityDefaults {
    pub(crate) skill_type: bool,
    pub(crate) delivery: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SkillIdentityCandidate {
    pub(crate) id: SkillId,
    pub(crate) aliases: Vec<SkillId>,
    pub(crate) availability: SkillAvailability,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum SkillLookupOutcome {
    Resolved(SkillId),
    Unavailable {
        id: SkillId,
        availability: SkillAvailability,
    },
    Ambiguous(Vec<SkillId>),
    NotFound,
}

pub(crate) fn resolve_skill_identity(
    requested: &SkillId,
    candidates: &[SkillIdentityCandidate],
) -> SkillLookupOutcome {
    if let Some(candidate) = candidates
        .iter()
        .find(|candidate| candidate.id == *requested)
    {
        return candidate_outcome(candidate);
    }

    let mut aliases = candidates
        .iter()
        .filter(|candidate| candidate.aliases.contains(requested))
        .collect::<Vec<_>>();
    aliases.sort_by(|left, right| left.id.cmp(&right.id));
    aliases.dedup_by(|left, right| left.id == right.id);
    match aliases.as_slice() {
        [] => SkillLookupOutcome::NotFound,
        [candidate] => candidate_outcome(candidate),
        _ => SkillLookupOutcome::Ambiguous(
            aliases
                .into_iter()
                .map(|candidate| candidate.id.clone())
                .collect(),
        ),
    }
}

fn candidate_outcome(candidate: &SkillIdentityCandidate) -> SkillLookupOutcome {
    if candidate.availability == SkillAvailability::Available {
        SkillLookupOutcome::Resolved(candidate.id.clone())
    } else {
        SkillLookupOutcome::Unavailable {
            id: candidate.id.clone(),
            availability: candidate.availability,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn candidate(
        id: &str,
        aliases: &[&str],
        availability: SkillAvailability,
    ) -> SkillIdentityCandidate {
        SkillIdentityCandidate {
            id: SkillId::parse(id).expect("id"),
            aliases: aliases
                .iter()
                .map(|alias| SkillId::parse(*alias).expect("alias"))
                .collect(),
            availability,
        }
    }

    #[test]
    fn dimensions_are_independent_and_have_stable_wire_values() {
        assert_eq!(SkillType::Utility.as_str(), "utility");
        assert_eq!(SkillDelivery::OnDemand.as_str(), "on-demand");
        assert_eq!(SkillLayer::Project.rank(), 0);
        assert_eq!(SkillLayer::System.rank(), 3);
        assert_eq!(SkillOrigin::Installed.as_str(), "installed");
        assert_eq!(SkillTrust::Untrusted.as_str(), "untrusted");
        assert_eq!(SkillAvailability::Conflicting.as_str(), "conflicting");
    }

    #[test]
    fn canonical_id_wins_before_aliases() {
        let requested = SkillId::parse("developer").expect("request");
        let candidates = vec![
            candidate("developer", &[], SkillAvailability::Available),
            candidate("other-role", &["developer"], SkillAvailability::Available),
        ];
        assert_eq!(
            resolve_skill_identity(&requested, &candidates),
            SkillLookupOutcome::Resolved(requested)
        );
    }

    #[test]
    fn ambiguous_aliases_and_unavailable_utility_fail_closed() {
        let alias = SkillId::parse("dev").expect("alias");
        let candidates = vec![
            candidate("developer", &["dev"], SkillAvailability::Available),
            candidate("development-coach", &["dev"], SkillAvailability::Available),
        ];
        assert_eq!(
            resolve_skill_identity(&alias, &candidates),
            SkillLookupOutcome::Ambiguous(vec![
                SkillId::parse("developer").expect("id"),
                SkillId::parse("development-coach").expect("id")
            ])
        );

        let utility = candidate(
            "code-explorer",
            &["explore"],
            SkillAvailability::Unsupported,
        );
        assert_eq!(
            resolve_skill_identity(&utility.id, std::slice::from_ref(&utility)),
            SkillLookupOutcome::Unavailable {
                id: utility.id,
                availability: SkillAvailability::Unsupported
            }
        );
    }
}
