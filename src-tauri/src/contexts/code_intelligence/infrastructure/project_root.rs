use crate::contexts::code_intelligence::domain::models::{
    ConfigurationFingerprint, LanguageFamily, ServerKind,
};
use std::path::{Path, PathBuf};
use thiserror::Error;

const MAX_ANCESTORS: usize = 128;
const RUST_MARKERS: &[&str] = &["Cargo.toml"];
const TYPESCRIPT_MARKERS: &[&str] = &["tsconfig.json", "jsconfig.json", "package.json"];

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
}

pub(crate) struct ProjectRootResolver;

impl ProjectRootResolver {
    pub(crate) fn resolve(
        session_root: &Path,
        document: &Path,
        language: LanguageFamily,
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

        let markers = markers_for(language);
        for _ in 0..MAX_ANCESTORS {
            if markers.iter().any(|marker| current.join(marker).is_file()) {
                return Ok(current);
            }
            if current == session_root {
                return Ok(session_root);
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
    server_kind: ServerKind,
    configuration_fingerprint: ConfigurationFingerprint,
}

impl ProcessKey {
    pub(crate) fn new(
        session_root: &Path,
        project_root: &Path,
        server_kind: ServerKind,
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
            server_kind,
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

    pub(crate) const fn server_kind(&self) -> ServerKind {
        self.server_kind
    }

    pub(crate) fn same_instance_scope(&self, other: &Self) -> bool {
        self.session_root == other.session_root
            && self.project_root == other.project_root
            && self.server_kind == other.server_kind
    }
}

fn canonical_directory(path: &Path) -> Option<PathBuf> {
    path.canonicalize().ok().filter(|path| path.is_dir())
}

const fn markers_for(language: LanguageFamily) -> &'static [&'static str] {
    match language {
        LanguageFamily::Rust => RUST_MARKERS,
        LanguageFamily::TypeScriptJavaScript => TYPESCRIPT_MARKERS,
    }
}
