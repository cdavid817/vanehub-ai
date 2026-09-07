//! The client filesystem methods (`fs/read_text_file`, `fs/write_text_file`) the host proxies for
//! an ACP agent, scoped to the session's authorized roots.
//!
//! Authorization is by canonical path, never by string prefix. A path that names something inside
//! the workspace but resolves -- through a symlink, a junction, a `..`, a device name -- to
//! something outside it is refused before any byte moves. Writes go to a sibling temporary file
//! and are renamed into place, and the parent directory is re-checked at write time so a link
//! swapped in between validation and write cannot redirect the content.

use super::budget::FS_PROXY_MAX_BYTES;
use std::fs;
use std::io::Write;
use std::path::{Component, Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum FsProxyError {
    /// Relative, device, UNC, or otherwise unacceptable path shape.
    InvalidPath(String),
    /// Resolves outside every authorized root.
    OutsideRoots,
    NotFound,
    TooLarge {
        limit: u64,
    },
    Io(String),
}

impl FsProxyError {
    pub(crate) fn message(&self) -> String {
        match self {
            Self::InvalidPath(reason) => format!("path rejected: {reason}"),
            Self::OutsideRoots => "path is outside the session's authorized roots".to_string(),
            Self::NotFound => "file not found".to_string(),
            Self::TooLarge { limit } => format!("file exceeds the {limit}-byte proxy limit"),
            Self::Io(detail) => format!("file operation failed: {detail}"),
        }
    }
}

/// The directories an agent may touch through the proxy. Roots are canonicalized once at
/// construction; a root that does not exist is dropped rather than trusted as a prefix.
#[derive(Debug, Clone)]
pub(crate) struct AuthorizedRoots {
    roots: Vec<PathBuf>,
}

impl AuthorizedRoots {
    pub(crate) fn new(candidates: impl IntoIterator<Item = PathBuf>) -> Self {
        let roots = candidates
            .into_iter()
            .filter_map(|candidate| fs::canonicalize(candidate).ok())
            .collect();
        Self { roots }
    }

    #[cfg(test)]
    pub(crate) fn is_empty(&self) -> bool {
        self.roots.is_empty()
    }

    fn contains(&self, canonical: &Path) -> bool {
        self.roots.iter().any(|root| canonical.starts_with(root))
    }

    /// Resolves `requested` to a canonical path inside the roots. For a path that does not
    /// exist yet (a new file), the nearest existing ancestor is canonicalized and the remaining
    /// normal components are appended -- with no `..` permitted among them.
    pub(crate) fn resolve(&self, requested: &str) -> Result<PathBuf, FsProxyError> {
        let path = Path::new(requested);
        validate_shape(path)?;
        if let Ok(canonical) = fs::canonicalize(path) {
            return if self.contains(&canonical) {
                Ok(canonical)
            } else {
                Err(FsProxyError::OutsideRoots)
            };
        }
        let mut existing = path.to_path_buf();
        let mut trailing: Vec<std::ffi::OsString> = Vec::new();
        loop {
            match fs::canonicalize(&existing) {
                Ok(canonical) => {
                    let mut resolved = canonical;
                    for component in trailing.iter().rev() {
                        resolved.push(component);
                    }
                    return if self.contains(&resolved) {
                        Ok(resolved)
                    } else {
                        Err(FsProxyError::OutsideRoots)
                    };
                }
                Err(_) => {
                    let Some(name) = existing.file_name() else {
                        return Err(FsProxyError::OutsideRoots);
                    };
                    trailing.push(name.to_os_string());
                    if !existing.pop() {
                        return Err(FsProxyError::OutsideRoots);
                    }
                }
            }
        }
    }
}

fn validate_shape(path: &Path) -> Result<(), FsProxyError> {
    if !path.is_absolute() {
        return Err(FsProxyError::InvalidPath("must be absolute".to_string()));
    }
    let text = path.to_string_lossy();
    if text.chars().any(char::is_control) {
        return Err(FsProxyError::InvalidPath("control characters".to_string()));
    }
    if text.starts_with("\\\\") || text.starts_with("//") {
        return Err(FsProxyError::InvalidPath(
            "UNC and device paths".to_string(),
        ));
    }
    for component in path.components() {
        match component {
            Component::ParentDir => {
                return Err(FsProxyError::InvalidPath("parent traversal".to_string()))
            }
            Component::Prefix(prefix) => {
                let raw = prefix.as_os_str().to_string_lossy();
                if raw.starts_with("\\\\") {
                    return Err(FsProxyError::InvalidPath("UNC prefix".to_string()));
                }
            }
            _ => {}
        }
    }
    Ok(())
}

pub(crate) fn read_text_file(
    roots: &AuthorizedRoots,
    requested: &str,
    line: Option<u64>,
    limit: Option<u64>,
) -> Result<String, FsProxyError> {
    let path = roots.resolve(requested)?;
    let metadata = fs::metadata(&path).map_err(|error| {
        if error.kind() == std::io::ErrorKind::NotFound {
            FsProxyError::NotFound
        } else {
            FsProxyError::Io(error.to_string())
        }
    })?;
    if !metadata.is_file() {
        return Err(FsProxyError::InvalidPath("not a regular file".to_string()));
    }
    if metadata.len() > FS_PROXY_MAX_BYTES {
        return Err(FsProxyError::TooLarge {
            limit: FS_PROXY_MAX_BYTES,
        });
    }
    let content = fs::read_to_string(&path).map_err(|error| FsProxyError::Io(error.to_string()))?;
    let start = line.unwrap_or(1).max(1) as usize - 1;
    match (start, limit) {
        (0, None) => Ok(content),
        _ => {
            let lines: Vec<&str> = content.lines().collect();
            let end = limit
                .map(|limit| start.saturating_add(limit as usize))
                .unwrap_or(lines.len())
                .min(lines.len());
            if start >= lines.len() {
                return Ok(String::new());
            }
            Ok(lines[start..end].join("\n"))
        }
    }
}

pub(crate) fn write_text_file(
    roots: &AuthorizedRoots,
    requested: &str,
    content: &str,
) -> Result<(), FsProxyError> {
    if content.len() as u64 > FS_PROXY_MAX_BYTES {
        return Err(FsProxyError::TooLarge {
            limit: FS_PROXY_MAX_BYTES,
        });
    }
    let path = roots.resolve(requested)?;
    let parent = path
        .parent()
        .ok_or_else(|| FsProxyError::InvalidPath("no parent directory".to_string()))?;
    fs::create_dir_all(parent).map_err(|error| FsProxyError::Io(error.to_string()))?;
    // Re-validate after creating parents: the ancestor chain is what a link swap would attack.
    let canonical_parent =
        fs::canonicalize(parent).map_err(|error| FsProxyError::Io(error.to_string()))?;
    if !roots.contains(&canonical_parent) {
        return Err(FsProxyError::OutsideRoots);
    }
    if path.exists() && fs::symlink_metadata(&path).is_ok_and(|meta| meta.file_type().is_symlink())
    {
        // Canonicalized target was inside the roots, but a symlink is replaced, never followed
        // for the write itself, so the rename below cannot be redirected.
        fs::remove_file(&path).map_err(|error| FsProxyError::Io(error.to_string()))?;
    }
    let file_name = path
        .file_name()
        .ok_or_else(|| FsProxyError::InvalidPath("no file name".to_string()))?
        .to_string_lossy()
        .to_string();
    let temporary = canonical_parent.join(format!(
        ".{file_name}.vanehub-acp-{}.tmp",
        std::process::id()
    ));
    let write = (|| -> std::io::Result<()> {
        let mut file = fs::File::create(&temporary)?;
        file.write_all(content.as_bytes())?;
        file.sync_all()?;
        fs::rename(&temporary, canonical_parent.join(&file_name))
    })();
    if let Err(error) = write {
        let _ = fs::remove_file(&temporary);
        return Err(FsProxyError::Io(error.to_string()));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::TempDirectory;

    #[test]
    fn reads_and_writes_inside_the_workspace_and_refuses_outside() {
        let workspace = TempDirectory::new("acp-fs-proxy");
        let outside = TempDirectory::new("acp-fs-outside");
        let roots = AuthorizedRoots::new([workspace.path().to_path_buf()]);
        assert!(!roots.is_empty());

        let inside = workspace.path().join("notes").join("a.txt");
        write_text_file(&roots, &inside.to_string_lossy(), "第一行\n第二行\n第三行")
            .expect("write inside");
        assert_eq!(
            read_text_file(&roots, &inside.to_string_lossy(), None, None).expect("read"),
            "第一行\n第二行\n第三行"
        );
        assert_eq!(
            read_text_file(&roots, &inside.to_string_lossy(), Some(2), Some(1)).expect("slice"),
            "第二行"
        );
        assert_eq!(
            read_text_file(&roots, &inside.to_string_lossy(), Some(9), None).expect("past end"),
            ""
        );
        assert!(!workspace.path().read_dir().expect("dir").any(|entry| {
            entry
                .expect("entry")
                .file_name()
                .to_string_lossy()
                .contains(".tmp")
        }));

        let escape = outside.path().join("victim.txt");
        assert_eq!(
            write_text_file(&roots, &escape.to_string_lossy(), "x").expect_err("outside"),
            FsProxyError::OutsideRoots
        );
        assert!(!escape.exists());
        let traversal = workspace.path().join("..").join("victim.txt");
        assert!(matches!(
            write_text_file(&roots, &traversal.to_string_lossy(), "x"),
            Err(FsProxyError::InvalidPath(_))
        ));
        assert!(matches!(
            read_text_file(&roots, "relative/path.txt", None, None),
            Err(FsProxyError::InvalidPath(_))
        ));
        assert_eq!(
            read_text_file(
                &roots,
                &workspace.path().join("missing.txt").to_string_lossy(),
                None,
                None
            )
            .expect_err("missing"),
            FsProxyError::NotFound
        );
    }

    #[cfg(unix)]
    #[test]
    fn a_symlink_out_of_the_workspace_is_refused_without_touching_the_target() {
        let workspace = TempDirectory::new("acp-fs-link");
        let outside = TempDirectory::new("acp-fs-link-target");
        let roots = AuthorizedRoots::new([workspace.path().to_path_buf()]);
        let target = outside.path().join("secret.txt");
        fs::write(&target, "keep").expect("target");
        let link = workspace.path().join("link.txt");
        std::os::unix::fs::symlink(&target, &link).expect("symlink");
        assert_eq!(
            read_text_file(&roots, &link.to_string_lossy(), None, None).expect_err("escape"),
            FsProxyError::OutsideRoots
        );
        assert_eq!(
            write_text_file(&roots, &link.to_string_lossy(), "overwritten").expect_err("escape"),
            FsProxyError::OutsideRoots
        );
        assert_eq!(fs::read_to_string(&target).expect("target intact"), "keep");

        // A linked directory pointing outside is equally an escape for a new file beneath it.
        let linked_dir = workspace.path().join("linked");
        std::os::unix::fs::symlink(outside.path(), &linked_dir).expect("dir symlink");
        let beneath = linked_dir.join("new.txt");
        assert_eq!(
            write_text_file(&roots, &beneath.to_string_lossy(), "x").expect_err("escape"),
            FsProxyError::OutsideRoots
        );
        assert!(!outside.path().join("new.txt").exists());
    }

    #[test]
    fn device_and_unc_shapes_are_rejected_before_resolution() {
        let roots = AuthorizedRoots::new([std::env::temp_dir()]);
        for path in ["\\\\server\\share\\x", "//server/share/x", "/tmp/\u{0}x"] {
            assert!(
                matches!(
                    read_text_file(&roots, path, None, None),
                    Err(FsProxyError::InvalidPath(_))
                ),
                "{path}"
            );
        }
        assert_eq!(
            FsProxyError::TooLarge { limit: 3 }.message(),
            "file exceeds the 3-byte proxy limit"
        );
        assert!(AuthorizedRoots::new([PathBuf::from("/definitely/missing/root")]).is_empty());
    }
}
