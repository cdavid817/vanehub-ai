//! The Policy Decision Point: resolves `(principal, action, resource)` to an `Effect` and records
//! every decision to the audit trail.

use super::error::PermissionsApplicationError;
use super::ports::{
    AuditDecider, AuditRecord, AuditRepository, GrantQuery, GrantRepository, PermissionsClockPort,
    PermissionsIdPort, PrincipalRepository,
};
use crate::contexts::permissions::domain::{
    policies_for_template, resolve_for, risk_level_for, Action, Effect, Principal,
    PolicyTemplateName, Resource,
};
use std::sync::Arc;

#[derive(Clone)]
pub(crate) struct EvaluationService {
    principals: Arc<dyn PrincipalRepository>,
    grants: Arc<dyn GrantRepository>,
    audit: Arc<dyn AuditRepository>,
    clock: Arc<dyn PermissionsClockPort>,
    ids: Arc<dyn PermissionsIdPort>,
}

impl EvaluationService {
    pub(crate) fn new(
        principals: Arc<dyn PrincipalRepository>,
        grants: Arc<dyn GrantRepository>,
        audit: Arc<dyn AuditRepository>,
        clock: Arc<dyn PermissionsClockPort>,
        ids: Arc<dyn PermissionsIdPort>,
    ) -> Self {
        Self {
            principals,
            grants,
            audit,
            clock,
            ids,
        }
    }

    /// Resolves `(agent, action, resource)` to an `Effect`, in this exact order: (1) the MCP
    /// floor, unconditional and ahead of everything else (design.md D3); (2) a matching
    /// `Session`/`Project`/`Global` grant; (3) the agent's assigned template's rules; (4) default
    /// `Ask` if nothing matched. Every resolved decision is audited. Any internal/storage failure
    /// along the way fails closed to `Ask` — this function never returns `Allow` as a side effect
    /// of an error (design.md Risks; `permissions-core`'s "Evaluation failure fails closed").
    pub(crate) fn evaluate(
        &self,
        agent_id: &str,
        action: Action,
        resource: Resource,
        session_id: &str,
        generation_id: &str,
        project_key: &str,
    ) -> Effect {
        self.evaluate_inner(
            agent_id,
            &action,
            &resource,
            session_id,
            generation_id,
            project_key,
        )
        .unwrap_or(Effect::Ask)
    }

    fn evaluate_inner(
        &self,
        agent_id: &str,
        action: &Action,
        resource: &Resource,
        session_id: &str,
        generation_id: &str,
        project_key: &str,
    ) -> Result<Effect, PermissionsApplicationError> {
        let principal = self.get_or_create_principal(agent_id)?;

        let effect = if action.is_mcp_tool() {
            // Unconditional floor: no grant or template lookup can ever loosen this, including
            // `yolo` (design.md D3, `permissions-core`'s "MCP-sourced actions are floored at
            // Ask").
            Effect::Ask
        } else {
            let query = GrantQuery {
                principal_id: principal.id(),
                action,
                resource,
                session_id,
                project_key,
            };
            match self.grants.find_matching(&query)?.map(|grant| grant.effect) {
                Some(effect) => effect,
                None => {
                    let template_policies = policies_for_template(principal.template());
                    resolve_for(&template_policies, action, resource)
                }
            }
        };

        self.record(
            &principal,
            session_id,
            generation_id,
            action,
            resource,
            effect,
            AuditDecider::Policy,
        )?;
        Ok(effect)
    }

    /// Lazily creates a `Standard`-templated principal for an agent seen for the first time —
    /// matching the legacy `agent-tool-trust` spec's "new agents default to requiring approval"
    /// guarantee for agents registered after this migration, the same way existing agents were
    /// backfilled to `standard` unless they were already trusted.
    fn get_or_create_principal(
        &self,
        agent_id: &str,
    ) -> Result<Principal, PermissionsApplicationError> {
        if let Some(principal) = self.principals.find_by_agent_id(agent_id)? {
            return Ok(principal);
        }
        let id = self.ids.next_id("principal");
        let principal = Principal::new(
            id,
            agent_id.to_string(),
            PolicyTemplateName::Standard,
            None,
            None,
        )?;
        self.principals.create(&principal)?;
        Ok(principal)
    }

    /// Assigns a policy template to an agent's principal (`permissions-approval`'s template
    /// picker, replacing the legacy `set_agent_tool_trust` boolean toggle). Confirmation-on-
    /// increase is enforced by the caller (a command-layer/UI concern per design.md D8's
    /// refinement), not here.
    pub(crate) fn assign_template(
        &self,
        agent_id: &str,
        template: PolicyTemplateName,
    ) -> Result<Principal, PermissionsApplicationError> {
        let mut principal = self.get_or_create_principal(agent_id)?;
        principal.reassign_template(template);
        self.principals.update_template(principal.id(), template)?;
        Ok(principal)
    }

    #[allow(clippy::too_many_arguments)]
    fn record(
        &self,
        principal: &Principal,
        session_id: &str,
        generation_id: &str,
        action: &Action,
        resource: &Resource,
        effect: Effect,
        decider: AuditDecider,
    ) -> Result<(), PermissionsApplicationError> {
        self.audit.append(AuditRecord {
            id: self.ids.next_id("audit"),
            principal_id: principal.id().to_string(),
            session_id: session_id.to_string(),
            generation_id: generation_id.to_string(),
            action: action.clone(),
            resource: resource.clone(),
            effect,
            risk_level: risk_level_for(action),
            decider,
            channel: "native_agent",
            created_at: self.clock.now(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contexts::permissions::domain::{Effect, Grant, Scope};
    use std::collections::HashMap;
    use std::sync::Mutex;

    #[derive(Default)]
    struct FakePrincipals {
        by_agent: Mutex<HashMap<String, Principal>>,
    }

    impl PrincipalRepository for FakePrincipals {
        fn find_by_agent_id(
            &self,
            agent_id: &str,
        ) -> Result<Option<Principal>, PermissionsApplicationError> {
            Ok(self.by_agent.lock().unwrap().get(agent_id).cloned())
        }

        fn create(&self, principal: &Principal) -> Result<(), PermissionsApplicationError> {
            self.by_agent
                .lock()
                .unwrap()
                .insert(principal.agent_id().to_string(), principal.clone());
            Ok(())
        }

        fn update_template(
            &self,
            principal_id: &str,
            template: PolicyTemplateName,
        ) -> Result<(), PermissionsApplicationError> {
            let mut principals = self.by_agent.lock().unwrap();
            if let Some(principal) = principals.values_mut().find(|p| p.id() == principal_id) {
                principal.reassign_template(template);
            }
            Ok(())
        }
    }

    #[derive(Default)]
    struct FakeGrants {
        grants: Mutex<Vec<Grant>>,
    }

    impl GrantRepository for FakeGrants {
        fn find_matching(
            &self,
            query: &GrantQuery<'_>,
        ) -> Result<Option<Grant>, PermissionsApplicationError> {
            Ok(self
                .grants
                .lock()
                .unwrap()
                .iter()
                .find(|grant| {
                    grant.matches(
                        query.principal_id,
                        query.action,
                        query.resource,
                        query.session_id,
                        query.project_key,
                    )
                })
                .cloned())
        }

        fn create(&self, grant: &Grant) -> Result<(), PermissionsApplicationError> {
            self.grants.lock().unwrap().push(grant.clone());
            Ok(())
        }
    }

    #[derive(Default)]
    struct FakeAudit {
        records: Mutex<Vec<(String, Effect, AuditDecider)>>,
    }

    impl AuditRepository for FakeAudit {
        fn append(&self, record: AuditRecord) -> Result<(), PermissionsApplicationError> {
            self.records
                .lock()
                .unwrap()
                .push((record.action.as_str().to_string(), record.effect, record.decider));
            Ok(())
        }
    }

    struct FixedClock;
    impl PermissionsClockPort for FixedClock {
        fn now(&self) -> String {
            "0".to_string()
        }
    }

    struct FakeIds(Mutex<u64>);
    impl PermissionsIdPort for FakeIds {
        fn next_id(&self, prefix: &str) -> String {
            let mut counter = self.0.lock().unwrap();
            *counter += 1;
            format!("{prefix}-{counter}")
        }
    }

    fn service() -> (EvaluationService, Arc<FakeGrants>, Arc<FakeAudit>) {
        let principals = Arc::new(FakePrincipals::default());
        let grants = Arc::new(FakeGrants::default());
        let audit = Arc::new(FakeAudit::default());
        let service = EvaluationService::new(
            principals,
            grants.clone(),
            audit.clone(),
            Arc::new(FixedClock),
            Arc::new(FakeIds(Mutex::new(0))),
        );
        (service, grants, audit)
    }

    #[test]
    fn new_agent_defaults_to_standard_and_asks_for_shell() {
        let (service, _grants, _audit) = service();
        let effect = service.evaluate(
            "agent-1",
            Action::shell_exec(),
            crate::contexts::permissions::domain::Resource::workspace(),
            "session-1",
            "generation-1",
            "project-1",
        );
        assert_eq!(effect, Effect::Ask);
    }

    #[test]
    fn mcp_is_floored_at_ask_even_for_a_trusted_agent() {
        let (service, _grants, _audit) = service();
        service
            .assign_template("agent-1", PolicyTemplateName::Trusted)
            .expect("assign should succeed");
        let effect = service.evaluate(
            "agent-1",
            Action::mcp_tool(),
            crate::contexts::permissions::domain::Resource::mcp_tool("server", "tool"),
            "session-1",
            "generation-1",
            "project-1",
        );
        assert_eq!(effect, Effect::Ask);
    }

    #[test]
    fn trusted_agent_auto_allows_shell_and_file_write() {
        let (service, _grants, _audit) = service();
        service
            .assign_template("agent-1", PolicyTemplateName::Trusted)
            .expect("assign should succeed");
        assert_eq!(
            service.evaluate(
                "agent-1",
                Action::shell_exec(),
                crate::contexts::permissions::domain::Resource::workspace(),
                "session-1",
                "generation-1",
                "project-1",
            ),
            Effect::Allow
        );
        assert_eq!(
            service.evaluate(
                "agent-1",
                Action::file_write(),
                crate::contexts::permissions::domain::Resource::file_path("a.txt"),
                "session-1",
                "generation-1",
                "project-1",
            ),
            Effect::Allow
        );
    }

    #[test]
    fn a_matching_grant_overrides_the_templates_default_ask() {
        let (service, grants, _audit) = service();
        // Standard-templated agent normally asks for file.write; a grant should override it.
        let effect_before = service.evaluate(
            "agent-1",
            Action::file_write(),
            crate::contexts::permissions::domain::Resource::file_path("a.txt"),
            "session-1",
            "generation-1",
            "project-1",
        );
        assert_eq!(effect_before, Effect::Ask);

        grants
            .create(&Grant {
                id: "grant-1".to_string(),
                principal_id: "principal-1".to_string(),
                action: Action::file_write(),
                resource: crate::contexts::permissions::domain::Resource::file_path("a.txt"),
                effect: Effect::Allow,
                scope: Scope::Session,
                session_id: Some("session-1".to_string()),
                project_key: None,
                created_at: "0".to_string(),
            })
            .unwrap();

        let effect_after = service.evaluate(
            "agent-1",
            Action::file_write(),
            crate::contexts::permissions::domain::Resource::file_path("a.txt"),
            "session-1",
            "generation-1",
            "project-1",
        );
        assert_eq!(effect_after, Effect::Allow);
    }

    #[test]
    fn every_evaluation_is_audited() {
        let (service, _grants, audit) = service();
        service.evaluate(
            "agent-1",
            Action::file_read(),
            crate::contexts::permissions::domain::Resource::file_path("a.txt"),
            "session-1",
            "generation-1",
            "project-1",
        );
        let records = audit.records.lock().unwrap();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0], ("file.read".to_string(), Effect::Allow, AuditDecider::Policy));
    }
}
