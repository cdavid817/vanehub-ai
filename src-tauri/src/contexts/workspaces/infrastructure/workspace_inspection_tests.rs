//! Where a target comes from, and which provider gets to answer.
//!
//! Against a real database because the rule under test is about a *registered* binding: a double
//! would let the test decide what the session says, which is precisely the input the production
//! rule refuses to take from anybody.
//!
//! The local provider's own reads are not re-proved here. They are the confined implementations
//! this seam wraps, and their bounds — path escape, symlink, size, locale, diff — are already
//! covered where they live. Re-asserting them through the provider would test the delegation twice
//! and the bounds nowhere new.

use super::workspace_inspection::SessionWorkspaceTargetResolver;
use crate::contexts::workspaces::application::SearchCancellationToken;
use crate::contexts::workspaces::application::{
    CapabilityState, DirectoryFingerprint, DirectoryListing, DocumentListing, FileContent,
    FileSearchListing, GitDiffRequest, GitDiffResult, GitStatusResult, ListDirectoryRequest,
    ReadTextFileRequest, RemoteWorkspaceTarget, WatchMode, WorkspaceContentSearchRequest,
    WorkspaceContentSearchResult, WorkspaceInspectionCapabilities, WorkspaceInspectionError,
    WorkspaceInspectionProvider, WorkspaceInspectionRouter, WorkspacePathSearchRequest,
    WorkspacePathSearchResult, WorkspaceSearchCoverage, WorkspaceSearchRequest, WorkspaceTarget,
    WorkspaceTargetResolver,
};
use crate::platform::database::NativeDatabase;
use crate::test_support::TempDirectory;
use rusqlite::params;
use std::fs;
use std::sync::{Arc, Mutex};

struct Fixture {
    _directory: TempDirectory,
    database: NativeDatabase,
}

fn fixture(label: &str) -> Fixture {
    let directory = TempDirectory::new(label);
    let workspace = directory.path().join("workspace");
    fs::create_dir_all(&workspace).expect("workspace directory");
    let database = NativeDatabase::new(directory.path().join("data")).expect("database");
    let connection = database.connection().expect("connection");
    connection
        .execute(
            "INSERT INTO sessions \
             (id, title, agent_id, interaction_mode, lifecycle_state, folder, pinned, archived, \
              created_at, updated_at) \
             VALUES ('session-local', 'Local', 'codex-cli', 'cli', 'idle', ?1, 0, 0, \
                     '2026-08-25T10:00:00Z', '2026-08-25T10:00:00Z')",
            params![workspace.to_string_lossy().as_ref()],
        )
        .expect("insert local session");
    connection
        .execute(
            "INSERT INTO sessions \
             (id, title, agent_id, interaction_mode, lifecycle_state, remote_workspace_host, \
              remote_workspace_path, remote_workspace_display_name, remote_workspace_uri, \
              remote_ssh_connection_id, remote_ssh_connection_revision, pinned, archived, \
              created_at, updated_at) \
             VALUES ('session-remote', 'Remote', 'codex-cli', 'cli', 'idle', 'example.com', \
                     '/work/app', 'Remote app', 'ssh://example.com/work/app', 'connection-1', 7, \
                     0, 0, '2026-08-25T10:00:00Z', '2026-08-25T10:00:00Z')",
            [],
        )
        .expect("insert remote session");
    connection
        .execute(
            "INSERT INTO sessions \
             (id, title, agent_id, interaction_mode, lifecycle_state, remote_workspace_host, \
              remote_workspace_path, remote_workspace_display_name, remote_workspace_uri, \
              pinned, archived, created_at, updated_at) \
             VALUES ('session-unbound', 'Unbound', 'codex-cli', 'cli', 'idle', 'example.com', \
                     '/work/app', 'Remote app', 'ssh://example.com/work/app', 0, 0, \
                     '2026-08-25T10:00:00Z', '2026-08-25T10:00:00Z')",
            [],
        )
        .expect("insert unbound remote session");
    drop(connection);
    Fixture {
        _directory: directory,
        database,
    }
}

fn resolver(fixture: &Fixture) -> SessionWorkspaceTargetResolver {
    SessionWorkspaceTargetResolver::new(fixture.database.clone())
}

#[test]
fn a_local_session_resolves_to_its_canonical_root() {
    let fixture = fixture("inspection-local-target");

    let target = resolver(&fixture).resolve("session-local").expect("target");

    match target {
        WorkspaceTarget::Local(local) => {
            assert_eq!(local.session_id, "session-local");
            // Canonical, not the configured string: a provider that confined against unresolved
            // text would be guarding a boundary that `..` can still walk out of.
            assert!(local.root.is_absolute());
        }
        WorkspaceTarget::Remote(_) => panic!("a local session resolved to a remote target"),
    }
}

#[test]
fn a_remote_session_resolves_to_its_registered_binding() {
    let fixture = fixture("inspection-remote-target");

    let target = resolver(&fixture)
        .resolve("session-remote")
        .expect("target");

    match target {
        WorkspaceTarget::Remote(remote) => {
            assert_eq!(remote.connection_id, "connection-1");
            // The revision travels with the target. A profile edited between two reads must not
            // silently change which machine an answer came from.
            assert_eq!(remote.connection_revision, 7);
            assert_eq!(remote.root, "/work/app");
            assert_eq!(remote.display_name, "Remote app");
        }
        WorkspaceTarget::Local(_) => panic!("a remote session resolved to a local target"),
    }
}

/// A remote workspace with no SSH binding is refused, not quietly answered locally.
#[test]
fn a_remote_session_without_a_binding_is_unavailable_rather_than_local() {
    let fixture = fixture("inspection-unbound-target");

    let error = resolver(&fixture)
        .resolve("session-unbound")
        .expect_err("refusal");

    // Falling back would show this machine's files under a remote host's name: real files,
    // plausible paths, and nothing on screen saying which computer they came from.
    assert_eq!(
        error,
        WorkspaceInspectionError::TargetUnavailable("workspace_remote_binding_missing")
    );
}

#[test]
fn an_unknown_session_has_no_target() {
    let fixture = fixture("inspection-unknown-target");

    let error = resolver(&fixture)
        .resolve("session-absent")
        .expect_err("refusal");

    assert_eq!(
        error,
        WorkspaceInspectionError::TargetUnavailable("workspace_session_not_found")
    );
}

/// The resolver takes a session id and nothing else.
///
/// Asserted against the source because the property is the absence of a parameter, and absence is
/// not something a call can demonstrate: a test that passed only a session id would pass equally
/// well against a signature that also accepted a root.
#[test]
fn nothing_lets_a_caller_name_a_root() {
    let port = include_str!("../application/inspection.rs");
    let resolver_source = include_str!("workspace_inspection.rs");

    // One method, one argument. A second parameter here would be the frontend-supplied absolute
    // root that every confinement rule below exists to make impossible.
    assert!(port.contains("fn resolve(&self, session_id: &str)"));
    // And the struct literals that build a target live only in the resolver.
    for constructor in [
        "WorkspaceTarget::Local(LocalWorkspaceTarget",
        "WorkspaceTarget::Remote(RemoteWorkspaceTarget",
    ] {
        assert_eq!(
            resolver_source.matches(constructor).count(),
            1,
            "{constructor} is constructed somewhere other than the resolver"
        );
    }
}

// -------------------------------------------------------------------------------------------
// Selection
// -------------------------------------------------------------------------------------------

/// Answers nothing and records that it was asked.
///
/// A double rather than the real local provider because the property under test is *which*
/// provider ran, and the real one needs an app handle a unit test cannot construct. What the real
/// one adds - delegating to the confined reads - is proved where those reads live.
#[derive(Default)]
struct RecordingProvider {
    calls: Mutex<Vec<String>>,
}

#[async_trait::async_trait]
impl WorkspaceInspectionProvider for RecordingProvider {
    async fn capabilities(
        &self,
        target: &WorkspaceTarget,
    ) -> Result<WorkspaceInspectionCapabilities, WorkspaceInspectionError> {
        self.calls
            .lock()
            .expect("calls")
            .push("capabilities".to_string());
        Ok(WorkspaceInspectionCapabilities {
            provider: target.provider(),
            list_files: CapabilityState::available(),
            read_text_files: CapabilityState::available(),
            search_files: CapabilityState::available(),
            git_status: CapabilityState::available(),
            git_diff: CapabilityState::available(),
            watch_mode: WatchMode::EventDerived,
        })
    }

    async fn search_content(
        &self,
        _target: &WorkspaceTarget,
        request: WorkspaceContentSearchRequest,
        _cancellation: crate::contexts::workspaces::application::SearchCancellationToken,
    ) -> Result<WorkspaceContentSearchResult, WorkspaceInspectionError> {
        self.calls
            .lock()
            .expect("calls")
            .push(format!("search_content:{}", request.query));
        Ok(WorkspaceContentSearchResult {
            coverage: WorkspaceSearchCoverage::complete(),
            matches: Vec::new(),
        })
    }

    async fn search_paths(
        &self,
        _target: &WorkspaceTarget,
        request: WorkspacePathSearchRequest,
        _execution: crate::contexts::workspaces::application::WorkspaceInspectionExecution,
    ) -> Result<WorkspacePathSearchResult, WorkspaceInspectionError> {
        self.calls
            .lock()
            .expect("calls")
            .push(format!("search_paths:{}", request.query));
        Ok(WorkspacePathSearchResult {
            coverage: WorkspaceSearchCoverage::complete(),
            matches: Vec::new(),
            next_cursor: None,
        })
    }

    async fn directory_fingerprints(
        &self,
        _target: &WorkspaceTarget,
        paths: &[String],
    ) -> Result<Vec<DirectoryFingerprint>, WorkspaceInspectionError> {
        self.calls
            .lock()
            .expect("calls")
            .push(format!("fingerprints:{}", paths.len()));
        Ok(Vec::new())
    }

    async fn list_directory(
        &self,
        target: &WorkspaceTarget,
        _request: ListDirectoryRequest,
    ) -> Result<DirectoryListing, WorkspaceInspectionError> {
        self.calls
            .lock()
            .expect("calls")
            .push(format!("list:{}", target.session_id()));
        Err(WorkspaceInspectionError::NotFound)
    }

    async fn list_documents(
        &self,
        _target: &WorkspaceTarget,
        _cancellation: SearchCancellationToken,
    ) -> Result<DocumentListing, WorkspaceInspectionError> {
        Err(WorkspaceInspectionError::NotFound)
    }

    async fn read_text_file(
        &self,
        _target: &WorkspaceTarget,
        _request: ReadTextFileRequest,
    ) -> Result<FileContent, WorkspaceInspectionError> {
        Err(WorkspaceInspectionError::NotFound)
    }

    async fn search(
        &self,
        _target: &WorkspaceTarget,
        _request: WorkspaceSearchRequest,
    ) -> Result<FileSearchListing, WorkspaceInspectionError> {
        Err(WorkspaceInspectionError::NotFound)
    }

    async fn git_status(
        &self,
        _target: &WorkspaceTarget,
    ) -> Result<GitStatusResult, WorkspaceInspectionError> {
        Err(WorkspaceInspectionError::NotFound)
    }

    async fn git_diff(
        &self,
        _target: &WorkspaceTarget,
        _request: GitDiffRequest,
    ) -> Result<GitDiffResult, WorkspaceInspectionError> {
        Err(WorkspaceInspectionError::NotFound)
    }
}

fn router(fixture: &Fixture, local: Arc<RecordingProvider>) -> WorkspaceInspectionRouter {
    WorkspaceInspectionRouter::new(Arc::new(resolver(fixture)), local)
}

/// A remote session gets `unsupported`, never the local provider.
///
/// The provider's own refusal is a second line of defence; a test that only exercised that one
/// would pass while the router happily routed remote sessions to local.
#[test]
fn a_remote_session_never_reaches_the_local_provider() {
    let fixture = fixture("inspection-remote-selection");
    let local = Arc::new(RecordingProvider::default());
    let router = router(&fixture, local.clone());

    let error = tauri::async_runtime::block_on(
        router.list_directory("session-remote", ListDirectoryRequest::default()),
    )
    .expect_err("refusal");

    assert_eq!(
        error,
        WorkspaceInspectionError::Unsupported("workspace_remote_inspection_unavailable")
    );
    assert!(local.calls.lock().expect("calls").is_empty());
}

#[test]
fn a_local_session_reaches_the_local_provider() {
    let fixture = fixture("inspection-local-selection");
    let local = Arc::new(RecordingProvider::default());
    let router = router(&fixture, local.clone());

    let _ = tauri::async_runtime::block_on(
        router.list_directory("session-local", ListDirectoryRequest::default()),
    );

    assert_eq!(
        *local.calls.lock().expect("calls"),
        vec!["list:session-local".to_string()]
    );
}

#[test]
fn a_registered_remote_provider_takes_the_remote_session() {
    let fixture = fixture("inspection-remote-registered");
    let local = Arc::new(RecordingProvider::default());
    let remote = Arc::new(RecordingProvider::default());
    let router = router(&fixture, local.clone()).with_remote(remote.clone());

    let _ = tauri::async_runtime::block_on(
        router.list_directory("session-remote", ListDirectoryRequest::default()),
    );

    // The routing follows the target, not the order the providers were registered in.
    assert!(local.calls.lock().expect("calls").is_empty());
    assert_eq!(
        *remote.calls.lock().expect("calls"),
        vec!["list:session-remote".to_string()]
    );
}

/// The local provider refuses a remote target even if a router ever handed it one.
#[test]
fn the_local_provider_refuses_a_remote_target() {
    let target = WorkspaceTarget::Remote(RemoteWorkspaceTarget {
        session_id: "session-remote".to_string(),
        connection_id: "connection-1".to_string(),
        connection_revision: 7,
        root: "/work/app".to_string(),
        display_name: "Remote app".to_string(),
    });

    let error = super::workspace_inspection::require_local(&target).expect_err("refusal");

    assert_eq!(
        error,
        WorkspaceInspectionError::Unsupported("workspace_provider_local_only")
    );
}
