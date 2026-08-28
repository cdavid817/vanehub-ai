//! A remembered approval decision (`permissions-core`'s "Remembered grants are consulted before
//! falling back to templates").
//!
//! A grant is one *value* of a [`CanonicalGrantKey`], not a row appended to a list. Everything that
//! decides whether two grants are the same decision lives in the key; everything that can change
//! while the decision stays the same — the effect, which revision it is, whether its delivery has
//! been acknowledged — lives here. That split is what makes "remember this again" an update, and
//! what lets storage enforce one effective row per key instead of hoping writers behave.

use super::action::Action;
#[cfg(test)]
use super::canonical_grant_key::RememberedScope;
use super::canonical_grant_key::{CanonicalGrantKey, GrantActivationState, PersistedEffect};
use super::resource::Resource;
use super::scope::Scope;

/// The revision every newly remembered key starts at. Monotonic per key, so a reader can tell which
/// of two values for one key is the later statement without consulting a clock.
///
/// Storage writes the literal `1` in its `INSERT`, because the column default and the constant
/// cannot be shared across the SQL boundary; this is the name the fixtures and the migration tests
/// assert against.
#[cfg_attr(not(test), expect(dead_code))]
pub(crate) const FIRST_GRANT_REVISION: i64 = 1;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Grant {
    pub(crate) id: String,
    pub(crate) key: CanonicalGrantKey,
    pub(crate) effect: PersistedEffect,
    /// Increments each time this key is remembered again. Highest wins within one key; normal
    /// operation keeps exactly one row, and the revision is what makes a recovered or replicated
    /// pair resolvable rather than ambiguous.
    pub(crate) revision: i64,
    pub(crate) activation_state: GrantActivationState,
    /// The resolution whose delivery gates activation. `None` only for rows that predate the
    /// resolution ledger and were normalised into `Active` by migration.
    pub(crate) resolution_id: Option<String>,
    pub(crate) created_at: String,
    pub(crate) updated_at: String,
}

impl Grant {
    /// The wire shape of the binding, for readers that still see three columns.
    ///
    /// Storage writes those columns from the intent rather than from a `Grant`, so these three
    /// accessors have no production caller today; they are what the repository and migration tests
    /// assert a round-tripped row against.
    #[cfg_attr(not(test), expect(dead_code))]
    pub(crate) fn scope(&self) -> Scope {
        self.key.scope.scope()
    }

    #[cfg_attr(not(test), expect(dead_code))]
    pub(crate) fn session_id(&self) -> Option<&str> {
        self.key.scope.session_id()
    }

    #[cfg_attr(not(test), expect(dead_code))]
    pub(crate) fn project_key(&self) -> Option<&str> {
        self.key.scope.project_key()
    }

    /// Whether this grant may take part in an evaluation at all.
    ///
    /// Asked separately from whether the key applies, because a grant that exists is not yet a
    /// grant that counts: until the approval that produced it was acknowledged as delivered, it is
    /// recorded intent. Evaluation consults only active rows.
    pub(crate) fn is_active(&self) -> bool {
        matches!(self.activation_state, GrantActivationState::Active)
    }

    /// Whether this grant covers an evaluation for `principal_id`/`action`/`resource` happening in
    /// `session_id` under `project_key`, and is active.
    ///
    /// Activation is folded in deliberately. Every caller that asks "does this grant apply" is
    /// about to act on the answer, and a variant that answered "yes, but it is not active yet"
    /// would be one call site away from authorizing an undelivered approval.
    pub(crate) fn matches(
        &self,
        principal_id: &str,
        action: &Action,
        resource: &Resource,
        session_id: &str,
        project_key: &str,
    ) -> bool {
        self.is_active()
            && self
                .key
                .applies_to(principal_id, action, resource, session_id, project_key)
    }

    /// How narrow this grant's binding is. Higher wins; see [`RememberedScope::specificity`].
    ///
    /// Used by the in-memory fakes, which have to rank the same way the SQL does — a fake that
    /// returned the first applicable grant would hide the very defect the query was changed to
    /// remove.
    #[cfg_attr(not(test), expect(dead_code))]
    pub(crate) fn specificity(&self) -> u8 {
        self.key.scope.specificity()
    }
}

#[cfg(test)]
impl Grant {
    /// A test-only constructor for the common shape: one active grant at the first revision.
    ///
    /// Exists so a test states the two things it is about — the binding and the effect — instead of
    /// restating seven fields it does not care about, which is how a fixture drifts into asserting
    /// the default rather than the case.
    pub(crate) fn active_for_test(
        id: &str,
        principal_id: &str,
        action: Action,
        resource: Resource,
        effect: PersistedEffect,
        scope: RememberedScope,
    ) -> Self {
        Self {
            id: id.to_string(),
            key: CanonicalGrantKey::new(principal_id, action, resource, scope)
                .expect("well-formed key"),
            effect,
            revision: FIRST_GRANT_REVISION,
            activation_state: GrantActivationState::Active,
            resolution_id: None,
            created_at: "0".to_string(),
            updated_at: "0".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn grant(scope: RememberedScope) -> Grant {
        Grant::active_for_test(
            "grant-1",
            "principal-1",
            Action::file_write(),
            Resource::file_path("a.txt"),
            PersistedEffect::Allow,
            scope,
        )
    }

    #[test]
    fn session_scoped_grant_only_matches_its_own_session() {
        let grant = grant(RememberedScope::Session("session-1".into()));
        assert!(grant.matches(
            "principal-1",
            &Action::file_write(),
            &Resource::file_path("a.txt"),
            "session-1",
            "project-1"
        ));
        assert!(!grant.matches(
            "principal-1",
            &Action::file_write(),
            &Resource::file_path("a.txt"),
            "session-2",
            "project-1"
        ));
    }

    #[test]
    fn project_scoped_grant_only_matches_its_own_project() {
        let grant = grant(RememberedScope::Project("project-1".into()));
        assert!(grant.matches(
            "principal-1",
            &Action::file_write(),
            &Resource::file_path("a.txt"),
            "session-1",
            "project-1"
        ));
        assert!(!grant.matches(
            "principal-1",
            &Action::file_write(),
            &Resource::file_path("a.txt"),
            "session-1",
            "project-2"
        ));
    }

    #[test]
    fn global_scoped_grant_matches_any_session_or_project() {
        let grant = grant(RememberedScope::Global);
        assert!(grant.matches(
            "principal-1",
            &Action::file_write(),
            &Resource::file_path("a.txt"),
            "session-anything",
            "project-anything"
        ));
    }

    #[test]
    fn a_grant_awaiting_delivery_matches_nothing_yet() {
        // The whole point of the two-phase activation: the row is durable, the approval it came
        // from may never have reached the agent, and evaluation must not act on it until it did.
        let mut grant = grant(RememberedScope::Global);
        grant.activation_state = GrantActivationState::PendingDelivery;
        assert!(!grant.is_active());
        assert!(!grant.matches(
            "principal-1",
            &Action::file_write(),
            &Resource::file_path("a.txt"),
            "session-1",
            "project-1"
        ));
    }

    #[test]
    fn mismatched_principal_action_or_resource_never_matches() {
        let grant = grant(RememberedScope::Global);
        assert!(!grant.matches(
            "principal-2",
            &Action::file_write(),
            &Resource::file_path("a.txt"),
            "session-1",
            "project-1"
        ));
        assert!(!grant.matches(
            "principal-1",
            &Action::shell_exec(),
            &Resource::file_path("a.txt"),
            "session-1",
            "project-1"
        ));
        assert!(!grant.matches(
            "principal-1",
            &Action::file_write(),
            &Resource::file_path("b.txt"),
            "session-1",
            "project-1"
        ));
    }

    #[test]
    fn the_wire_shape_of_a_binding_is_recoverable_from_the_grant() {
        // Storage still writes `scope`, `session_id` and `project_key` as three columns, so the
        // grant has to be able to say what each of them is without the caller destructuring the
        // binding itself.
        let session = grant(RememberedScope::Session("session-1".into()));
        assert_eq!(session.scope(), Scope::Session);
        assert_eq!(session.session_id(), Some("session-1"));
        assert_eq!(session.project_key(), None);

        let project = grant(RememberedScope::Project("project-1".into()));
        assert_eq!(project.scope(), Scope::Project);
        assert_eq!(project.session_id(), None);
        assert_eq!(project.project_key(), Some("project-1"));

        let global = grant(RememberedScope::Global);
        assert_eq!(global.scope(), Scope::Global);
        assert_eq!(global.session_id(), None);
        assert_eq!(global.project_key(), None);
    }
}
