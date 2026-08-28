//! The Policy Decision Point: resolves `(principal, action, resource)` to an `Effect` and records
//! every decision to the audit trail.

use super::error::PermissionsApplicationError;
#[cfg(test)]
use super::ports::PendingGrantIntent;
use super::ports::{
    AuditDecider, AuditRecord, AuditRepository, DefaultTemplatePort, GrantQuery, GrantRepository,
    PermissionsClockPort, PermissionsIdPort, PrincipalRepository,
};
use crate::contexts::permissions::domain::{
    policies_for_template, resolve_for, risk_level_for, Action, Effect, PolicyTemplateName,
    Principal, Resource,
};
use std::sync::Arc;

#[derive(Clone)]
pub(crate) struct EvaluationService {
    principals: Arc<dyn PrincipalRepository>,
    grants: Arc<dyn GrantRepository>,
    audit: Arc<dyn AuditRepository>,
    clock: Arc<dyn PermissionsClockPort>,
    ids: Arc<dyn PermissionsIdPort>,
    default_template: Arc<dyn DefaultTemplatePort>,
}

impl EvaluationService {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        principals: Arc<dyn PrincipalRepository>,
        grants: Arc<dyn GrantRepository>,
        audit: Arc<dyn AuditRepository>,
        clock: Arc<dyn PermissionsClockPort>,
        ids: Arc<dyn PermissionsIdPort>,
        default_template: Arc<dyn DefaultTemplatePort>,
    ) -> Self {
        Self {
            principals,
            grants,
            audit,
            clock,
            ids,
            default_template,
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
            // Re-checked against the domain rule rather than trusted. The repository decides
            // precedence in SQL, and this is the one place that can tell a storage-side widening —
            // a wrong predicate, a stale index, an inactive row leaking through — from an
            // authorization the user actually granted. A grant that does not apply is dropped, not
            // honoured, so the failure mode is falling back to the template rather than executing.
            let effective = self.grants.find_effective_grant(&query)?.filter(|grant| {
                grant.matches(principal.id(), action, resource, session_id, project_key)
            });
            match effective.map(|grant| grant.effect.as_effect()) {
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

    /// Lazily creates a principal — templated per the configurable default (design.md D2 of
    /// `add-permissions-settings-ui`; `permissions-core`'s "Newly created principals default to
    /// a configurable template") — for an agent seen for the first time. Matches the legacy
    /// `agent-tool-trust` spec's "new agents default to requiring approval" guarantee as long as
    /// the default remains `Standard`, the same way existing agents were backfilled to `standard`
    /// unless they were already trusted.
    fn get_or_create_principal(
        &self,
        agent_id: &str,
    ) -> Result<Principal, PermissionsApplicationError> {
        // One atomic operation rather than read-then-write: two evaluations meeting a new agent at
        // the same moment would otherwise have one of them lose the unique-`agent_id` insert and
        // fail closed to Ask, which is a race presenting itself as a policy decision.
        self.principals.get_or_create(
            agent_id,
            &self.ids.next_id("principal"),
            self.default_template.default_template(),
        )
    }

    /// Reports an agent's current policy template without ever writing a principal row as a side
    /// effect (`add-permissions-settings-ui` design.md D1; `permissions-approval`'s "Reading a
    /// principal's policy template never creates it"). When no row exists yet, synthesizes an
    /// ephemeral, never-persisted `Principal` at the current default — its `id` is never read by
    /// any caller (the DTO mapping only uses `agent_id`/`template`), so a synthetic, non-colliding
    /// value is fine.
    ///
    /// The `bool` reports whether this is a real, previously-assigned row (`true`) or a
    /// synthesized default (`false`) — the Agent Policies UI needs this specifically for the
    /// `claude-code` principal, to know whether to show the first-use hook-installation
    /// confirmation (`add-claude-code-permission-callback`'s "Enabling Claude Code hook
    /// management requires a distinct first-use confirmation"). Every other caller is free to
    /// ignore it.
    pub(crate) fn find_principal(
        &self,
        agent_id: &str,
    ) -> Result<(Principal, bool), PermissionsApplicationError> {
        if let Some(principal) = self.principals.find_by_agent_id(agent_id)? {
            return Ok((principal, true));
        }
        let principal = Principal::new(
            format!("synthetic:{agent_id}"),
            agent_id.to_string(),
            self.default_template.default_template(),
            None,
            None,
        )?;
        Ok((principal, false))
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
    use crate::contexts::permissions::domain::{
        Effect, Grant, GrantActivationState, PersistedEffect, RememberedScope,
    };
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

        fn get_or_create(
            &self,
            agent_id: &str,
            id_hint: &str,
            default_template: PolicyTemplateName,
        ) -> Result<Principal, PermissionsApplicationError> {
            let mut principals = self.by_agent.lock().unwrap();
            if let Some(principal) = principals.get(agent_id) {
                return Ok(principal.clone());
            }
            let principal = Principal::new(
                id_hint.to_string(),
                agent_id.to_string(),
                default_template,
                None,
                None,
            )?;
            principals.insert(agent_id.to_string(), principal.clone());
            Ok(principal)
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
        /// Ranks in memory the way the SQL does. A fake that returned the first applicable grant
        /// would hide exactly the defect the real query was changed to remove, so the precedence
        /// rule is restated here rather than assumed.
        fn find_effective_grant(
            &self,
            query: &GrantQuery<'_>,
        ) -> Result<Option<Grant>, PermissionsApplicationError> {
            Ok(self
                .grants
                .lock()
                .unwrap()
                .iter()
                .filter(|grant| {
                    grant.matches(
                        query.principal_id,
                        query.action,
                        query.resource,
                        query.session_id,
                        query.project_key,
                    )
                })
                .max_by_key(|grant| (grant.specificity(), grant.revision))
                .cloned())
        }

        fn upsert_pending_grant_intent(
            &self,
            intent: &PendingGrantIntent,
        ) -> Result<Grant, PermissionsApplicationError> {
            let mut grants = self.grants.lock().unwrap();
            let existing = grants.iter().position(|grant| grant.key == intent.key);
            let grant = Grant {
                id: existing
                    .map(|index| grants[index].id.clone())
                    .unwrap_or_else(|| intent.id.clone()),
                key: intent.key.clone(),
                effect: intent.effect,
                revision: existing.map_or(1, |index| grants[index].revision + 1),
                activation_state: GrantActivationState::PendingDelivery,
                resolution_id: Some(intent.resolution_id.clone()),
                created_at: intent.now.clone(),
                updated_at: intent.now.clone(),
            };
            match existing {
                Some(index) => grants[index] = grant.clone(),
                None => grants.push(grant.clone()),
            }
            Ok(grant)
        }

        fn activate_grant_for_resolution(
            &self,
            resolution_id: &str,
            now: &str,
        ) -> Result<(), PermissionsApplicationError> {
            for grant in self.grants.lock().unwrap().iter_mut() {
                if grant.resolution_id.as_deref() == Some(resolution_id)
                    && grant.activation_state == GrantActivationState::PendingDelivery
                {
                    grant.activation_state = GrantActivationState::Active;
                    grant.updated_at = now.to_string();
                }
            }
            Ok(())
        }
    }

    #[derive(Default)]
    struct FakeAudit {
        records: Mutex<Vec<(String, Effect, AuditDecider)>>,
    }

    impl AuditRepository for FakeAudit {
        fn append(&self, record: AuditRecord) -> Result<(), PermissionsApplicationError> {
            self.records.lock().unwrap().push((
                record.action.as_str().to_string(),
                record.effect,
                record.decider,
            ));
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

    struct FakeDefaultTemplate(Mutex<PolicyTemplateName>);
    impl DefaultTemplatePort for FakeDefaultTemplate {
        fn default_template(&self) -> PolicyTemplateName {
            *self.0.lock().unwrap()
        }
    }

    fn service() -> (EvaluationService, Arc<FakeGrants>, Arc<FakeAudit>) {
        let (service, grants, audit, _principals, _default_template) =
            service_with_default(PolicyTemplateName::Standard);
        (service, grants, audit)
    }

    #[allow(clippy::type_complexity)]
    fn service_with_default(
        default_template: PolicyTemplateName,
    ) -> (
        EvaluationService,
        Arc<FakeGrants>,
        Arc<FakeAudit>,
        Arc<FakePrincipals>,
        Arc<FakeDefaultTemplate>,
    ) {
        let principals = Arc::new(FakePrincipals::default());
        let grants = Arc::new(FakeGrants::default());
        let audit = Arc::new(FakeAudit::default());
        let default_template = Arc::new(FakeDefaultTemplate(Mutex::new(default_template)));
        let service = EvaluationService::new(
            principals.clone(),
            grants.clone(),
            audit.clone(),
            Arc::new(FixedClock),
            Arc::new(FakeIds(Mutex::new(0))),
            default_template.clone(),
        );
        (service, grants, audit, principals, default_template)
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

        grants.grants.lock().unwrap().push(Grant::active_for_test(
            "grant-1",
            "principal-1",
            Action::file_write(),
            crate::contexts::permissions::domain::Resource::file_path("a.txt"),
            PersistedEffect::Allow,
            RememberedScope::Session("session-1".to_string()),
        ));

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
        assert_eq!(
            records[0],
            ("file.read".to_string(), Effect::Allow, AuditDecider::Policy)
        );
    }

    #[test]
    fn new_agent_inherits_the_configured_default_template() {
        let (service, _grants, _audit, _principals, _default_template) =
            service_with_default(PolicyTemplateName::Trusted);
        assert_eq!(
            service.evaluate(
                "agent-1",
                Action::shell_exec(),
                crate::contexts::permissions::domain::Resource::workspace(),
                "session-1",
                "generation-1",
                "project-1",
            ),
            Effect::Allow,
            "a Trusted-defaulted new agent should auto-allow shell.exec, not ask"
        );
    }

    #[test]
    fn changing_the_default_after_a_principal_exists_does_not_retroactively_change_it() {
        let (service, _grants, _audit, _principals, default_template) =
            service_with_default(PolicyTemplateName::Standard);
        // First evaluation lazily creates the principal at the Standard default.
        assert_eq!(
            service.evaluate(
                "agent-1",
                Action::shell_exec(),
                crate::contexts::permissions::domain::Resource::workspace(),
                "session-1",
                "generation-1",
                "project-1",
            ),
            Effect::Ask
        );

        *default_template.0.lock().unwrap() = PolicyTemplateName::Trusted;

        assert_eq!(
            service.evaluate(
                "agent-1",
                Action::shell_exec(),
                crate::contexts::permissions::domain::Resource::workspace(),
                "session-1",
                "generation-1",
                "project-1",
            ),
            Effect::Ask,
            "an already-created principal must keep its own assigned template, not follow later default changes"
        );
    }

    #[test]
    fn find_principal_round_trips_an_existing_row_without_rewriting_it() {
        let (service, _grants, _audit, principals, _default_template) =
            service_with_default(PolicyTemplateName::Standard);
        service
            .assign_template("agent-1", PolicyTemplateName::Readonly)
            .expect("assign should succeed");

        let (found, has_explicit_assignment) = service
            .find_principal("agent-1")
            .expect("find should succeed");

        assert_eq!(found.template(), PolicyTemplateName::Readonly);
        assert!(has_explicit_assignment);
        assert_eq!(principals.by_agent.lock().unwrap().len(), 1);
    }

    #[test]
    fn find_principal_synthesizes_the_default_without_creating_a_row() {
        let (service, _grants, _audit, principals, _default_template) =
            service_with_default(PolicyTemplateName::Trusted);

        let (found, has_explicit_assignment) = service
            .find_principal("never-seen-agent")
            .expect("find should succeed");

        assert_eq!(found.agent_id(), "never-seen-agent");
        assert_eq!(found.template(), PolicyTemplateName::Trusted);
        assert!(!has_explicit_assignment);
        assert!(
            principals.by_agent.lock().unwrap().is_empty(),
            "reading a never-seen agent's principal must not create a row"
        );
    }

    #[test]
    fn claude_code_principal_defaults_to_the_configured_template_like_any_other_agent() {
        // add-claude-code-permission-callback design.md D2: claude-code is one global principal,
        // identified by agent id alone like every other principal — no special-cased identity
        // model needed for the CLI hook bridge.
        let (service, _grants, _audit, principals, _default_template) =
            service_with_default(PolicyTemplateName::Trusted);

        let (found, has_explicit_assignment) = service
            .find_principal("claude-code")
            .expect("find should succeed");

        assert_eq!(found.agent_id(), "claude-code");
        assert_eq!(found.template(), PolicyTemplateName::Trusted);
        assert!(!has_explicit_assignment);
        assert!(
            principals.by_agent.lock().unwrap().is_empty(),
            "reading the claude-code principal before any explicit assignment must not create a row"
        );
    }

    #[test]
    fn find_principal_reflects_a_changed_default_with_no_caching() {
        let (service, _grants, _audit, _principals, default_template) =
            service_with_default(PolicyTemplateName::Standard);
        assert_eq!(
            service.find_principal("agent-1").unwrap().0.template(),
            PolicyTemplateName::Standard
        );

        *default_template.0.lock().unwrap() = PolicyTemplateName::Readonly;

        assert_eq!(
            service.find_principal("agent-1").unwrap().0.template(),
            PolicyTemplateName::Readonly
        );
    }
}
