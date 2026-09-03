//! What identifies one remembered decision, and which of several applicable ones wins.
//!
//! A remembered grant used to be a row: repeating a decision appended another one, and evaluation
//! took whichever the table handed back first. That makes the effective permission a function of
//! insertion order, row id, and query plan — three things no security rule should depend on. The
//! types here replace that with a stated identity and a stated order.
//!
//! Two ideas carry the whole module. A *canonical key* is the tuple a decision is about, including
//! what the scope is bound to, so "remember this again" is an update rather than an append. And
//! *specificity* ranks the keys that apply to one evaluation, so a session decision overrides a
//! project one and a project decision overrides a global one — deliberately including when the
//! broader row is a `Deny`, because the narrower row is the more recent, more informed statement
//! about a narrower situation.

use super::action::Action;
use super::effect::Effect;
use super::error::PermissionsDomainError;
use super::resource::Resource;
use super::scope::Scope;

/// What a remembered grant is bound to, inseparable from which kind of binding it is.
///
/// Modelled as one value rather than a `Scope` beside two `Option<String>`s because three of the
/// four combinations that shape allows are meaningless: a session grant with no session, a project
/// grant with no project, and a global grant that names one anyway. Keeping them representable
/// means every reader has to re-check them, and the schema has to trust that every writer did.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) enum RememberedScope {
    Session(String),
    Project(String),
    Global,
}

impl RememberedScope {
    /// Builds the binding from the wire shape — a scope plus the two optional owners a row carries.
    ///
    /// Rejects rather than repairs. A session grant whose session is missing is not a global grant
    /// with extra fields; it is a row nobody can say the meaning of, and silently widening it would
    /// turn a lost owner into an authorization that covers every session.
    pub(crate) fn parse(
        scope: Scope,
        session_id: Option<&str>,
        project_key: Option<&str>,
    ) -> Result<Self, PermissionsDomainError> {
        let session = session_id.filter(|value| !value.is_empty());
        let project = project_key.filter(|value| !value.is_empty());
        match (scope, session, project) {
            (Scope::Once, _, _) => Err(PermissionsDomainError::UnrememberableScope),
            (Scope::Session, Some(session), None) => Ok(Self::Session(session.to_string())),
            (Scope::Project, None, Some(project)) => Ok(Self::Project(project.to_string())),
            (Scope::Global, None, None) => Ok(Self::Global),
            _ => Err(PermissionsDomainError::ScopeOwnerMismatch),
        }
    }

    pub(crate) fn scope(&self) -> Scope {
        match self {
            Self::Session(_) => Scope::Session,
            Self::Project(_) => Scope::Project,
            Self::Global => Scope::Global,
        }
    }

    pub(crate) fn session_id(&self) -> Option<&str> {
        match self {
            Self::Session(session) => Some(session),
            _ => None,
        }
    }

    pub(crate) fn project_key(&self) -> Option<&str> {
        match self {
            Self::Project(project) => Some(project),
            _ => None,
        }
    }

    /// How narrow this binding is. Higher wins.
    ///
    /// Ranked on the binding alone, never on the effect: specificity is evaluated first so that a
    /// session `Allow` beats a global `Deny`. That is the intended rule — the narrower row is the
    /// later, more specific answer to a narrower question — and folding the effect in here is how
    /// a "deny always wins" instinct would quietly reintroduce order-dependence.
    ///
    /// The production read of this rule is the SQL `CASE` in `find_effective_grant`; this is the
    /// statement the fakes and the domain tests hold that query to. Deleting it would leave the
    /// precedence rule expressed only in a string.
    #[cfg_attr(not(test), expect(dead_code))]
    pub(crate) fn specificity(&self) -> u8 {
        match self {
            Self::Session(_) => 3,
            Self::Project(_) => 2,
            Self::Global => 1,
        }
    }

    /// Whether this binding covers an evaluation happening in `session_id` under `project_key`.
    pub(crate) fn applies_to(&self, session_id: &str, project_key: &str) -> bool {
        match self {
            Self::Session(session) => session == session_id,
            Self::Project(project) => project == project_key,
            Self::Global => true,
        }
    }
}

/// An effect that can actually be remembered.
///
/// `Ask` is not one of them, and the type is what says so. `Ask` means "nobody has decided yet";
/// storing it as a remembered decision would record the absence of a decision as one, and the
/// evaluation that then found it would stop before reaching the template that had a real answer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum PersistedEffect {
    Allow,
    Deny,
}

impl PersistedEffect {
    pub(crate) fn parse(effect: Effect) -> Result<Self, PermissionsDomainError> {
        match effect {
            Effect::Allow => Ok(Self::Allow),
            Effect::Deny => Ok(Self::Deny),
            Effect::Ask => Err(PermissionsDomainError::UnrememberableEffect),
        }
    }

    pub(crate) fn as_effect(self) -> Effect {
        match self {
            Self::Allow => Effect::Allow,
            Self::Deny => Effect::Deny,
        }
    }

    pub(crate) fn token(self) -> &'static str {
        match self {
            Self::Allow => "allow",
            Self::Deny => "deny",
        }
    }

    /// Parses a stored token. Unknown text is refused rather than defaulted: the old reader mapped
    /// anything it did not recognise to `Ask`, which turned a corrupt row into a silent no-op
    /// instead of something a migration or an invariant check could see.
    pub(crate) fn from_token(token: &str) -> Result<Self, PermissionsDomainError> {
        match token {
            "allow" => Ok(Self::Allow),
            "deny" => Ok(Self::Deny),
            _ => Err(PermissionsDomainError::UnrememberableEffect),
        }
    }
}

/// Whether a remembered grant is allowed to influence evaluation yet.
///
/// A grant is written as part of the same transaction that records the decision, but the decision
/// has not reached the waiting agent at that point. Until delivery is acknowledged the row exists
/// as intent only: activating it earlier would let an approval that never actually reached anyone
/// authorize the *next* attempt.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum GrantActivationState {
    PendingDelivery,
    Active,
}

impl GrantActivationState {
    pub(crate) fn token(self) -> &'static str {
        match self {
            Self::PendingDelivery => "pending_delivery",
            Self::Active => "active",
        }
    }

    pub(crate) fn from_token(token: &str) -> Result<Self, PermissionsDomainError> {
        match token {
            "pending_delivery" => Ok(Self::PendingDelivery),
            "active" => Ok(Self::Active),
            _ => Err(PermissionsDomainError::UnknownActivationState),
        }
    }
}

/// The identity of one remembered decision.
///
/// Everything that makes two decisions "the same decision" and nothing that does not: no row id, no
/// timestamp, no effect. Remembering an `Allow` and later a `Deny` for this tuple are two values of
/// one key, which is why the second replaces the first instead of joining it.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct CanonicalGrantKey {
    pub(crate) principal_id: String,
    pub(crate) action: Action,
    pub(crate) resource: Resource,
    pub(crate) scope: RememberedScope,
}

impl CanonicalGrantKey {
    pub(crate) fn new(
        principal_id: impl Into<String>,
        action: Action,
        resource: Resource,
        scope: RememberedScope,
    ) -> Result<Self, PermissionsDomainError> {
        let principal_id = principal_id.into();
        if principal_id.is_empty() {
            return Err(PermissionsDomainError::RequiredValue("principal_id"));
        }
        Ok(Self {
            principal_id,
            action,
            resource,
            scope,
        })
    }

    /// Whether this key is one of the ones an evaluation should consider.
    ///
    /// Exact on principal, action, and resource — this change adds no wildcard, prefix, or
    /// path-normalisation matching, and a test in the repository holds that line.
    pub(crate) fn applies_to(
        &self,
        principal_id: &str,
        action: &Action,
        resource: &Resource,
        session_id: &str,
        project_key: &str,
    ) -> bool {
        self.principal_id == principal_id
            && &self.action == action
            && &self.resource == resource
            && self.scope.applies_to(session_id, project_key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(scope: RememberedScope) -> CanonicalGrantKey {
        CanonicalGrantKey::new(
            "principal-1",
            Action::file_write(),
            Resource::file_path("a.txt"),
            scope,
        )
        .expect("well-formed key")
    }

    #[test]
    fn a_once_scope_can_never_become_a_remembered_binding() {
        assert_eq!(
            RememberedScope::parse(Scope::Once, None, None),
            Err(PermissionsDomainError::UnrememberableScope)
        );
        assert_eq!(
            RememberedScope::parse(Scope::Once, Some("session-1"), None),
            Err(PermissionsDomainError::UnrememberableScope)
        );
    }

    #[test]
    fn a_binding_must_carry_exactly_its_own_owner() {
        assert!(RememberedScope::parse(Scope::Session, Some("session-1"), None).is_ok());
        assert!(RememberedScope::parse(Scope::Project, None, Some("project-1")).is_ok());
        assert!(RememberedScope::parse(Scope::Global, None, None).is_ok());

        for (scope, session, project) in [
            (Scope::Session, None, None),
            (Scope::Session, None, Some("project-1")),
            (Scope::Session, Some("session-1"), Some("project-1")),
            (Scope::Project, None, None),
            (Scope::Project, Some("session-1"), None),
            (Scope::Global, Some("session-1"), None),
            (Scope::Global, None, Some("project-1")),
        ] {
            assert_eq!(
                RememberedScope::parse(scope, session, project),
                Err(PermissionsDomainError::ScopeOwnerMismatch),
                "{scope:?} with session {session:?} and project {project:?} was accepted"
            );
        }
    }

    #[test]
    fn an_empty_owner_string_is_a_missing_owner() {
        // A row that stored `''` instead of `NULL` is the same defect wearing different clothes:
        // treating it as present would produce a session grant bound to no session.
        assert_eq!(
            RememberedScope::parse(Scope::Session, Some(""), None),
            Err(PermissionsDomainError::ScopeOwnerMismatch)
        );
        assert_eq!(
            RememberedScope::parse(Scope::Global, Some(""), Some("")),
            Ok(RememberedScope::Global)
        );
    }

    #[test]
    fn specificity_ranks_session_above_project_above_global() {
        assert!(
            RememberedScope::Session("session-1".into()).specificity()
                > RememberedScope::Project("project-1".into()).specificity()
        );
        assert!(
            RememberedScope::Project("project-1".into()).specificity()
                > RememberedScope::Global.specificity()
        );
    }

    #[test]
    fn every_binding_matches_only_the_evaluations_it_covers() {
        let session = RememberedScope::Session("session-1".into());
        assert!(session.applies_to("session-1", "project-1"));
        assert!(!session.applies_to("session-2", "project-1"));

        let project = RememberedScope::Project("project-1".into());
        assert!(project.applies_to("session-9", "project-1"));
        assert!(!project.applies_to("session-9", "project-2"));

        assert!(RememberedScope::Global.applies_to("anything", "anywhere"));
    }

    #[test]
    fn a_narrower_binding_outranks_a_broader_one_whatever_the_effects_are() {
        // The combination the requirement is written about: the session row says Allow while both
        // broader rows say Deny. Specificity is evaluated before effect, so the session row wins.
        let applicable = [
            (key(RememberedScope::Global), PersistedEffect::Deny),
            (
                key(RememberedScope::Project("project-1".into())),
                PersistedEffect::Deny,
            ),
            (
                key(RememberedScope::Session("session-1".into())),
                PersistedEffect::Allow,
            ),
        ];
        let winner = applicable
            .iter()
            .filter(|(key, _)| {
                key.applies_to(
                    "principal-1",
                    &Action::file_write(),
                    &Resource::file_path("a.txt"),
                    "session-1",
                    "project-1",
                )
            })
            .max_by_key(|(key, _)| key.scope.specificity())
            .expect("one of them applies");
        assert_eq!(winner.1, PersistedEffect::Allow);
    }

    #[test]
    fn a_key_never_matches_a_different_principal_action_or_resource() {
        let key = key(RememberedScope::Global);
        assert!(!key.applies_to(
            "principal-2",
            &Action::file_write(),
            &Resource::file_path("a.txt"),
            "session-1",
            "project-1"
        ));
        assert!(!key.applies_to(
            "principal-1",
            &Action::shell_exec(),
            &Resource::file_path("a.txt"),
            "session-1",
            "project-1"
        ));
        assert!(!key.applies_to(
            "principal-1",
            &Action::file_write(),
            &Resource::file_path("b.txt"),
            "session-1",
            "project-1"
        ));
    }

    #[test]
    fn a_resource_that_merely_starts_with_another_is_not_a_match() {
        // Guards the non-goal explicitly: this change adds exact matching only, and a prefix rule
        // introduced by accident would silently widen every stored grant.
        let key = CanonicalGrantKey::new(
            "principal-1",
            Action::file_write(),
            Resource::file_path("src"),
            RememberedScope::Global,
        )
        .expect("well-formed key");
        assert!(!key.applies_to(
            "principal-1",
            &Action::file_write(),
            &Resource::file_path("src/lib.rs"),
            "session-1",
            "project-1"
        ));
    }

    #[test]
    fn ask_is_not_a_rememberable_effect() {
        assert_eq!(
            PersistedEffect::parse(Effect::Ask),
            Err(PermissionsDomainError::UnrememberableEffect)
        );
        assert_eq!(
            PersistedEffect::parse(Effect::Allow),
            Ok(PersistedEffect::Allow)
        );
        assert_eq!(
            PersistedEffect::parse(Effect::Deny),
            Ok(PersistedEffect::Deny)
        );
    }

    #[test]
    fn stored_tokens_round_trip_and_unknown_text_is_refused() {
        for effect in [PersistedEffect::Allow, PersistedEffect::Deny] {
            assert_eq!(PersistedEffect::from_token(effect.token()), Ok(effect));
        }
        // The old reader mapped anything unrecognised to `Ask`, which is how a corrupt row became
        // an invisible no-op rather than something the invariant check could report.
        assert!(PersistedEffect::from_token("ask").is_err());
        assert!(PersistedEffect::from_token("").is_err());

        for state in [
            GrantActivationState::PendingDelivery,
            GrantActivationState::Active,
        ] {
            assert_eq!(GrantActivationState::from_token(state.token()), Ok(state));
        }
        assert!(GrantActivationState::from_token("delivered").is_err());
    }

    #[test]
    fn a_key_needs_a_principal() {
        assert_eq!(
            CanonicalGrantKey::new(
                "",
                Action::file_write(),
                Resource::file_path("a.txt"),
                RememberedScope::Global
            ),
            Err(PermissionsDomainError::RequiredValue("principal_id"))
        );
    }
}
