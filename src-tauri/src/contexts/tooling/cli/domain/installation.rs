//! One discovered executable, which one the host runs, which one VaneHub recommends, and every way
//! those two can disagree.
//!
//! The split is the point. A single `active` identity cannot answer both "what does the user's
//! terminal run" and "which installation should VaneHub act on". When a broken launcher sits first
//! on PATH and a healthy copy sits behind it, collapsing the two hides exactly the problem the user
//! needs to see -- and calling the healthy one "active" is a false statement about their machine.
//!
//! `path_selected` is decided by PATH order alone and never by health. `recommended` is decided by
//! probe results. When they differ, that difference *is* the conflict.

use super::ids::{CliInstallationId, CliSourceId};
use super::source::{CliSourceConfidence, CliSourceKind};
use super::status::CliExecutableStatus;
use super::version::NormalizedCliVersion;

/// Where discovery found a candidate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CliEnvironmentOrigin {
    /// Resolved by walking PATH. This is what a shell would actually execute.
    Path,
    /// Found in a bounded, platform-specific location that is not on this process's PATH. Real,
    /// useful for diagnosis, and not what runs when the user types the command.
    KnownLocation,
}

impl CliEnvironmentOrigin {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Path => "path",
            Self::KnownLocation => "known-location",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CliInstallation {
    pub(crate) id: CliInstallationId,
    pub(crate) executable_path: String,
    /// Resolved target when symlinks could be followed. `None` when canonicalization failed, which
    /// is a diagnostic, not a reason to drop the candidate.
    pub(crate) canonical_path: Option<String>,
    /// Other launcher files that are the same logical installation -- on Windows, one npm global
    /// install produces `tool`, `tool.cmd`, and `tool.ps1` side by side.
    pub(crate) alias_paths: Vec<String>,
    /// The launcher exists but resolves to a target that does not. A dangling shim, not a working
    /// installation.
    pub(crate) target_missing: bool,
    pub(crate) reported_version: Option<NormalizedCliVersion>,
    pub(crate) source_id: Option<CliSourceId>,
    pub(crate) source_kind: CliSourceKind,
    pub(crate) source_confidence: CliSourceConfidence,
    /// Position in PATH, lowest first. `None` for a known-location candidate.
    pub(crate) path_priority: Option<u32>,
    pub(crate) environment_origin: CliEnvironmentOrigin,
    pub(crate) executable_status: CliExecutableStatus,
}

impl CliInstallation {
    pub(crate) fn is_runnable(&self) -> bool {
        self.executable_status.is_runnable()
    }

    pub(crate) fn is_on_path(&self) -> bool {
        self.environment_origin == CliEnvironmentOrigin::Path
    }

    /// The identity two candidates are deduplicated by: the canonical target when available, else
    /// the literal path. Two PATH entries that symlink to one binary are one installation.
    pub(crate) fn dedup_key(&self) -> &str {
        self.canonical_path
            .as_deref()
            .unwrap_or(&self.executable_path)
    }

    /// The identity that groups launcher aliases into one logical installation.
    ///
    /// On Windows `tool`, `tool.cmd`, and `tool.ps1` in one directory are three files with three
    /// canonical paths but one npm install behind them, so the key drops the extension and folds
    /// case. On Unix a canonical target already collapses symlinks and there is no extension
    /// convention to fold, so stem-grouping there would merge genuinely different programs.
    pub(crate) fn launcher_family_key(&self) -> String {
        let path = self.dedup_key().replace('\\', "/");
        if !cfg!(target_os = "windows") {
            return path;
        }
        let (directory, file) = match path.rfind('/') {
            Some(index) => (&path[..index], &path[index + 1..]),
            None => ("", path.as_str()),
        };
        let stem = match file.rfind('.') {
            Some(index) => &file[..index],
            None => file,
        };
        format!("{directory}/{stem}").to_ascii_lowercase()
    }
}

/// Which installation the host runs, and which one VaneHub recommends.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct ActiveSelection {
    /// First PATH-resolved launcher, however healthy. `None` when nothing is on PATH -- which is a
    /// real answer, not a missing one.
    pub(crate) path_selected: Option<usize>,
    /// Best probed installation. Equal to `path_selected` in the ordinary case.
    pub(crate) recommended: Option<usize>,
}

impl ActiveSelection {
    /// Whether the host would run something other than what VaneHub recommends.
    pub(crate) fn diverges(&self) -> bool {
        match (self.path_selected, self.recommended) {
            (Some(path), Some(recommended)) => path != recommended,
            // Nothing on PATH while a recommendation exists is also a divergence: the recommended
            // installation is not what the command would run.
            (None, Some(_)) => true,
            _ => false,
        }
    }
}

/// Selects both identities.
///
/// PATH order alone decides `path_selected`; probe results decide `recommended`. Neither borrows
/// the other's rule.
pub(crate) fn select_active(installations: &[CliInstallation]) -> ActiveSelection {
    let lowest_path_entry = installations
        .iter()
        .enumerate()
        .filter(|(_, installation)| installation.is_on_path())
        .min_by_key(|(_, installation)| installation.path_priority.unwrap_or(u32::MAX))
        .map(|(index, _)| index);

    let first_runnable_on_path = installations
        .iter()
        .enumerate()
        .filter(|(_, installation)| installation.is_on_path() && installation.is_runnable())
        .min_by_key(|(_, installation)| installation.path_priority.unwrap_or(u32::MAX))
        .map(|(index, _)| index);

    let recommended = first_runnable_on_path
        // Nothing runnable on PATH: a runnable known-location copy is the best VaneHub can point
        // at, even though it is not what the command runs.
        .or_else(|| installations.iter().position(CliInstallation::is_runnable))
        // Nothing runs at all: recommend what PATH would pick so the diagnosis has a subject.
        .or(lowest_path_entry)
        .or(if installations.is_empty() {
            None
        } else {
            Some(0)
        });

    ActiveSelection {
        path_selected: lowest_path_entry,
        recommended,
    }
}

/// Removes candidates that resolve to the same binary, keeping the first occurrence so PATH order
/// is preserved.
pub(crate) fn deduplicate(installations: Vec<CliInstallation>) -> Vec<CliInstallation> {
    let mut seen = Vec::new();
    let mut unique = Vec::new();
    for installation in installations {
        let key = installation.dedup_key().to_string();
        if seen.contains(&key) {
            continue;
        }
        seen.push(key);
        unique.push(installation);
    }
    unique
}

/// Collapses launcher aliases into one logical installation per family.
///
/// The surviving member is the first in PATH order; the rest become `alias_paths`. Without this a
/// single npm global install on Windows reports as three competing installations and every such
/// machine looks conflicted.
pub(crate) fn group_launcher_families(installations: Vec<CliInstallation>) -> Vec<CliInstallation> {
    let mut families: Vec<(String, CliInstallation)> = Vec::new();
    for installation in installations {
        let key = installation.launcher_family_key();
        match families.iter_mut().find(|(existing, _)| existing == &key) {
            Some((_, kept)) => {
                // A runnable sibling upgrades an unprobed preferred launcher -- they are the same
                // program, so probing either one answers for the family.
                let sibling_is_runnable = installation.is_runnable();
                let promote = !kept.is_runnable() && sibling_is_runnable;
                // The alias list records what was folded away, so the details drawer can still show
                // every file the user has on disk.
                kept.alias_paths.push(installation.executable_path);
                kept.alias_paths.extend(installation.alias_paths);
                if promote {
                    kept.executable_status = installation.executable_status;
                    if kept.reported_version.is_none() {
                        kept.reported_version = installation.reported_version;
                    }
                }
            }
            None => families.push((key, installation)),
        }
    }
    families.into_iter().map(|(_, kept)| kept).collect()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum CliConflictSeverity {
    /// Worth showing, changes nothing about what runs.
    Info,
    /// Changes which executable runs, or which version.
    Warning,
    /// Makes a mutation target ambiguous or unsafe.
    Error,
}

impl CliConflictSeverity {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Info => "info",
            Self::Warning => "warning",
            Self::Error => "error",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CliConflictKind {
    /// Several launcher aliases for one logical installation. Expected on Windows.
    DuplicateLauncherAlias,
    /// An earlier PATH entry hides a later installation.
    PathShadowing,
    /// The PATH-selected launcher is broken while a healthy installation exists behind it.
    BrokenPathPrecedence,
    /// Distinct installations owned by different sources.
    MultipleInstallationSources,
    /// Installations report different versions.
    VersionDivergence,
    /// No source can be established for the installation a mutation would target.
    AmbiguousSourceOwnership,
    /// Reachable from a login shell but absent from this process's PATH.
    EnvironmentPathDivergence,
    /// The executable's architecture does not match the host.
    ArchitectureMismatch,
    /// A launcher resolves to a target that no longer exists.
    StaleLauncherTarget,
}

impl CliConflictKind {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::DuplicateLauncherAlias => "duplicate-launcher-alias",
            Self::PathShadowing => "path-shadowing",
            Self::BrokenPathPrecedence => "broken-path-precedence",
            Self::MultipleInstallationSources => "multiple-installation-sources",
            Self::VersionDivergence => "version-divergence",
            Self::AmbiguousSourceOwnership => "ambiguous-source-ownership",
            Self::EnvironmentPathDivergence => "environment-path-divergence",
            Self::ArchitectureMismatch => "architecture-mismatch",
            Self::StaleLauncherTarget => "stale-launcher-target",
        }
    }

    fn severity(self) -> CliConflictSeverity {
        match self {
            Self::DuplicateLauncherAlias => CliConflictSeverity::Info,
            Self::PathShadowing | Self::VersionDivergence | Self::EnvironmentPathDivergence => {
                CliConflictSeverity::Warning
            }
            Self::BrokenPathPrecedence
            | Self::MultipleInstallationSources
            | Self::AmbiguousSourceOwnership
            | Self::ArchitectureMismatch
            | Self::StaleLauncherTarget => CliConflictSeverity::Error,
        }
    }

    /// Whether the target of a machine change is ambiguous or unsafe while this holds.
    fn blocks_mutation(self) -> bool {
        matches!(
            self,
            Self::BrokenPathPrecedence
                | Self::MultipleInstallationSources
                | Self::AmbiguousSourceOwnership
                | Self::ArchitectureMismatch
                | Self::StaleLauncherTarget
        )
    }

    /// Whether launching the tool would run something other than the recommended installation.
    fn blocks_launch(self) -> bool {
        matches!(
            self,
            Self::PathShadowing
                | Self::BrokenPathPrecedence
                | Self::VersionDivergence
                | Self::EnvironmentPathDivergence
                | Self::ArchitectureMismatch
                | Self::StaleLauncherTarget
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CliConflict {
    pub(crate) kind: CliConflictKind,
    pub(crate) severity: CliConflictSeverity,
    /// The installations involved, so the details drawer can point at them.
    pub(crate) installations: Vec<CliInstallationId>,
    pub(crate) blocks_mutation: bool,
    pub(crate) blocks_launch: bool,
    /// Stable code the frontend localizes. Never free text.
    pub(crate) reason_code: &'static str,
}

impl CliConflict {
    fn of(kind: CliConflictKind, installations: Vec<CliInstallationId>) -> Self {
        Self {
            kind,
            severity: kind.severity(),
            installations,
            blocks_mutation: kind.blocks_mutation(),
            blocks_launch: kind.blocks_launch(),
            reason_code: kind.as_str(),
        }
    }
}

/// Every conflict present, most severe first.
pub(crate) fn derive_conflicts(
    installations: &[CliInstallation],
    selection: ActiveSelection,
) -> Vec<CliConflict> {
    let mut conflicts = Vec::new();
    let id_of = |index: usize| installations[index].id.clone();

    // Per-installation faults come first: they are true regardless of how many installations exist.
    let stale: Vec<CliInstallationId> = installations
        .iter()
        .filter(|installation| installation.target_missing)
        .map(|installation| installation.id.clone())
        .collect();
    if !stale.is_empty() {
        conflicts.push(CliConflict::of(CliConflictKind::StaleLauncherTarget, stale));
    }

    let wrong_architecture: Vec<CliInstallationId> = installations
        .iter()
        .filter(|installation| {
            installation.executable_status == CliExecutableStatus::UnsupportedArchitecture
        })
        .map(|installation| installation.id.clone())
        .collect();
    if !wrong_architecture.is_empty() {
        conflicts.push(CliConflict::of(
            CliConflictKind::ArchitectureMismatch,
            wrong_architecture,
        ));
    }

    let aliased: Vec<CliInstallationId> = installations
        .iter()
        .filter(|installation| !installation.alias_paths.is_empty())
        .map(|installation| installation.id.clone())
        .collect();
    if !aliased.is_empty() {
        conflicts.push(CliConflict::of(
            CliConflictKind::DuplicateLauncherAlias,
            aliased,
        ));
    }

    // A runnable installation that is not on PATH is reachable from a login shell but not from
    // this process. Saying it is missing would be wrong; so would saying it is what runs.
    let off_path: Vec<CliInstallationId> = installations
        .iter()
        .filter(|installation| !installation.is_on_path() && installation.is_runnable())
        .map(|installation| installation.id.clone())
        .collect();
    if !off_path.is_empty() && selection.path_selected.is_some() {
        conflicts.push(CliConflict::of(
            CliConflictKind::EnvironmentPathDivergence,
            off_path,
        ));
    }

    if installations.len() > 1 {
        conflicts.extend(multi_installation_conflicts(
            installations,
            selection,
            &id_of,
        ));
    }

    // The recommended installation carries no establishable source, so no mutation can name one.
    if let Some(recommended) = selection.recommended {
        let installation = &installations[recommended];
        if installation.source_id.is_none()
            || installation.source_confidence == CliSourceConfidence::Unknown
        {
            conflicts.push(CliConflict::of(
                CliConflictKind::AmbiguousSourceOwnership,
                vec![installation.id.clone()],
            ));
        }
    }

    conflicts.sort_by_key(|conflict| std::cmp::Reverse(conflict.severity));
    conflicts
}

fn multi_installation_conflicts(
    installations: &[CliInstallation],
    selection: ActiveSelection,
    id_of: &impl Fn(usize) -> CliInstallationId,
) -> Vec<CliConflict> {
    let mut conflicts = Vec::new();
    let all_ids: Vec<CliInstallationId> = installations
        .iter()
        .map(|installation| installation.id.clone())
        .collect();

    // The severe case: PATH runs a broken launcher while a healthy one exists behind it.
    if let (Some(path_selected), Some(recommended)) =
        (selection.path_selected, selection.recommended)
    {
        if path_selected != recommended
            && !installations[path_selected].is_runnable()
            && installations[recommended].is_runnable()
        {
            conflicts.push(CliConflict::of(
                CliConflictKind::BrokenPathPrecedence,
                vec![id_of(path_selected), id_of(recommended)],
            ));
        } else if path_selected != recommended {
            conflicts.push(CliConflict::of(
                CliConflictKind::PathShadowing,
                vec![id_of(path_selected), id_of(recommended)],
            ));
        }
    }

    let mut sources: Vec<CliSourceKind> = installations
        .iter()
        .map(|installation| installation.source_kind)
        .collect();
    sources.sort_unstable();
    sources.dedup();
    if sources.len() > 1 {
        conflicts.push(CliConflict::of(
            CliConflictKind::MultipleInstallationSources,
            all_ids.clone(),
        ));
    }

    let mut versions: Vec<&str> = installations
        .iter()
        .filter_map(|installation| installation.reported_version.as_ref())
        .map(NormalizedCliVersion::as_str)
        .collect();
    versions.sort_unstable();
    versions.dedup();
    if versions.len() > 1 {
        conflicts.push(CliConflict::of(CliConflictKind::VersionDivergence, all_ids));
    }
    conflicts
}

/// Whether any conflict makes a machine change unsafe.
pub(crate) fn conflicts_block_mutation(conflicts: &[CliConflict]) -> bool {
    conflicts.iter().any(|conflict| conflict.blocks_mutation)
}

#[cfg(test)]
#[path = "installation_tests.rs"]
mod tests;
