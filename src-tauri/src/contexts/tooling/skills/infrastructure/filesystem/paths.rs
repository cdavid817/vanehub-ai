use crate::contexts::tooling::skills::application::{
    SkillApplicationError, SkillFilesystemTransaction,
};
use crate::contexts::tooling::skills::domain::{
    SkillId, SkillLocation, SkillMountPath, SkillScope,
};
use std::path::{Path, PathBuf};

const VANEHUB_DIR: &str = ".vanehub";
const SKILLS_DIR: &str = "skills";
pub(super) const SKILL_FILE: &str = "SKILL.md";

#[derive(Clone, Default)]
pub(super) struct SkillPathResolver {
    home_root: Option<PathBuf>,
}

impl SkillPathResolver {
    #[cfg(test)]
    pub(super) fn with_home_root(home_root: PathBuf) -> Self {
        Self {
            home_root: Some(home_root),
        }
    }

    pub(super) fn scope_root(
        &self,
        location: &SkillLocation,
    ) -> Result<PathBuf, SkillApplicationError> {
        let candidate = match location.scope {
            SkillScope::Global => self.home_root.clone().unwrap_or_else(default_home_root),
            SkillScope::Workspace => {
                PathBuf::from(location.workspace_path.as_deref().ok_or_else(|| {
                    SkillApplicationError::Filesystem("Workspace path is required".to_string())
                })?)
            }
        };
        let canonical = candidate.canonicalize().map_err(filesystem_error)?;
        if !canonical.is_dir() {
            return Err(SkillApplicationError::Filesystem(format!(
                "Skill scope root is not a directory: {}",
                canonical.display()
            )));
        }
        Ok(canonical)
    }

    pub(super) fn source_root(
        &self,
        location: &SkillLocation,
    ) -> Result<PathBuf, SkillApplicationError> {
        let root = self.scope_root(location)?;
        descendant(&root, root.join(VANEHUB_DIR).join(SKILLS_DIR))
    }

    pub(super) fn source_paths(
        &self,
        location: &SkillLocation,
        id: &SkillId,
    ) -> Result<(PathBuf, PathBuf), SkillApplicationError> {
        let root = self.source_root(location)?;
        let directory = descendant(&root, root.join(id.as_str()))?;
        let document = descendant(&root, directory.join(SKILL_FILE))?;
        Ok((directory, document))
    }

    pub(super) fn mount_target(
        &self,
        location: &SkillLocation,
        id: &SkillId,
        mount_path: &SkillMountPath,
    ) -> Result<PathBuf, SkillApplicationError> {
        let root = self.scope_root(location)?;
        descendant(&root, root.join(mount_path.as_str()).join(id.as_str()))
    }

    pub(super) fn durable_backup(
        &self,
        location: &SkillLocation,
        id: &SkillId,
        agent_id: &str,
        transaction: &SkillFilesystemTransaction,
    ) -> Result<PathBuf, SkillApplicationError> {
        let root = self.scope_root(location)?;
        descendant(
            &root,
            root.join(VANEHUB_DIR)
                .join("backups")
                .join(SKILLS_DIR)
                .join(&transaction.id)
                .join(format!("{agent_id}-{}", id.as_str())),
        )
    }
}

fn default_home_root() -> PathBuf {
    std::env::var_os("USERPROFILE")
        .or_else(|| std::env::var_os("HOME"))
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
}

fn descendant(root: &Path, candidate: PathBuf) -> Result<PathBuf, SkillApplicationError> {
    if candidate != root && candidate.starts_with(root) {
        Ok(candidate)
    } else {
        Err(SkillApplicationError::Filesystem(
            "Skill path resolves outside its scope root".to_string(),
        ))
    }
}

fn filesystem_error(error: std::io::Error) -> SkillApplicationError {
    SkillApplicationError::Filesystem(error.to_string())
}
