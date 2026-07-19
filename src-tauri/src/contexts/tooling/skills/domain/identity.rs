use super::SkillDomainError;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct SkillId(String);

impl SkillId {
    pub(crate) fn parse(value: impl Into<String>) -> Result<Self, SkillDomainError> {
        let value = value.into();
        if is_kebab_case(&value) {
            Ok(Self(value))
        } else {
            Err(SkillDomainError::InvalidId)
        }
    }

    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum SkillScope {
    Global,
    Workspace,
}

impl SkillScope {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Global => "global",
            Self::Workspace => "workspace",
        }
    }

    pub(crate) fn parse(value: &str) -> Option<Self> {
        match value {
            "global" => Some(Self::Global),
            "workspace" => Some(Self::Workspace),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct SkillLocation {
    pub(crate) scope: SkillScope,
    pub(crate) workspace_path: Option<String>,
}

impl SkillLocation {
    pub(crate) fn new(
        scope: SkillScope,
        workspace_path: Option<&str>,
    ) -> Result<Self, SkillDomainError> {
        let workspace_path = match scope {
            SkillScope::Global => None,
            SkillScope::Workspace => Some(
                workspace_path
                    .map(str::trim)
                    .filter(|path| !path.is_empty())
                    .ok_or(SkillDomainError::WorkspacePathRequired)?
                    .to_string(),
            ),
        };
        Ok(Self {
            scope,
            workspace_path,
        })
    }

    pub(crate) fn storage_workspace_key(&self) -> &str {
        self.workspace_path.as_deref().unwrap_or("")
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct SkillKey {
    pub(crate) id: SkillId,
    pub(crate) location: SkillLocation,
}

impl SkillKey {
    pub(crate) fn new(id: SkillId, location: SkillLocation) -> Self {
        Self { id, location }
    }
}

fn is_kebab_case(value: &str) -> bool {
    if value.is_empty() || value.starts_with('-') || value.ends_with('-') {
        return false;
    }
    value
        .bytes()
        .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn skill_id_preserves_the_existing_kebab_case_contract() {
        for value in ["code-review", "skill2", "1", "a--b"] {
            assert_eq!(SkillId::parse(value).expect("valid").as_str(), value);
        }
        for value in [
            "",
            "-code",
            "code-",
            "Code-review",
            "code_review",
            "code review",
        ] {
            assert_eq!(SkillId::parse(value), Err(SkillDomainError::InvalidId));
        }
    }

    #[test]
    fn location_normalizes_global_and_requires_a_workspace_boundary() {
        let global =
            SkillLocation::new(SkillScope::Global, Some("ignored")).expect("global location");
        assert_eq!(global.workspace_path, None);
        assert_eq!(global.storage_workspace_key(), "");

        let workspace = SkillLocation::new(SkillScope::Workspace, Some("  D:/code/app  "))
            .expect("workspace location");
        assert_eq!(workspace.workspace_path.as_deref(), Some("D:/code/app"));
        assert_eq!(workspace.storage_workspace_key(), "D:/code/app");
        assert_eq!(
            SkillLocation::new(SkillScope::Workspace, Some("  ")),
            Err(SkillDomainError::WorkspacePathRequired)
        );
    }

    #[test]
    fn the_same_id_in_different_scopes_has_a_distinct_domain_key() {
        let id = SkillId::parse("code-review").expect("id");
        let global = SkillKey::new(
            id.clone(),
            SkillLocation::new(SkillScope::Global, None).expect("global"),
        );
        let workspace = SkillKey::new(
            id,
            SkillLocation::new(SkillScope::Workspace, Some("D:/code/app")).expect("workspace"),
        );
        assert_ne!(global, workspace);
    }
}
