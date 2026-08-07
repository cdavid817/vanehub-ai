//! A remembered approval decision (`permissions-core`'s "Remembered grants are consulted before
//! falling back to templates"). Only ever created for `Scope::Session`/`Project`/`Global`
//! resolutions — `Scope::Once` never persists one.

use super::action::Action;
use super::effect::Effect;
use super::resource::Resource;
use super::scope::Scope;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Grant {
    pub(crate) id: String,
    pub(crate) principal_id: String,
    pub(crate) action: Action,
    pub(crate) resource: Resource,
    pub(crate) effect: Effect,
    pub(crate) scope: Scope,
    /// Set only for `Scope::Session` — the specific session this grant is bound to.
    pub(crate) session_id: Option<String>,
    /// Set only for `Scope::Project` — the workspace/project key this grant is bound to.
    pub(crate) project_key: Option<String>,
    pub(crate) created_at: String,
}

impl Grant {
    /// Whether this grant covers an evaluation for `principal_id`/`action`/`resource` happening
    /// in `session_id` under `project_key`. `Scope::Once` grants can never match — none should
    /// exist in storage, but the check stays explicit rather than assumed.
    pub(crate) fn matches(
        &self,
        principal_id: &str,
        action: &Action,
        resource: &Resource,
        session_id: &str,
        project_key: &str,
    ) -> bool {
        if self.principal_id != principal_id || &self.action != action || &self.resource != resource
        {
            return false;
        }
        match self.scope {
            Scope::Once => false,
            Scope::Session => self.session_id.as_deref() == Some(session_id),
            Scope::Project => self.project_key.as_deref() == Some(project_key),
            Scope::Global => true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn grant(scope: Scope, session_id: Option<&str>, project_key: Option<&str>) -> Grant {
        Grant {
            id: "grant-1".to_string(),
            principal_id: "principal-1".to_string(),
            action: Action::file_write(),
            resource: Resource::file_path("a.txt"),
            effect: Effect::Allow,
            scope,
            session_id: session_id.map(str::to_string),
            project_key: project_key.map(str::to_string),
            created_at: "0".to_string(),
        }
    }

    #[test]
    fn session_scoped_grant_only_matches_its_own_session() {
        let grant = grant(Scope::Session, Some("session-1"), None);
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
        let grant = grant(Scope::Project, None, Some("project-1"));
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
        let grant = grant(Scope::Global, None, None);
        assert!(grant.matches(
            "principal-1",
            &Action::file_write(),
            &Resource::file_path("a.txt"),
            "session-anything",
            "project-anything"
        ));
    }

    #[test]
    fn once_scoped_grant_never_matches() {
        let grant = grant(Scope::Once, None, None);
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
        let grant = grant(Scope::Global, None, None);
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
}
