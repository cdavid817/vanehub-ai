#![cfg_attr(not(test), allow(dead_code))]

use crate::contexts::tooling::skills::application::OverlayKey;
use crate::contexts::tooling::skills::domain::OverlayScope;
use std::fmt;
use std::path::{Path, PathBuf};

const VANEHUB_DIRECTORY: &str = ".vanehub";
const OVERLAY_DIRECTORY: &str = "skill_overlays";
const PROJECT_SKILLS_DIRECTORY: &str = "skills";
const PROJECT_OVERLAY_DIRECTORY: &str = ".overlays";

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct OverlayStorageLayout {
    pub(super) manifest_path: PathBuf,
    pub(super) payload_root: PathBuf,
    pub(super) history_root: PathBuf,
}

impl OverlayStorageLayout {
    pub(super) fn resolve(home_root: &Path, key: &OverlayKey) -> Result<Self, OverlayLayoutError> {
        let (overlay_root, manifest_root) = match key.scope {
            OverlayScope::System => {
                reject_workspace_identity(key)?;
                let root = home_overlay_root(home_root);
                (root.clone(), root)
            }
            OverlayScope::User => {
                reject_workspace_identity(key)?;
                let root = home_overlay_root(home_root);
                let manifest_root = root.join("user");
                (root, manifest_root)
            }
            OverlayScope::Project => {
                let workspace = key
                    .workspace_identity
                    .as_deref()
                    .filter(|identity| !identity.trim().is_empty())
                    .ok_or(OverlayLayoutError::MissingProjectWorkspace)?;
                let root = Path::new(workspace)
                    .join(VANEHUB_DIRECTORY)
                    .join(PROJECT_SKILLS_DIRECTORY)
                    .join(PROJECT_OVERLAY_DIRECTORY);
                (root.clone(), root)
            }
        };
        let skill_id = key.canonical_skill_id.as_str();
        Ok(Self {
            manifest_path: manifest_root.join(format!("{skill_id}.json")),
            payload_root: overlay_root.join(".payloads"),
            history_root: overlay_root.join("history").join(skill_id),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum OverlayLayoutError {
    MissingProjectWorkspace,
    UnexpectedWorkspaceIdentity,
}

impl fmt::Display for OverlayLayoutError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingProjectWorkspace => {
                formatter.write_str("Project Overlay requires a canonical workspace identity")
            }
            Self::UnexpectedWorkspaceIdentity => formatter
                .write_str("Only a Project Overlay may carry a canonical workspace identity"),
        }
    }
}

pub(super) fn is_overlay_manifest_path(manifest_root: &Path, candidate: &Path) -> bool {
    candidate.parent() == Some(manifest_root)
        && candidate
            .extension()
            .is_some_and(|extension| extension == "json")
        && candidate
            .file_stem()
            .and_then(|stem| stem.to_str())
            .is_some_and(|stem| !stem.is_empty() && !stem.starts_with('.'))
}

fn home_overlay_root(home_root: &Path) -> PathBuf {
    home_root.join(VANEHUB_DIRECTORY).join(OVERLAY_DIRECTORY)
}

fn reject_workspace_identity(key: &OverlayKey) -> Result<(), OverlayLayoutError> {
    if key.workspace_identity.is_some() {
        Err(OverlayLayoutError::UnexpectedWorkspaceIdentity)
    } else {
        Ok(())
    }
}
