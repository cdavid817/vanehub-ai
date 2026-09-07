//! A worktree the application created, tracked independently of the sessions that use it.
//!
//! Git can prove *what* a directory is; only this record can prove *why* the application is
//! allowed to remove it on a user's behalf. The two are deliberately separate, and neither
//! substitutes for the other: a directory that looks exactly like one of ours is still not ours
//! until a record says so, and a record whose Git identity no longer matches is no longer a
//! licence to delete anything.

use super::WorkspaceDomainError;

/// Who created the worktree. Only `OrdinarySession` ever becomes cleanup-eligible here; Loop and
/// sub-Agent worktrees keep their own retention policies, and external ones were never ours.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum WorktreeOrigin {
    OrdinarySession,
    Loop,
    Subagent,
    External,
}

/// How much the application knows about why it may act on this worktree.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum WorktreeProvenance {
    /// Intent recorded, Git not yet run (or its result not yet bound).
    Provisioning,
    /// Created by this application through the ordinary session path, identity recorded after
    /// `git worktree add` returned.
    Verified,
    /// Predates provenance tracking, but session metadata, a successful creation operation, and
    /// the current Git identity all agree.
    LegacyVerified,
    /// Predates provenance tracking and the evidence is incomplete. Never cleanup-eligible.
    LegacyUnverified,
    /// Not created by this application. Never cleanup-eligible.
    External,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ManagedWorktreeStatus {
    Provisioning,
    Attached,
    Retained,
    Removing,
    Removed,
    NeedsAttention,
}

macro_rules! storage_enum {
    ($name:ident { $($variant:ident => $literal:literal),+ $(,)? }) => {
        impl $name {
            pub(crate) fn as_str(self) -> &'static str {
                match self {
                    $(Self::$variant => $literal,)+
                }
            }

            pub(crate) fn parse(value: &str) -> Result<Self, WorkspaceDomainError> {
                match value {
                    $($literal => Ok(Self::$variant),)+
                    _ => Err(WorkspaceDomainError::UnknownWorktreeState),
                }
            }
        }
    };
}

storage_enum!(WorktreeOrigin {
    OrdinarySession => "ordinary_session",
    Loop => "loop",
    Subagent => "subagent",
    External => "external",
});

storage_enum!(WorktreeProvenance {
    Provisioning => "provisioning",
    Verified => "verified",
    LegacyVerified => "legacy_verified",
    LegacyUnverified => "legacy_unverified",
    External => "external",
});

storage_enum!(ManagedWorktreeStatus {
    Provisioning => "provisioning",
    Attached => "attached",
    Retained => "retained",
    Removing => "removing",
    Removed => "removed",
    NeedsAttention => "needs_attention",
});

/// The Git facts recorded when the worktree became ours, and re-checked before it is removed.
///
/// Every path is the absolute form Git itself reported — never assembled from a basename — and
/// `canonical_root` is the filesystem's canonical form of the working directory. `fs_identity`
/// is whatever stable identity the platform offers for the root directory (device and inode on
/// Unix, the file index on Windows); `None` when the platform offers none, which is why it is
/// only ever one of several checks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct WorktreeIdentity {
    pub(crate) canonical_root: String,
    pub(crate) git_dir: String,
    pub(crate) common_dir: String,
    pub(crate) branch: Option<String>,
    pub(crate) head: Option<String>,
    pub(crate) fs_identity: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ManagedWorktree {
    pub(crate) id: String,
    pub(crate) origin: WorktreeOrigin,
    pub(crate) provenance: WorktreeProvenance,
    pub(crate) status: ManagedWorktreeStatus,
    /// The directory the intent targeted. Recorded before Git runs, so it is the requested path;
    /// `identity` carries the canonical form once Git has confirmed it.
    pub(crate) requested_root: String,
    pub(crate) project_root: String,
    pub(crate) identity: Option<WorktreeIdentity>,
    pub(crate) creation_operation_id: Option<String>,
    pub(crate) attention_reason: Option<String>,
    pub(crate) revision: u64,
    pub(crate) created_at: String,
    pub(crate) updated_at: String,
}

impl ManagedWorktree {
    pub(crate) fn provisioning(
        id: String,
        origin: WorktreeOrigin,
        project_root: String,
        requested_root: String,
        creation_operation_id: Option<String>,
        now: String,
    ) -> Result<Self, WorkspaceDomainError> {
        if id.trim().is_empty()
            || requested_root.trim().is_empty()
            || project_root.trim().is_empty()
        {
            return Err(WorkspaceDomainError::InvalidWorktreeResource);
        }
        Ok(Self {
            id,
            origin,
            provenance: WorktreeProvenance::Provisioning,
            status: ManagedWorktreeStatus::Provisioning,
            requested_root,
            project_root,
            identity: None,
            creation_operation_id,
            attention_reason: None,
            revision: 1,
            created_at: now.clone(),
            updated_at: now,
        })
    }

    /// A record for a worktree that predates provenance tracking and whose evidence was complete.
    pub(crate) fn legacy_verified(
        id: String,
        project_root: String,
        identity: WorktreeIdentity,
        creation_operation_id: Option<String>,
        now: String,
    ) -> Result<Self, WorkspaceDomainError> {
        let mut record = Self::provisioning(
            id,
            WorktreeOrigin::OrdinarySession,
            project_root,
            identity.canonical_root.clone(),
            creation_operation_id,
            now,
        )?;
        record.provenance = WorktreeProvenance::LegacyVerified;
        record.status = ManagedWorktreeStatus::Attached;
        record.identity = Some(identity);
        Ok(record)
    }

    /// Whether the record alone permits cleanup to be *offered*. Every other check still applies.
    pub(crate) fn cleanup_eligible(&self) -> bool {
        self.origin == WorktreeOrigin::OrdinarySession
            && matches!(
                self.provenance,
                WorktreeProvenance::Verified | WorktreeProvenance::LegacyVerified
            )
            && matches!(
                self.status,
                ManagedWorktreeStatus::Attached | ManagedWorktreeStatus::Retained
            )
            && self.identity.is_some()
    }

    pub(crate) fn confirm_created(
        &mut self,
        identity: WorktreeIdentity,
        now: String,
    ) -> Result<(), WorkspaceDomainError> {
        if self.status != ManagedWorktreeStatus::Provisioning {
            return Err(WorkspaceDomainError::InvalidWorktreeTransition {
                from: self.status.as_str(),
                to: ManagedWorktreeStatus::Attached.as_str(),
            });
        }
        self.provenance = WorktreeProvenance::Verified;
        self.status = ManagedWorktreeStatus::Attached;
        self.identity = Some(identity);
        self.touch(now);
        Ok(())
    }

    pub(crate) fn begin_removal(&mut self, now: String) -> Result<(), WorkspaceDomainError> {
        if !self.cleanup_eligible() {
            return Err(WorkspaceDomainError::InvalidWorktreeTransition {
                from: self.status.as_str(),
                to: ManagedWorktreeStatus::Removing.as_str(),
            });
        }
        self.status = ManagedWorktreeStatus::Removing;
        self.touch(now);
        Ok(())
    }

    /// Removal did not happen and the directory was observed intact: back to the state it had.
    pub(crate) fn removal_refused(&mut self, now: String) -> Result<(), WorkspaceDomainError> {
        if self.status != ManagedWorktreeStatus::Removing {
            return Err(WorkspaceDomainError::InvalidWorktreeTransition {
                from: self.status.as_str(),
                to: ManagedWorktreeStatus::Attached.as_str(),
            });
        }
        self.status = ManagedWorktreeStatus::Attached;
        self.touch(now);
        Ok(())
    }

    pub(crate) fn mark_removed(&mut self, now: String) -> Result<(), WorkspaceDomainError> {
        if !matches!(
            self.status,
            ManagedWorktreeStatus::Removing | ManagedWorktreeStatus::NeedsAttention
        ) {
            return Err(WorkspaceDomainError::InvalidWorktreeTransition {
                from: self.status.as_str(),
                to: ManagedWorktreeStatus::Removed.as_str(),
            });
        }
        self.status = ManagedWorktreeStatus::Removed;
        self.touch(now);
        Ok(())
    }

    pub(crate) fn mark_retained(&mut self, now: String) -> Result<(), WorkspaceDomainError> {
        if !matches!(
            self.status,
            ManagedWorktreeStatus::Attached | ManagedWorktreeStatus::Retained
        ) {
            return Err(WorkspaceDomainError::InvalidWorktreeTransition {
                from: self.status.as_str(),
                to: ManagedWorktreeStatus::Retained.as_str(),
            });
        }
        self.status = ManagedWorktreeStatus::Retained;
        self.touch(now);
        Ok(())
    }

    /// Any state may need attention; nothing automatic ever leaves it.
    pub(crate) fn mark_needs_attention(&mut self, reason: &str, now: String) {
        self.status = ManagedWorktreeStatus::NeedsAttention;
        self.attention_reason = Some(reason.to_string());
        self.touch(now);
    }

    fn touch(&mut self, now: String) {
        self.revision += 1;
        self.updated_at = now;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn identity(root: &str) -> WorktreeIdentity {
        WorktreeIdentity {
            canonical_root: root.to_string(),
            git_dir: format!("{root}/.git-dir"),
            common_dir: "/repo/.git".to_string(),
            branch: Some("vanehub/feature".to_string()),
            head: Some("abc".to_string()),
            fs_identity: Some("1:2".to_string()),
        }
    }

    fn provisioning() -> ManagedWorktree {
        ManagedWorktree::provisioning(
            "wt-1".to_string(),
            WorktreeOrigin::OrdinarySession,
            "/repo".to_string(),
            "/repo-feature".to_string(),
            Some("op-1".to_string()),
            "t0".to_string(),
        )
        .expect("record")
    }

    #[test]
    fn provisioning_records_are_never_cleanup_eligible() {
        let record = provisioning();
        assert!(!record.cleanup_eligible());
        assert_eq!(record.status, ManagedWorktreeStatus::Provisioning);
        assert_eq!(record.provenance, WorktreeProvenance::Provisioning);
    }

    #[test]
    fn confirmation_makes_an_ordinary_worktree_eligible_and_bumps_revision() {
        let mut record = provisioning();
        record
            .confirm_created(identity("/repo-feature"), "t1".to_string())
            .expect("confirm");
        assert!(record.cleanup_eligible());
        assert_eq!(record.revision, 2);
        assert_eq!(record.updated_at, "t1");
        assert!(record
            .confirm_created(identity("/x"), "t2".to_string())
            .is_err());
    }

    #[test]
    fn loop_and_external_origins_never_become_eligible() {
        for origin in [
            WorktreeOrigin::Loop,
            WorktreeOrigin::Subagent,
            WorktreeOrigin::External,
        ] {
            let mut record = ManagedWorktree::provisioning(
                "wt".to_string(),
                origin,
                "/repo".to_string(),
                "/repo-x".to_string(),
                None,
                "t0".to_string(),
            )
            .expect("record");
            record
                .confirm_created(identity("/repo-x"), "t1".to_string())
                .expect("confirm");
            assert!(!record.cleanup_eligible(), "{origin:?}");
            assert!(record.begin_removal("t2".to_string()).is_err());
        }
    }

    #[test]
    fn removal_transitions_are_guarded_and_attention_is_sticky() {
        let mut record = provisioning();
        assert!(record.begin_removal("t1".to_string()).is_err());
        record
            .confirm_created(identity("/repo-feature"), "t1".to_string())
            .expect("confirm");
        record.begin_removal("t2".to_string()).expect("removing");
        assert!(!record.cleanup_eligible());
        record.removal_refused("t3".to_string()).expect("refused");
        assert_eq!(record.status, ManagedWorktreeStatus::Attached);
        record
            .begin_removal("t4".to_string())
            .expect("removing again");
        record.mark_removed("t5".to_string()).expect("removed");
        assert!(record.mark_retained("t6".to_string()).is_err());
        record.mark_needs_attention("removal_unknown", "t7".to_string());
        assert_eq!(record.status, ManagedWorktreeStatus::NeedsAttention);
        assert_eq!(record.attention_reason.as_deref(), Some("removal_unknown"));
        assert!(!record.cleanup_eligible());
    }

    #[test]
    fn storage_literals_round_trip() {
        for value in [
            WorktreeProvenance::Provisioning,
            WorktreeProvenance::Verified,
            WorktreeProvenance::LegacyVerified,
            WorktreeProvenance::LegacyUnverified,
            WorktreeProvenance::External,
        ] {
            assert_eq!(WorktreeProvenance::parse(value.as_str()), Ok(value));
        }
        assert!(ManagedWorktreeStatus::parse("bogus").is_err());
        assert_eq!(
            WorktreeOrigin::parse("ordinary_session"),
            Ok(WorktreeOrigin::OrdinarySession)
        );
    }
}
