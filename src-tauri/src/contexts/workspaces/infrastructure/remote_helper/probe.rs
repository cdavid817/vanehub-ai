//! What a remote host turns out to be able to do.
//!
//! Two checks happen before anything reaches the network, and they are the reason this is not just
//! "run the helper and see": the profile revision must still match the one the session was bound
//! to, and the host must still be trusted. A probe that skipped them would connect to whatever the
//! profile says *now* and report its capabilities under a session that was bound to something else.
//!
//! Every capability answers separately. A host with Git but no ripgrep is an ordinary host, and a
//! single availability flag would either hide the search gap or disable the four things that work.
//! The Shell stays reachable in every case — it needs none of this.

use super::protocol::{HelperOperation, HelperProbe, HelperRequest, RemoteHelperError};
use super::transport::{exchange, RemoteHelperSession};
use crate::contexts::workspaces::application::{
    CapabilityState, RemoteWorkspaceTarget, WatchMode, WorkspaceInspectionCapabilities,
};

/// Refuses a binding that has moved since the session was bound to it.
///
/// Three scalars rather than the connections API, so the decision can be seen without a network:
/// "the binding is stale" is a rule, and a rule that can only be observed by watching a connection
/// fail is a rule nobody can test. The lookup that produces these values belongs to the caller.
pub(crate) fn revalidate(
    bound_revision: i64,
    current_revision: i64,
    host_trusted: bool,
) -> Result<(), RemoteHelperError> {
    if current_revision != bound_revision {
        // The profile was edited after this session was bound to it. Reconnecting under the new
        // one would answer about a different machine while the reader believed they were still
        // looking at the first.
        return Err(RemoteHelperError::ProfileStale);
    }
    if !host_trusted {
        // Trust can be revoked between two reads, and a probe is the first thing a panel runs:
        // this is where a revoked host has to stop, not three requests later.
        return Err(RemoteHelperError::HostUntrusted);
    }
    Ok(())
}
/// Runs the probe and turns it into per-feature availability.
pub(crate) async fn probe_capabilities(
    session: &dyn RemoteHelperSession,
    target: &RemoteWorkspaceTarget,
) -> Result<WorkspaceInspectionCapabilities, RemoteHelperError> {
    let response = exchange(
        session,
        &target.connection_id,
        target.connection_revision,
        &HelperRequest::new(target.root.clone(), HelperOperation::Probe),
    )
    .await?;

    let probe = response
        .result
        .and_then(|result| result.probe)
        .ok_or(RemoteHelperError::MalformedResponse)?;
    Ok(capabilities_from(&probe))
}

/// The mapping from what was found to what may be offered.
///
/// A separate function because this is the part worth testing exhaustively, and it needs no
/// connection: every combination of found and missing prerequisites is a table, not a scenario.
pub(crate) fn capabilities_from(probe: &HelperProbe) -> WorkspaceInspectionCapabilities {
    // A non-POSIX host fails everything at once. The helper's path handling, its `realpath`
    // confinement, and its argument-array subprocess calls all assume POSIX semantics, and a
    // partial answer would offer operations whose safety argument does not hold.
    if !probe.posix {
        let unavailable = || {
            CapabilityState::unavailable("remote_host_not_posix")
                .with_remediation("remote_use_posix_host")
        };
        return WorkspaceInspectionCapabilities {
            provider: "ssh",
            list_files: unavailable(),
            read_text_files: unavailable(),
            search_files: unavailable(),
            git_status: unavailable(),
            git_diff: unavailable(),
            watch_mode: WatchMode::None,
        };
    }

    // A root that does not resolve to a readable directory is not a smaller workspace; it is a
    // workspace that is not there, and every read below it would fail one at a time.
    let filesystem = if probe.root_readable {
        CapabilityState::available()
    } else {
        CapabilityState::unavailable("remote_root_unreadable")
            .with_remediation("remote_check_workspace_path")
    };
    let git = |state: &CapabilityState| {
        if !state.available {
            // Git under an unreadable root would answer about whatever directory the command
            // happened to start in.
            return CapabilityState::unavailable("remote_root_unreadable")
                .with_remediation("remote_check_workspace_path");
        }
        if probe.git {
            CapabilityState::available()
        } else {
            CapabilityState::unavailable("remote_git_missing")
                .with_remediation("remote_install_git")
        }
    };

    WorkspaceInspectionCapabilities {
        provider: "ssh",
        list_files: filesystem.clone(),
        read_text_files: filesystem.clone(),
        // Search is ripgrep or nothing. A fallback that walked the tree in Python would be a second
        // search with different bounds, different ordering, and different ignore rules, reached
        // exactly when nobody could tell which one answered.
        search_files: if !filesystem.available {
            filesystem.clone()
        } else if probe.ripgrep {
            CapabilityState::available()
        } else {
            CapabilityState::unavailable("remote_ripgrep_missing")
                .with_remediation("remote_install_ripgrep")
        },
        git_status: git(&filesystem),
        git_diff: git(&filesystem),
        // Polling, because nothing on the remote host tells this process when a file changed. Saying
        // so is what stops a reader believing an external change would appear on its own.
        watch_mode: WatchMode::Polling,
    }
}
