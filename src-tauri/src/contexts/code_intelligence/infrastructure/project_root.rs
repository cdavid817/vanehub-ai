use crate::contexts::code_intelligence::domain::models::{ConfigurationFingerprint, Language};
use std::path::{Path, PathBuf};
use thiserror::Error;

const MAX_ANCESTORS: usize = 128;

#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ProjectRootError {
    #[error("session root is unavailable")]
    SessionRootUnavailable,
    #[error("document is unavailable")]
    DocumentUnavailable,
    #[error("document resolves outside the session root")]
    OutsideSessionRoot,
    #[error("project root traversal limit exceeded")]
    TraversalLimit,
    #[error("project root is unavailable")]
    ProjectRootUnavailable,
    /// Kept distinct from `ProjectRootUnavailable` on purpose: "no compilation database" tells a
    /// user what to do and "project root unavailable" does not.
    #[error("the language requires a project marker and none was found in the workspace")]
    RequiredMarkerMissing,
}

pub(crate) struct ProjectRootResolver;

impl ProjectRootResolver {
    pub(crate) fn resolve(
        session_root: &Path,
        document: &Path,
        language: Language,
    ) -> Result<PathBuf, ProjectRootError> {
        let session_root =
            canonical_directory(session_root).ok_or(ProjectRootError::SessionRootUnavailable)?;
        let document = document
            .canonicalize()
            .map_err(|_| ProjectRootError::DocumentUnavailable)?;
        if !document.is_file() {
            return Err(ProjectRootError::DocumentUnavailable);
        }
        let mut current = document
            .parent()
            .ok_or(ProjectRootError::OutsideSessionRoot)?
            .to_path_buf();
        if !current.starts_with(&session_root) {
            return Err(ProjectRootError::OutsideSessionRoot);
        }

        let markers = language.root_markers;
        for _ in 0..MAX_ANCESTORS {
            // A marker may name a path inside the candidate directory rather than a file directly
            // in it, which is how C/C++ finds `build/compile_commands.json` without a second
            // detection mechanism.
            if markers.iter().any(|marker| current.join(marker).is_file()) {
                return Ok(current);
            }
            if current == session_root {
                // Falling back to the workspace root is right for a server that degrades
                // gracefully without a manifest. A server that instead answers confidently wrong
                // gets a refusal, because an unavailable result is the better answer.
                return if language.requires_root_marker {
                    Err(ProjectRootError::RequiredMarkerMissing)
                } else {
                    Ok(session_root)
                };
            }
            let Some(parent) = current.parent() else {
                return Err(ProjectRootError::OutsideSessionRoot);
            };
            if !parent.starts_with(&session_root) {
                return Err(ProjectRootError::OutsideSessionRoot);
            }
            current = parent.to_path_buf();
        }
        Err(ProjectRootError::TraversalLimit)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct ProcessKey {
    session_root: PathBuf,
    project_root: PathBuf,
    language: Language,
    configuration_fingerprint: ConfigurationFingerprint,
}

impl ProcessKey {
    pub(crate) fn new(
        session_root: &Path,
        project_root: &Path,
        language: Language,
        configuration_fingerprint: ConfigurationFingerprint,
    ) -> Result<Self, ProjectRootError> {
        let session_root =
            canonical_directory(session_root).ok_or(ProjectRootError::SessionRootUnavailable)?;
        let project_root =
            canonical_directory(project_root).ok_or(ProjectRootError::ProjectRootUnavailable)?;
        if !project_root.starts_with(&session_root) {
            return Err(ProjectRootError::OutsideSessionRoot);
        }
        Ok(Self {
            session_root,
            project_root,
            language,
            configuration_fingerprint,
        })
    }

    pub(crate) fn session_root(&self) -> PathBuf {
        self.session_root.clone()
    }

    pub(crate) fn session_root_ref(&self) -> &Path {
        &self.session_root
    }

    pub(crate) fn project_root(&self) -> PathBuf {
        self.project_root.clone()
    }

    pub(crate) fn project_root_ref(&self) -> &Path {
        &self.project_root
    }

    pub(crate) const fn language(&self) -> Language {
        self.language
    }

    pub(crate) fn same_instance_scope(&self, other: &Self) -> bool {
        self.session_root == other.session_root
            && self.project_root == other.project_root
            && self.language == other.language
    }
}

fn canonical_directory(path: &Path) -> Option<PathBuf> {
    path.canonicalize().ok().filter(|path| path.is_dir())
}
