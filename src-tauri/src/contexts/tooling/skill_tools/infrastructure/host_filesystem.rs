use super::invocation_budget::SkillToolInvocationBudget;
use crate::contexts::tooling::skill_tools::application::SkillToolApplicationError;
use crate::contexts::tooling::skill_tools::domain::{SkillFilesystemPermissions, SkillToolLimits};
use crate::platform::filesystem::{BoundaryError, BoundedFilesystem};
use globset::{Glob, GlobMatcher};
use std::io::Write;
use std::path::Path;

pub(crate) struct SkillToolFilesystemGateway {
    boundary: BoundedFilesystem,
    read_scopes: Vec<GlobMatcher>,
    write_scopes: Vec<GlobMatcher>,
    budget: SkillToolInvocationBudget,
    temporary_directory: tempfile::TempDir,
}

impl SkillToolFilesystemGateway {
    pub(crate) fn new(
        workspace_root: &Path,
        temporary_root: &Path,
        permissions: &SkillFilesystemPermissions,
        limits: SkillToolLimits,
    ) -> Result<Self, SkillToolApplicationError> {
        Self::with_budget(
            workspace_root,
            temporary_root,
            permissions,
            SkillToolInvocationBudget::new(limits),
        )
    }

    pub(crate) fn with_budget(
        workspace_root: &Path,
        temporary_root: &Path,
        permissions: &SkillFilesystemPermissions,
        budget: SkillToolInvocationBudget,
    ) -> Result<Self, SkillToolApplicationError> {
        std::fs::create_dir_all(temporary_root).map_err(filesystem_error)?;
        let temporary_directory = tempfile::Builder::new()
            .prefix("vanehub-skill-tool-")
            .tempdir_in(temporary_root)
            .map_err(filesystem_error)?;
        Ok(Self {
            boundary: BoundedFilesystem::new(workspace_root).map_err(boundary_error)?,
            read_scopes: compile_scopes(&permissions.read)?,
            write_scopes: compile_scopes(&permissions.write)?,
            budget,
            temporary_directory,
        })
    }

    pub(crate) fn temporary_directory(&self) -> &Path {
        self.temporary_directory.path()
    }

    pub(crate) fn read(&mut self, relative: &str) -> Result<Vec<u8>, SkillToolApplicationError> {
        admit(&self.read_scopes, relative, "filesystem.read")?;
        self.budget.reserve_host_call()?;
        let target = self
            .boundary
            .resolve_existing(relative)
            .map_err(boundary_error)?;
        let metadata = std::fs::metadata(&target).map_err(filesystem_error)?;
        if !metadata.is_file() {
            return Err(SkillToolApplicationError::HostDenied(
                "filesystem.read.not-file".to_string(),
            ));
        }
        self.budget.consume_file(metadata.len())?;
        self.budget.consume_output(metadata.len())?;
        std::fs::read(target).map_err(filesystem_error)
    }

    pub(crate) fn write(
        &mut self,
        relative: &str,
        bytes: &[u8],
    ) -> Result<(), SkillToolApplicationError> {
        admit(&self.write_scopes, relative, "filesystem.write")?;
        self.budget.reserve_host_call()?;
        self.budget.consume_file(bytes.len() as u64)?;
        let target = match self.boundary.resolve_existing(relative) {
            Ok(existing) => existing,
            Err(BoundaryError::Io(error)) if error.kind() == std::io::ErrorKind::NotFound => {
                self.boundary
                    .resolve_with_existing_parent(relative)
                    .map_err(boundary_error)?
                    .0
            }
            Err(error) => return Err(boundary_error(error)),
        };
        if target.exists() {
            let metadata = std::fs::symlink_metadata(&target).map_err(filesystem_error)?;
            if !metadata.file_type().is_file() || metadata.file_type().is_symlink() {
                return Err(SkillToolApplicationError::HostDenied(
                    "filesystem.write.not-regular-file".to_string(),
                ));
            }
        }
        atomic_write(&target, bytes)
    }
}

fn atomic_write(target: &Path, bytes: &[u8]) -> Result<(), SkillToolApplicationError> {
    let parent = target.parent().ok_or_else(|| {
        SkillToolApplicationError::HostDenied("filesystem.write.parent".to_string())
    })?;
    let mut temporary = tempfile::NamedTempFile::new_in(parent).map_err(filesystem_error)?;
    temporary.write_all(bytes).map_err(filesystem_error)?;
    temporary.as_file().sync_all().map_err(filesystem_error)?;
    temporary
        .persist(target)
        .map(|_| ())
        .map_err(|error| filesystem_error(error.error))
}

fn compile_scopes(scopes: &[String]) -> Result<Vec<GlobMatcher>, SkillToolApplicationError> {
    scopes
        .iter()
        .map(|scope| {
            let relative = scope.strip_prefix("workspace/").ok_or_else(|| {
                SkillToolApplicationError::HostDenied("filesystem.scope".to_string())
            })?;
            Glob::new(relative)
                .map(|glob| glob.compile_matcher())
                .map_err(|_| SkillToolApplicationError::HostDenied("filesystem.scope".to_string()))
        })
        .collect()
}

fn admit(
    scopes: &[GlobMatcher],
    relative: &str,
    action: &str,
) -> Result<(), SkillToolApplicationError> {
    if scopes.iter().any(|scope| scope.is_match(relative)) {
        Ok(())
    } else {
        Err(SkillToolApplicationError::HostDenied(action.to_string()))
    }
}

fn boundary_error(error: BoundaryError) -> SkillToolApplicationError {
    match error {
        BoundaryError::Io(error) => filesystem_error(error),
        _ => SkillToolApplicationError::HostDenied("filesystem.boundary".to_string()),
    }
}

fn filesystem_error(error: std::io::Error) -> SkillToolApplicationError {
    SkillToolApplicationError::Filesystem(error.kind().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contexts::tooling::skill_tools::domain::DEFAULT_SKILL_TOOL_LIMITS;
    use crate::test_support::TempDirectory;

    fn gateway<'a>(
        workspace: &'a TempDirectory,
        temporary: &'a TempDirectory,
        permissions: &SkillFilesystemPermissions,
    ) -> SkillToolFilesystemGateway {
        SkillToolFilesystemGateway::new(
            workspace.path(),
            temporary.path(),
            permissions,
            DEFAULT_SKILL_TOOL_LIMITS,
        )
        .expect("gateway")
    }

    #[test]
    fn read_and_write_are_admitted_independently() {
        let workspace = TempDirectory::new("skill-filesystem-workspace");
        let temporary = TempDirectory::new("skill-filesystem-temporary");
        std::fs::create_dir_all(workspace.path().join("src/generated")).expect("directories");
        std::fs::write(workspace.path().join("src/input.txt"), b"input").expect("input");
        let permissions = SkillFilesystemPermissions {
            read: vec!["workspace/src/**".to_string()],
            write: vec!["workspace/src/generated/**".to_string()],
        };
        let mut gateway = gateway(&workspace, &temporary, &permissions);

        assert_eq!(gateway.read("src/input.txt").expect("read"), b"input");
        gateway
            .write("src/generated/result.txt", b"result")
            .expect("write");
        assert!(gateway.write("src/input.txt", b"replace").is_err());
        assert!(gateway.temporary_directory().starts_with(temporary.path()));
    }

    #[test]
    fn traversal_hidden_paths_and_byte_exhaustion_fail_closed() {
        let workspace = TempDirectory::new("skill-filesystem-adversarial");
        let temporary = TempDirectory::new("skill-filesystem-adversarial-temp");
        std::fs::create_dir_all(workspace.path().join("src")).expect("src");
        std::fs::create_dir_all(workspace.path().join(".git")).expect("hidden");
        let permissions = SkillFilesystemPermissions {
            read: vec!["workspace/**".to_string()],
            write: vec!["workspace/**".to_string()],
        };
        let mut limits = DEFAULT_SKILL_TOOL_LIMITS;
        limits.file_bytes = 4;
        let mut gateway = SkillToolFilesystemGateway::new(
            workspace.path(),
            temporary.path(),
            &permissions,
            limits,
        )
        .expect("gateway");

        assert!(gateway.read("../outside").is_err());
        assert!(gateway.write(".git/config", b"bad").is_err());
        assert!(matches!(
            gateway.write("src/large.txt", b"large"),
            Err(SkillToolApplicationError::ResourceLimit(_))
        ));
    }

    #[cfg(unix)]
    #[test]
    fn symlink_escape_is_rejected_at_io_time() {
        use std::os::unix::fs::symlink;
        let workspace = TempDirectory::new("skill-filesystem-symlink");
        let temporary = TempDirectory::new("skill-filesystem-symlink-temp");
        let outside = TempDirectory::new("skill-filesystem-outside");
        let secret = outside.write("secret.txt", "secret");
        symlink(secret, workspace.path().join("escape.txt")).expect("symlink");
        let permissions = SkillFilesystemPermissions {
            read: vec!["workspace/**".to_string()],
            write: vec![],
        };
        let mut gateway = gateway(&workspace, &temporary, &permissions);
        assert!(gateway.read("escape.txt").is_err());
    }

    #[cfg(unix)]
    #[test]
    fn symlink_swap_is_revalidated_for_every_operation() {
        use std::os::unix::fs::symlink;
        let workspace = TempDirectory::new("skill-filesystem-swap");
        let temporary = TempDirectory::new("skill-filesystem-swap-temp");
        let outside = TempDirectory::new("skill-filesystem-swap-outside");
        let inside = workspace.write("inside.txt", "inside");
        let secret = outside.write("secret.txt", "secret");
        let link = workspace.path().join("current.txt");
        symlink(inside, &link).expect("inside link");
        let permissions = SkillFilesystemPermissions {
            read: vec!["workspace/**".to_string()],
            write: vec![],
        };
        let mut gateway = gateway(&workspace, &temporary, &permissions);
        assert_eq!(gateway.read("current.txt").expect("inside read"), b"inside");

        std::fs::remove_file(&link).expect("remove old link");
        symlink(secret, link).expect("outside link");
        assert!(gateway.read("current.txt").is_err());
    }
}
