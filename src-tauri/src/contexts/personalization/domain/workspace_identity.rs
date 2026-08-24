use sha2::{Digest, Sha256};

use super::error::PersonalizationDomainError;
use super::scope::{WorkspaceIdentity, WorkspaceKey, WorkspaceKind};

/// Prefix on every derived key, so a derived key is never mistaken for a stable id the rest of the
/// application already assigned.
const DERIVED_KEY_PREFIX: &str = "ws";

/// Hex characters kept from the digest. 32 hex characters is 128 bits — far past any collision
/// concern for a per-machine workspace set, and short enough to stay readable in a diagnostic.
const DERIVED_KEY_HEX_LENGTH: usize = 32;

/// Where a workspace identity came from.
///
/// Ordered by preference: an id the application already assigns is always better than one derived
/// here, because deriving one means two subsystems can disagree about what "the same workspace"
/// means.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum WorkspaceIdentitySource {
    /// A stable id the workspace subsystem already owns. Used verbatim.
    StableId(String),
    /// A local filesystem root, normalized before hashing.
    LocalRoot { path: String },
    /// A remote workspace: the path alone is not an identity, because two hosts routinely expose
    /// the same path.
    Remote {
        host: String,
        port: u16,
        user: Option<String>,
        path: String,
    },
}

/// Normalizes a local filesystem root into a form two spellings of the same directory agree on.
///
/// Pure and filesystem-free on purpose: canonicalizing would make the key depend on whether the
/// directory currently exists and on symlink resolution at that instant, so a workspace would
/// change identity when a link was repointed or a drive was offline.
///
/// `case_insensitive` is passed in rather than read from `cfg!` so the rule is testable on every
/// platform. The caller sets it for filesystems that fold case; the key never leaves this machine,
/// so a platform-dependent rule here is correct rather than a portability problem.
pub(crate) fn normalize_local_root(path: &str, case_insensitive: bool) -> String {
    // Windows extended-length paths address the same directory as their plain form.
    let trimmed = path
        .trim()
        .strip_prefix(r"\\?\UNC\")
        .map(|rest| format!(r"\\{rest}"))
        .unwrap_or_else(|| {
            path.trim()
                .strip_prefix(r"\\?\")
                .unwrap_or(path.trim())
                .to_string()
        });

    let mut normalized = trimmed.replace('\\', "/");
    // Collapse repeated separators, but keep a leading `//` so a UNC root stays distinguishable
    // from an absolute local path.
    let leading_unc = normalized.starts_with("//");
    while normalized.contains("//") {
        normalized = normalized.replace("//", "/");
    }
    if leading_unc {
        normalized.insert(0, '/');
    }
    // A trailing separator is presentation, not identity — except on a bare root.
    while normalized.len() > 1 && normalized.ends_with('/') {
        normalized.pop();
    }
    if case_insensitive {
        normalized = normalized.to_lowercase();
    }
    normalized
}

/// Normalizes a remote path. Always case-sensitive: the remote filesystem's folding rules are not
/// knowable from here, and folding a case-sensitive remote path would merge distinct directories.
fn normalize_remote_path(path: &str) -> String {
    normalize_local_root(path, false)
}

fn derive_key(parts: &[&str]) -> Result<WorkspaceKey, PersonalizationDomainError> {
    let mut hasher = Sha256::new();
    hasher.update(b"personalization-workspace-v1");
    for part in parts {
        // Length-prefixed so two different splits cannot hash identically — `a` + `bc` must not
        // collide with `ab` + `c`.
        hasher.update(part.len().to_string().as_bytes());
        hasher.update(b":");
        hasher.update(part.as_bytes());
        hasher.update(b"\x1f");
    }
    let digest = hasher.finalize();
    let hex: String = digest
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>()
        .chars()
        .take(DERIVED_KEY_HEX_LENGTH)
        .collect();
    WorkspaceKey::parse(&format!("{DERIVED_KEY_PREFIX}_{hex}"))
}

impl WorkspaceIdentitySource {
    /// Produces the stable local key this source identifies.
    ///
    /// Nothing secret reaches the digest. A remote workspace contributes host, port, user, and
    /// path — its connection identity — and never a password, token, private key, or anything else
    /// a credential store holds. Those live with the SSH connection profile and are deliberately
    /// not inputs here: an identity derived from a secret would change when the secret rotated,
    /// and would put recoverable material into a value that appears in diagnostics.
    pub(crate) fn derive_key(
        &self,
        case_insensitive_local: bool,
    ) -> Result<WorkspaceKey, PersonalizationDomainError> {
        match self {
            Self::StableId(id) => WorkspaceKey::parse(id.trim()),
            Self::LocalRoot { path } => {
                derive_key(&["local", &normalize_local_root(path, case_insensitive_local)])
            }
            Self::Remote {
                host,
                port,
                user,
                path,
            } => derive_key(&[
                "remote",
                &host.trim().to_lowercase(),
                &port.to_string(),
                user.as_deref().unwrap_or("").trim(),
                &normalize_remote_path(path),
            ]),
        }
    }

    /// The path a user reads. Never an identity input: renaming a display label must not move a
    /// workspace's memories, and two workspaces may legitimately share a label.
    pub(crate) fn display_path(&self) -> String {
        match self {
            Self::StableId(id) => id.clone(),
            Self::LocalRoot { path } => path.trim().to_string(),
            Self::Remote {
                host, user, path, ..
            } => match user {
                Some(user) => format!("{user}@{host}:{path}"),
                None => format!("{host}:{path}"),
            },
        }
    }

    fn kind(&self) -> WorkspaceKind {
        match self {
            Self::Remote { .. } => WorkspaceKind::Remote,
            _ => WorkspaceKind::Local,
        }
    }

    /// Builds the full identity: the key authorization compares, plus the path a user reads.
    pub(crate) fn resolve(
        &self,
        case_insensitive_local: bool,
    ) -> Result<WorkspaceIdentity, PersonalizationDomainError> {
        Ok(WorkspaceIdentity::new(
            self.derive_key(case_insensitive_local)?,
            self.display_path(),
            self.kind(),
        ))
    }
}

/// Whether this platform's filesystem folds case for local paths.
///
/// Windows and macOS default to case-insensitive; Linux does not. The workspace key never leaves
/// the machine that derived it, so following the local filesystem's rule is what makes two
/// spellings of one directory agree.
pub(crate) fn local_paths_fold_case() -> bool {
    cfg!(any(target_os = "windows", target_os = "macos"))
}
