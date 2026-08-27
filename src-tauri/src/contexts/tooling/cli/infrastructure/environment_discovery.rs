//! Finding CLI executables, in the order a shell would find them.
//!
//! PATH is enumerated directly rather than through `which`/`where`, because the *position* of each
//! hit is what decides which installation is active and a resolver's output does not carry it.
//! Known locations are enumerated afterwards and tagged as such: they are real, useful for
//! diagnosis, and explicitly not what runs when the user types the command.
//!
//! Discovery never recursively scans a disk. Every location is either on PATH or in a bounded,
//! platform-specific list.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use crate::contexts::tooling::cli::application::environment_error::CliEnvironmentError;
use crate::contexts::tooling::cli::application::environment_ports::{
    CliCancellation, CliDiscoveryPort, CliProbeBudget,
};
use crate::contexts::tooling::cli::domain::ids::{CliInstallationId, CliSourceId, CliToolId};
use crate::contexts::tooling::cli::domain::installation::{CliEnvironmentOrigin, CliInstallation};
use crate::contexts::tooling::cli::domain::source::{CliSourceConfidence, CliSourceKind};
use crate::contexts::tooling::cli::domain::status::CliExecutableStatus;
// Only the Unix branch orders version-managed directories; Windows has no nvm layout to sort.
#[cfg(not(target_os = "windows"))]
use crate::contexts::tooling::cli::domain::version::NormalizedCliVersion;

pub(crate) struct SystemCliDiscovery;

impl CliDiscoveryPort for SystemCliDiscovery {
    fn discover(
        &self,
        _agent_id: &CliToolId,
        executable_names: &[&str],
        budget: CliProbeBudget,
        cancellation: &CliCancellation,
    ) -> Result<Vec<CliInstallation>, CliEnvironmentError> {
        let mut installations = Vec::new();
        for (path, priority) in path_candidates(executable_names, budget.max_candidates) {
            if cancellation.is_cancelled() {
                return Ok(installations);
            }
            installations.push(build_installation(
                &path,
                CliEnvironmentOrigin::Path,
                Some(priority),
            ));
        }
        let remaining = budget.max_candidates.saturating_sub(installations.len());
        for path in known_location_candidates(executable_names, remaining) {
            if cancellation.is_cancelled() {
                break;
            }
            installations.push(build_installation(
                &path,
                CliEnvironmentOrigin::KnownLocation,
                None,
            ));
        }
        Ok(installations)
    }

    fn environment_fingerprint(&self) -> Result<String, CliEnvironmentError> {
        Ok(environment_fingerprint())
    }
}

/// Filenames a bare command can resolve to on this platform.
///
/// Windows needs the extension list because `claude`, `claude.cmd`, and `claude.exe` are three
/// files a shell would try in order; Unix has exactly one name.
fn executable_filenames(name: &str) -> Vec<String> {
    if cfg!(target_os = "windows") {
        vec![
            format!("{name}.cmd"),
            format!("{name}.exe"),
            format!("{name}.bat"),
            format!("{name}.ps1"),
            name.to_string(),
        ]
    } else {
        vec![name.to_string()]
    }
}

/// PATH hits paired with the index of the directory they came from.
///
/// The index is the whole point: it is what makes "first runnable PATH entry" answerable, and it
/// is exactly what a resolver's flat output loses.
fn path_candidates(executable_names: &[&str], limit: usize) -> Vec<(PathBuf, u32)> {
    let Some(raw) = std::env::var_os("PATH") else {
        return Vec::new();
    };
    let mut found = Vec::new();
    for (index, directory) in std::env::split_paths(&raw).enumerate() {
        if found.len() >= limit {
            break;
        }
        for name in executable_names {
            // Every launcher in the directory, not just the first. One npm global install on
            // Windows writes `tool`, `tool.cmd`, and `tool.ps1` side by side, and stopping at the
            // first meant the other two were never seen -- so `group_launcher_families` had
            // nothing to fold and the details drawer's alias list was always empty. Folding is
            // what keeps one install from reporting as three; skipping the siblings only hid them.
            for filename in executable_filenames(name) {
                let candidate = directory.join(&filename);
                if candidate.is_file() {
                    found.push((candidate, u32::try_from(index).unwrap_or(u32::MAX)));
                }
            }
        }
    }
    found
}

/// Bounded, platform-specific locations a CLI can live in without being on PATH.
///
/// A desktop app launched from an icon inherits no shell profile, so a CLI installed exactly as its
/// guide documents can be entirely absent from PATH. These make it visible for diagnosis.
fn known_location_candidates(executable_names: &[&str], limit: usize) -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    for name in executable_names {
        candidates.extend(known_roots(name));
    }
    candidates
        .into_iter()
        .filter(|path| path.is_file())
        .take(limit)
        .collect()
}

#[cfg(target_os = "windows")]
fn known_roots(name: &str) -> Vec<PathBuf> {
    let mut roots = Vec::new();
    if let Some(app_data) = std::env::var_os("APPDATA") {
        let base = PathBuf::from(app_data).join("npm");
        roots.push(base.join(format!("{name}.cmd")));
        roots.push(base.join(format!("{name}.exe")));
    }
    if let Some(profile) = std::env::var_os("USERPROFILE") {
        roots.push(
            PathBuf::from(profile)
                .join("scoop")
                .join("shims")
                .join(format!("{name}.exe")),
        );
    }
    if let Some(local) = std::env::var_os("LOCALAPPDATA") {
        let base = PathBuf::from(local);
        roots.push(
            base.join("Programs")
                .join("OpenAI")
                .join("Codex")
                .join("bin")
                .join(format!("{name}.exe")),
        );
        roots.push(
            base.join("Microsoft")
                .join("WinGet")
                .join("Links")
                .join(format!("{name}.exe")),
        );
        roots.push(base.join(name).join("bin").join(format!("{name}.exe")));
    }
    roots
}

#[cfg(not(target_os = "windows"))]
fn known_roots(name: &str) -> Vec<PathBuf> {
    let mut roots = Vec::new();
    if let Some(home) = std::env::var_os("HOME") {
        let home = PathBuf::from(home);
        for relative in [
            ".local/bin",
            ".npm-global/bin",
            ".volta/bin",
            ".bun/bin",
            ".opencode/bin",
            ".asdf/shims",
        ] {
            roots.push(home.join(relative).join(name));
        }
        roots.extend(version_managed_bin_paths(&home, name));
    }
    for base in ["/usr/local/bin", "/usr/bin", "/opt/homebrew/bin"] {
        roots.push(PathBuf::from(base).join(name));
    }
    roots
}

/// Every nvm-managed node's bin directory, newest version first.
///
/// Ordered with the normalized version parser rather than by directory name. Lexical order puts
/// `v10.0.0` before `v9.0.0`, so the "newest" nvm install offered first was frequently the oldest
/// one -- and on a machine with both, that is the version reported for a tool the user believes is
/// current.
#[cfg(not(target_os = "windows"))]
fn version_managed_bin_paths(home: &Path, executable_name: &str) -> Vec<PathBuf> {
    let versions_root = home.join(".nvm").join("versions").join("node");
    let Ok(entries) = std::fs::read_dir(&versions_root) else {
        return Vec::new();
    };
    let mut versions: Vec<(NormalizedCliVersion, PathBuf)> = entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.is_dir())
        .map(|path| {
            let parsed = path
                .file_name()
                .and_then(|name| name.to_str())
                .map(NormalizedCliVersion::parse)
                .unwrap_or_else(|| NormalizedCliVersion::parse(""));
            (parsed, path)
        })
        .collect();
    // Newest first.
    versions.sort_by(|left, right| right.0.display_order(&left.0));
    versions
        .into_iter()
        .map(|(_, path)| path.join("bin").join(executable_name))
        .collect()
}

pub(super) fn build_installation(
    path: &Path,
    origin: CliEnvironmentOrigin,
    priority: Option<u32>,
) -> CliInstallation {
    let display = path.to_string_lossy().to_string();
    // Canonicalization failure is a diagnostic, not a reason to drop a real installation: a
    // permission-denied symlink still names a binary the user has on their machine.
    let resolved = std::fs::canonicalize(path);
    // A launcher that exists while its target does not is a dangling shim, not an installation.
    // `NotFound` from canonicalize on a path we just saw means the link resolved to nothing.
    let target_missing = matches!(
        resolved.as_ref().map_err(std::io::Error::kind),
        Err(std::io::ErrorKind::NotFound)
    );
    let canonical = resolved
        .ok()
        .map(|resolved| resolved.to_string_lossy().to_string());
    let (kind, confidence) = classify_source(&display);
    CliInstallation {
        id: installation_id(&display),
        executable_path: display,
        canonical_path: canonical,
        // Populated by `group_launcher_families` once the whole candidate set is known.
        alias_paths: Vec::new(),
        target_missing,
        reported_version: None,
        source_id: source_id_for(kind),
        source_kind: kind,
        source_confidence: confidence,
        path_priority: priority,
        environment_origin: origin,
        // Filled in by the probe pass. Not a fault -- simply not yet checked.
        executable_status: CliExecutableStatus::Unknown,
    }
}

/// Stable per path, so re-running discovery does not renumber installations and a details drawer
/// keeps pointing at the same row.
fn installation_id(path: &str) -> CliInstallationId {
    let sanitized: String = path
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '-' || ch == '.' {
                ch
            } else {
                '-'
            }
        })
        .collect();
    // Bounded so a deep path cannot produce an oversized identifier the value object would reject.
    let trimmed = if sanitized.chars().count() > 100 {
        sanitized
            .chars()
            .skip(sanitized.chars().count() - 100)
            .collect()
    } else {
        sanitized
    };
    CliInstallationId::new(format!("i-{trimmed}"))
        .unwrap_or_else(|_| CliInstallationId::trusted("i-unknown"))
}

/// Classifies a path into a source and how much that classification is worth.
///
/// Every answer here is `Inferred` at best. A path shape is evidence, not proof of ownership, and
/// treating it as proof is what let "this looks npm-ish" authorize an npm mutation.
pub(super) fn classify_source(path: &str) -> (CliSourceKind, CliSourceConfidence) {
    let value = path.replace('\\', "/").to_ascii_lowercase();
    let kind = if value.contains("/microsoft/winget/packages/")
        || value.contains("/microsoft/winget/links/")
    {
        CliSourceKind::Winget
    } else if value.contains("/appdata/roaming/npm/")
        || value.contains("/.npm/")
        || value.contains("/.npm-global/")
        || value.contains("/node_modules/")
        || value.contains("/.nvm/versions/node/")
    {
        CliSourceKind::Npm
    } else if value.contains("/homebrew/") || value.contains("/cellar/") {
        CliSourceKind::Homebrew
    } else if value.contains("/.volta/") {
        CliSourceKind::Volta
    } else if value.contains("/.bun/") {
        CliSourceKind::Bun
    } else if value.contains("/programs/openai/codex/") {
        CliSourceKind::Desktop
    } else if value.contains("/.local/bin/")
        || value.contains("/.claude/")
        || value.contains("/.opencode/")
    {
        CliSourceKind::VendorInstaller
    } else if value.starts_with("/usr/bin/") || value.starts_with("/usr/local/bin/") {
        CliSourceKind::System
    } else {
        CliSourceKind::Unknown
    };
    let confidence = match kind {
        CliSourceKind::Unknown => CliSourceConfidence::Unknown,
        _ => CliSourceConfidence::Inferred,
    };
    (kind, confidence)
}

fn source_id_for(kind: CliSourceKind) -> Option<CliSourceId> {
    // Only the three managed sources map onto a distribution the registry declares; a detect-only
    // kind has no source id to plan against, which is what keeps it detect-only.
    let id = match kind {
        CliSourceKind::Npm => "npm",
        CliSourceKind::Winget => "winget",
        CliSourceKind::VendorInstaller => "vendor",
        _ => return None,
    };
    CliSourceId::new(id).ok()
}

/// A non-secret fingerprint of the resolution environment.
///
/// Inputs are the things that change which binary runs: the OS, the architecture, and the PATH
/// entry order. HOME contributes only through a non-reversible hash of its own text, so two
/// machines with different users do not collide while the value itself never appears.
///
/// Deliberately excluded: credentials, command output, provider configuration, and any environment
/// variable that does not participate in resolution.
fn environment_fingerprint() -> String {
    let mut parts = vec![
        std::env::consts::OS.to_string(),
        std::env::consts::ARCH.to_string(),
        "local-desktop".to_string(),
    ];
    if let Some(path) = std::env::var_os("PATH") {
        let entries: Vec<String> = std::env::split_paths(&path)
            .map(|entry| entry.to_string_lossy().to_string())
            .collect();
        parts.push(format!("path:{}", stable_hash(&entries.join("|"))));
        parts.push(format!("pathlen:{}", entries.len()));
    }
    let home = std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(|value| value.to_string_lossy().to_string())
        .unwrap_or_default();
    parts.push(format!("home:{}", stable_hash(&home)));
    parts.join("/")
}

/// FNV-1a. Not cryptographic and does not need to be: this only has to change when its input
/// changes and never expose the input.
fn stable_hash(value: &str) -> String {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in value.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!("{hash:016x}")
}

/// Distinct source kinds present, for a details drawer that groups by source.
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "consumed by the details drawer in task group 11; remove with that group"
    )
)]
pub(crate) fn distinct_source_kinds(installations: &[CliInstallation]) -> Vec<CliSourceKind> {
    installations
        .iter()
        .map(|installation| installation.source_kind)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

#[cfg(test)]
#[path = "environment_discovery_tests.rs"]
mod tests;
