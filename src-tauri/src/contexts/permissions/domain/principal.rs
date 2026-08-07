//! The durable "who" a policy template is assigned to.
//!
//! A `Principal` is identified by agent id alone — one row per agent, persisting across every
//! session that agent ever participates in. This deliberately matches the legacy
//! `agents.auto_approve_tools` flag's own cross-session, cross-project scope (and the
//! `agent-tool-trust` spec it implemented): trust is a property of "this agent," not "this agent
//! in this one session." Session id and generation id are per-call evaluation context (needed for
//! `Session`-scoped grants and the stale-generation guard) and travel alongside a principal
//! reference into `evaluate()`/`approval_audit`, not as part of what identifies the principal.

use super::error::PermissionsDomainError;
use super::template::PolicyTemplateName;
use serde_json::Value;

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Principal {
    id: String,
    agent_id: String,
    template: PolicyTemplateName,
    parent_principal_id: Option<String>,
    budget_config: Option<Value>,
}

impl Principal {
    /// Rejects a non-null `parent_principal_id` with `delegation_not_enabled` (design.md D2) —
    /// the column exists from Phase 1 so a future delegation phase needs no breaking migration,
    /// but delegation itself is inert until that phase activates it. This constructor is used
    /// both when creating a new principal and when reconstructing one from storage: by this same
    /// invariant, no stored Phase-1 row can ever have a non-null `parent_principal_id` either.
    ///
    /// `template` is the only durable, template-related state a principal carries — its rules
    /// (`policies_for_template`) are a pure function of the name, so there is nothing else about
    /// a template assignment that needs persisting.
    pub(crate) fn new(
        id: String,
        agent_id: String,
        template: PolicyTemplateName,
        parent_principal_id: Option<String>,
        budget_config: Option<Value>,
    ) -> Result<Self, PermissionsDomainError> {
        if id.trim().is_empty() {
            return Err(PermissionsDomainError::RequiredValue("principal id"));
        }
        if agent_id.trim().is_empty() {
            return Err(PermissionsDomainError::RequiredValue("agent id"));
        }
        if parent_principal_id.is_some() {
            return Err(PermissionsDomainError::DelegationNotEnabled);
        }
        Ok(Self {
            id,
            agent_id,
            template,
            parent_principal_id,
            budget_config,
        })
    }

    pub(crate) fn id(&self) -> &str {
        &self.id
    }

    pub(crate) fn agent_id(&self) -> &str {
        &self.agent_id
    }

    pub(crate) fn template(&self) -> PolicyTemplateName {
        self.template
    }

    pub(crate) fn reassign_template(&mut self, template: PolicyTemplateName) {
        self.template = template;
    }

    pub(crate) fn parent_principal_id(&self) -> Option<&str> {
        self.parent_principal_id.as_deref()
    }

    pub(crate) fn budget_config(&self) -> Option<&Value> {
        self.budget_config.as_ref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constructing_with_a_parent_is_rejected() {
        let result = Principal::new(
            "principal-1".to_string(),
            "agent-1".to_string(),
            PolicyTemplateName::Standard,
            Some("principal-parent".to_string()),
            None,
        );
        assert_eq!(result, Err(PermissionsDomainError::DelegationNotEnabled));
    }

    #[test]
    fn constructing_without_a_parent_succeeds() {
        let principal = Principal::new(
            "principal-1".to_string(),
            "agent-1".to_string(),
            PolicyTemplateName::Trusted,
            None,
            None,
        )
        .expect("principal without a parent should construct");
        assert_eq!(principal.id(), "principal-1");
        assert_eq!(principal.agent_id(), "agent-1");
        assert_eq!(principal.template(), PolicyTemplateName::Trusted);
        assert_eq!(principal.parent_principal_id(), None);
    }

    #[test]
    fn empty_ids_are_rejected() {
        assert!(Principal::new(
            String::new(),
            "agent-1".to_string(),
            PolicyTemplateName::Standard,
            None,
            None
        )
        .is_err());
        assert!(Principal::new(
            "principal-1".to_string(),
            String::new(),
            PolicyTemplateName::Standard,
            None,
            None
        )
        .is_err());
    }

    #[test]
    fn reassigning_the_template_changes_it() {
        let mut principal = Principal::new(
            "principal-1".to_string(),
            "agent-1".to_string(),
            PolicyTemplateName::Standard,
            None,
            None,
        )
        .expect("principal should construct");
        principal.reassign_template(PolicyTemplateName::Readonly);
        assert_eq!(principal.template(), PolicyTemplateName::Readonly);
    }
}
