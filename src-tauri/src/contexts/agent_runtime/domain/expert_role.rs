//! Expert roles are reusable job descriptions that a session seat binds to an Agent.
//!
//! A role is deliberately not an Agent attribute: the same installed CLI must be able to review in
//! one session and architect in another, so the binding belongs to the seat, not to the tool.

use super::AgentRuntimeDomainError;

/// Whether a seat holding this role may be recommended as a peer reviewer, and whether that
/// recommendation should prefer a different model family.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ExpertRoleReviewPolicy {
    pub(crate) peer_reviewer: bool,
    pub(crate) require_different_family: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ExpertRoleOrigin {
    Builtin,
    User,
}

impl ExpertRoleOrigin {
    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            Self::Builtin => "builtin",
            Self::User => "user",
        }
    }

    pub(crate) fn parse(value: &str) -> Self {
        match value {
            "builtin" => Self::Builtin,
            _ => Self::User,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ExpertRole {
    pub(crate) id: String,
    pub(crate) display_name: String,
    pub(crate) avatar: String,
    pub(crate) color: String,
    /// Published to other seats as the basis for choosing whom to hand off to, so it is required.
    pub(crate) responsibility: String,
    pub(crate) instruction: String,
    pub(crate) skill_ids: Vec<String>,
    pub(crate) review_policy: ExpertRoleReviewPolicy,
    pub(crate) preferred_providers: Vec<String>,
    pub(crate) origin: ExpertRoleOrigin,
    pub(crate) created_at: String,
    pub(crate) updated_at: String,
}

pub(crate) struct ExpertRoleInput {
    pub(crate) display_name: String,
    pub(crate) avatar: String,
    pub(crate) color: String,
    pub(crate) responsibility: String,
    pub(crate) instruction: String,
    pub(crate) skill_ids: Vec<String>,
    pub(crate) review_policy: ExpertRoleReviewPolicy,
    pub(crate) preferred_providers: Vec<String>,
}

impl ExpertRole {
    /// Mirrors `validateExpertRoleInput` on the frontend so both runtimes reject the same inputs.
    pub(crate) fn new(
        id: String,
        input: ExpertRoleInput,
        created_at: String,
        updated_at: String,
    ) -> Result<Self, AgentRuntimeDomainError> {
        let display_name = required(input.display_name, "expert role display name")?;
        let responsibility = required(input.responsibility, "expert role responsibility")?;
        let instruction = required(input.instruction, "expert role instruction")?;
        if !is_hex_color(&input.color) {
            return Err(AgentRuntimeDomainError::InvalidExpertRole(
                "expert role color must be a hex value".to_string(),
            ));
        }
        let mut seen = std::collections::BTreeSet::new();
        for skill_id in &input.skill_ids {
            if !seen.insert(skill_id.clone()) {
                return Err(AgentRuntimeDomainError::InvalidExpertRole(format!(
                    "expert role repeats Skill '{skill_id}'"
                )));
            }
        }
        if input.review_policy.require_different_family && !input.review_policy.peer_reviewer {
            return Err(AgentRuntimeDomainError::InvalidExpertRole(
                "requiring a different model family needs peer review enabled".to_string(),
            ));
        }
        Ok(Self {
            id,
            display_name,
            avatar: input.avatar,
            color: input.color,
            responsibility,
            instruction,
            skill_ids: input.skill_ids,
            review_policy: input.review_policy,
            preferred_providers: input.preferred_providers,
            origin: ExpertRoleOrigin::User,
            created_at,
            updated_at,
        })
    }
}

fn required(value: String, field: &str) -> Result<String, AgentRuntimeDomainError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(AgentRuntimeDomainError::InvalidExpertRole(format!(
            "{field} is required"
        )));
    }
    Ok(trimmed.to_string())
}

fn is_hex_color(value: &str) -> bool {
    value.len() == 7
        && value.starts_with('#')
        && value[1..]
            .chars()
            .all(|character| character.is_ascii_hexdigit())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input() -> ExpertRoleInput {
        ExpertRoleInput {
            display_name: "架构师".to_string(),
            avatar: "🏛".to_string(),
            color: "#9B7EBD".to_string(),
            responsibility: "负责系统设计".to_string(),
            instruction: "你是架构师".to_string(),
            skill_ids: vec![],
            review_policy: ExpertRoleReviewPolicy {
                peer_reviewer: false,
                require_different_family: false,
            },
            preferred_providers: vec![],
        }
    }

    #[test]
    fn accepts_a_complete_role() {
        let role = ExpertRole::new("r1".to_string(), input(), "t".to_string(), "t".to_string());
        assert!(role.is_ok());
    }

    #[test]
    fn rejects_a_blank_responsibility() {
        let role = ExpertRole::new(
            "r1".to_string(),
            ExpertRoleInput {
                responsibility: "   ".to_string(),
                ..input()
            },
            "t".to_string(),
            "t".to_string(),
        );
        assert!(
            role.is_err(),
            "responsibility drives handoff routing and cannot be blank"
        );
    }

    #[test]
    fn rejects_requiring_a_different_family_without_peer_review() {
        let role = ExpertRole::new(
            "r1".to_string(),
            ExpertRoleInput {
                review_policy: ExpertRoleReviewPolicy {
                    peer_reviewer: false,
                    require_different_family: true,
                },
                ..input()
            },
            "t".to_string(),
            "t".to_string(),
        );
        assert!(role.is_err());
    }

    #[test]
    fn rejects_a_non_hex_color() {
        let role = ExpertRole::new(
            "r1".to_string(),
            ExpertRoleInput {
                color: "purple".to_string(),
                ..input()
            },
            "t".to_string(),
            "t".to_string(),
        );
        assert!(role.is_err());
    }
}
