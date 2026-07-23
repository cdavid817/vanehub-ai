use super::*;
use crate::contexts::workspaces::domain::{
    ProjectInspection, ProjectPath, RemoteWorkspace, WorktreeName,
};
use std::sync::{Arc, Mutex};

#[derive(Default)]
struct FakeHistoryState {
    projects: Vec<KnownProject>,
    remote_workspaces: Vec<KnownRemoteWorkspace>,
}

#[derive(Clone, Default)]
struct FakeHistory {
    state: Arc<Mutex<FakeHistoryState>>,
    calls: Arc<Mutex<Vec<String>>>,
}

impl WorkspaceHistoryRepository for FakeHistory {
    fn list_projects(&self) -> Result<Vec<KnownProject>, WorkspaceApplicationError> {
        self.calls
            .lock()
            .expect("calls")
            .push("history:list-projects".to_string());
        Ok(self.state.lock().expect("history").projects.clone())
    }

    fn list_remote_workspaces(
        &self,
    ) -> Result<Vec<KnownRemoteWorkspace>, WorkspaceApplicationError> {
        self.calls
            .lock()
            .expect("calls")
            .push("history:list-remote".to_string());
        Ok(self
            .state
            .lock()
            .expect("history")
            .remote_workspaces
            .clone())
    }

    fn remember_project(
        &self,
        inspection: &ProjectInspection,
        opened_at: &str,
    ) -> Result<(), WorkspaceApplicationError> {
        self.calls.lock().expect("calls").push(format!(
            "history:remember-project:{}:{opened_at}",
            inspection.path()
        ));
        Ok(())
    }

    fn remember_remote_workspace(
        &self,
        workspace: &RemoteWorkspace,
        opened_at: &str,
    ) -> Result<(), WorkspaceApplicationError> {
        self.calls.lock().expect("calls").push(format!(
            "history:remember-remote:{}:{opened_at}",
            workspace.uri()
        ));
        Ok(())
    }
}

#[derive(Clone)]
struct FakeFilesystem {
    canonical: String,
    target: String,
    calls: Arc<Mutex<Vec<String>>>,
}

impl WorkspaceFilesystemPort for FakeFilesystem {
    fn canonicalize_project(
        &self,
        path: &ProjectPath,
    ) -> Result<String, WorkspaceApplicationError> {
        self.calls
            .lock()
            .expect("calls")
            .push(format!("filesystem:canonicalize:{}", path.as_str()));
        Ok(self.canonical.clone())
    }

    fn sibling_worktree_target(
        &self,
        project_path: &str,
        name: &WorktreeName,
    ) -> Result<String, WorkspaceApplicationError> {
        self.calls.lock().expect("calls").push(format!(
            "filesystem:target:{project_path}:{}",
            name.as_str()
        ));
        Ok(self.target.clone())
    }
}

#[derive(Clone)]
struct FakeGit {
    root: Option<String>,
    calls: Arc<Mutex<Vec<String>>>,
}

impl WorkspaceGitPort for FakeGit {
    fn repository_root(
        &self,
        project_path: &str,
    ) -> Result<Option<String>, WorkspaceApplicationError> {
        self.calls
            .lock()
            .expect("calls")
            .push(format!("git:inspect:{project_path}"));
        Ok(self.root.clone())
    }

    fn create_worktree(
        &self,
        project_path: &str,
        target_path: &str,
        branch: &str,
    ) -> Result<(), WorkspaceApplicationError> {
        self.calls
            .lock()
            .expect("calls")
            .push(format!("git:create:{project_path}:{target_path}:{branch}"));
        Ok(())
    }

    fn validate_loop_worktree(
        &self,
        project_path: &str,
        target_path: &str,
        branch: &str,
        base_branch: &str,
    ) -> Result<(), WorkspaceApplicationError> {
        self.calls.lock().expect("calls").push(format!(
            "git:validate-loop:{project_path}:{target_path}:{branch}:{base_branch}"
        ));
        Ok(())
    }

    fn create_loop_worktree(
        &self,
        project_path: &str,
        target_path: &str,
        branch: &str,
        base_branch: &str,
    ) -> Result<(), WorkspaceApplicationError> {
        self.calls.lock().expect("calls").push(format!(
            "git:create-loop:{project_path}:{target_path}:{branch}:{base_branch}"
        ));
        Ok(())
    }
}

#[derive(Clone)]
struct FakeSelection {
    selected: Option<String>,
    calls: Arc<Mutex<Vec<String>>>,
}

impl ProjectDirectorySelectionPort for FakeSelection {
    fn select_directory(&self) -> Result<Option<String>, WorkspaceApplicationError> {
        self.calls
            .lock()
            .expect("calls")
            .push("selection:open".to_string());
        Ok(self.selected.clone())
    }
}

#[derive(Clone, Copy)]
struct FixedClock;

impl WorkspaceClockPort for FixedClock {
    fn now(&self) -> String {
        "2026-07-18T12:00:00Z".to_string()
    }
}

#[derive(Clone, Default)]
struct FakeSessionQueries {
    calls: Arc<Mutex<Vec<String>>>,
}

impl WorkspaceSessionQueryPort for FakeSessionQueries {
    fn resolve_session_root(
        &self,
        session_id: &str,
    ) -> Result<Option<String>, WorkspaceApplicationError> {
        self.calls
            .lock()
            .expect("calls")
            .push(format!("query:root:{session_id}"));
        Ok(Some("D:/workspace".to_string()))
    }

    fn list_directory(
        &self,
        session_id: &str,
        path: &str,
    ) -> Result<DirectoryListing, WorkspaceApplicationError> {
        self.calls
            .lock()
            .expect("calls")
            .push(format!("query:directory:{session_id}:{path}"));
        Ok(DirectoryListing {
            context: SessionWorkspaceContext::available(Some("app".to_string())),
            path: path.to_string(),
            items: Vec::new(),
            truncated: false,
            next_cursor: None,
        })
    }

    fn list_documents(
        &self,
        session_id: &str,
    ) -> Result<DocumentListing, WorkspaceApplicationError> {
        self.calls
            .lock()
            .expect("calls")
            .push(format!("query:documents:{session_id}"));
        Ok(DocumentListing {
            context: SessionWorkspaceContext::available(Some("app".to_string())),
            items: Vec::new(),
            truncated: false,
            next_cursor: None,
        })
    }

    fn read_file(
        &self,
        session_id: &str,
        path: &str,
    ) -> Result<FileContent, WorkspaceApplicationError> {
        self.calls
            .lock()
            .expect("calls")
            .push(format!("query:file:{session_id}:{path}"));
        Ok(FileContent {
            path: path.to_string(),
            name: "readme.md".to_string(),
            status: "text",
            size: 7,
            content: Some("fixture".to_string()),
        })
    }

    fn read_text_file(
        &self,
        session_id: &str,
        path: &str,
    ) -> Result<FileContent, WorkspaceApplicationError> {
        self.calls
            .lock()
            .expect("calls")
            .push(format!("query:text-file:{session_id}:{path}"));
        self.read_file(session_id, path)
    }

    fn git_status(&self, session_id: &str) -> Result<GitStatusResult, WorkspaceApplicationError> {
        self.calls
            .lock()
            .expect("calls")
            .push(format!("query:git-status:{session_id}"));
        Ok(GitStatusResult {
            context: SessionWorkspaceContext::available(Some("app".to_string())),
            is_git: true,
            branch: Some("main".to_string()),
            items: Vec::new(),
            truncated: false,
            next_cursor: None,
        })
    }

    fn git_diff(
        &self,
        session_id: &str,
        path: &str,
        source: GitDiffSource,
    ) -> Result<GitDiffResult, WorkspaceApplicationError> {
        self.calls
            .lock()
            .expect("calls")
            .push(format!("query:git-diff:{session_id}:{path}:{source:?}"));
        Ok(GitDiffResult {
            context: SessionWorkspaceContext::available(Some("app".to_string())),
            source,
            files: Vec::new(),
            truncated: false,
        })
    }

    fn list_logs(
        &self,
        query: &SessionLogQuery,
    ) -> Result<SessionLogPage, WorkspaceApplicationError> {
        self.calls
            .lock()
            .expect("calls")
            .push(format!("query:logs:{}:{}", query.session_id, query.search));
        Ok(SessionLogPage {
            items: Vec::new(),
            truncated: false,
            next_cursor: None,
        })
    }

    fn export_logs(
        &self,
        query: &SessionLogQuery,
    ) -> Result<SessionLogExportResult, WorkspaceApplicationError> {
        self.calls
            .lock()
            .expect("calls")
            .push(format!("query:export:{}", query.session_id));
        Ok(SessionLogExportResult {
            status: "cancelled",
            path: None,
        })
    }
}

fn service(calls: Arc<Mutex<Vec<String>>>) -> (WorkspaceApplicationService, FakeHistory) {
    let history = FakeHistory {
        calls: calls.clone(),
        ..FakeHistory::default()
    };
    let filesystem = FakeFilesystem {
        canonical: "C:\\code\\app".to_string(),
        target: "C:\\code\\app-feature-a".to_string(),
        calls: calls.clone(),
    };
    let git = FakeGit {
        root: Some("C:\\code\\app".to_string()),
        calls: calls.clone(),
    };
    let selection = FakeSelection {
        selected: Some("C:\\code\\selected".to_string()),
        calls,
    };
    (
        WorkspaceApplicationService::new(
            Arc::new(history.clone()),
            Arc::new(filesystem),
            Arc::new(git),
            Arc::new(selection),
            Arc::new(FixedClock),
        ),
        history,
    )
}

#[test]
fn project_inspection_validates_then_coordinates_filesystem_and_git() {
    let calls = Arc::new(Mutex::new(Vec::new()));
    let (service, _) = service(calls.clone());

    let inspection = service
        .inspect_project("  C:\\code\\app  ")
        .expect("inspection");

    assert_eq!(inspection.path(), "C:\\code\\app");
    assert_eq!(inspection.display_name(), "app");
    assert!(inspection.is_git());
    assert_eq!(
        *calls.lock().expect("calls"),
        vec![
            "filesystem:canonicalize:C:\\code\\app",
            "git:inspect:C:\\code\\app"
        ]
    );
}

#[test]
fn invalid_project_path_stops_before_external_ports() {
    let calls = Arc::new(Mutex::new(Vec::new()));
    let (service, _) = service(calls.clone());

    assert_eq!(
        service.inspect_project("  "),
        Err(WorkspaceApplicationError::Domain(
            crate::contexts::workspaces::domain::WorkspaceDomainError::ProjectPathRequired
        ))
    );
    assert!(calls.lock().expect("calls").is_empty());
}

#[test]
fn history_queries_and_records_use_the_repository_and_injected_clock() {
    let calls = Arc::new(Mutex::new(Vec::new()));
    let (service, history) = service(calls.clone());
    history
        .state
        .lock()
        .expect("history")
        .projects
        .push(KnownProject {
            path: "C:\\code\\app".to_string(),
            display_name: "app".to_string(),
            is_git: true,
            last_opened_at: "2026-07-18T11:00:00Z".to_string(),
        });
    let inspection =
        ProjectInspection::from_probe("C:\\code\\app", Some("C:\\code\\app".to_string()))
            .expect("inspection");
    let remote = RemoteWorkspace::new("example.com", None, Some("dev"), "/work/app", None)
        .expect("remote workspace");

    assert_eq!(service.list_known_projects().expect("projects").len(), 1);
    assert!(service
        .list_known_remote_workspaces()
        .expect("remote workspaces")
        .is_empty());
    service
        .remember_project(&inspection)
        .expect("remember project");
    service
        .remember_remote_workspace(&remote)
        .expect("remember remote");

    assert_eq!(
        *calls.lock().expect("calls"),
        vec![
            "history:list-projects",
            "history:list-remote",
            "history:remember-project:C:\\code\\app:2026-07-18T12:00:00Z",
            "history:remember-remote:ssh://dev@example.com/work/app:2026-07-18T12:00:00Z",
        ]
    );
}

#[test]
fn worktree_creation_validates_and_orders_target_before_explicit_git_effect() {
    let calls = Arc::new(Mutex::new(Vec::new()));
    let (service, _) = service(calls.clone());

    let created = service
        .create_worktree("C:\\code\\app", " feature-a ")
        .expect("worktree");

    assert_eq!(created.path, "C:\\code\\app-feature-a");
    assert_eq!(created.name, "feature-a");
    assert_eq!(created.branch, "vanehub/feature-a");
    assert_eq!(
        *calls.lock().expect("calls"),
        vec![
            "filesystem:target:C:\\code\\app:feature-a",
            "git:create:C:\\code\\app:C:\\code\\app-feature-a:vanehub/feature-a",
        ]
    );

    calls.lock().expect("calls").clear();
    assert!(service.create_worktree("C:\\code\\app", "../bad").is_err());
    assert!(calls.lock().expect("calls").is_empty());
}

#[test]
fn loop_worktree_is_canonicalized_and_guarded_before_creation() {
    let calls = Arc::new(Mutex::new(Vec::new()));
    let (service, _) = service(calls.clone());

    let created = service
        .create_guarded_loop_worktree(" C:\\code\\app ", "loop-42", "origin/main")
        .expect("Loop worktree");

    assert_eq!(created.branch, "vanehub/loop-42");
    assert_eq!(
        *calls.lock().expect("calls"),
        vec![
            "filesystem:canonicalize:C:\\code\\app",
            "git:inspect:C:\\code\\app",
            "filesystem:target:C:\\code\\app:loop-42",
            "git:validate-loop:C:\\code\\app:C:\\code\\app-feature-a:vanehub/loop-42:origin/main",
            "git:create-loop:C:\\code\\app:C:\\code\\app-feature-a:vanehub/loop-42:origin/main",
        ]
    );
}

#[test]
fn directory_selection_is_one_port_delegation() {
    let calls = Arc::new(Mutex::new(Vec::new()));
    let (service, _) = service(calls.clone());

    assert_eq!(
        service
            .select_project_directory()
            .expect("selection")
            .as_deref(),
        Some("C:\\code\\selected")
    );
    assert_eq!(*calls.lock().expect("calls"), vec!["selection:open"]);
}

#[test]
fn bounded_workspace_queries_delegate_only_through_the_injected_port() {
    let calls = Arc::new(Mutex::new(Vec::new()));
    let service = WorkspaceQueryApplicationService::new(Arc::new(FakeSessionQueries {
        calls: calls.clone(),
    }));
    let query = SessionLogQuery {
        session_id: "session-1".to_string(),
        levels: vec![WorkspaceLogLevel::Info],
        search: "ready".to_string(),
        cursor: None,
        limit: Some(25),
    };

    assert_eq!(
        service
            .list_directory("session-1", "src")
            .expect("directory")
            .path,
        "src"
    );
    service.list_documents("session-1").expect("documents");
    assert_eq!(
        service
            .read_file("session-1", "readme.md")
            .expect("file")
            .status,
        "text"
    );
    service
        .read_text_file("session-1", "readme.md")
        .expect("text file");
    assert!(service.git_status("session-1").expect("status").is_git);
    assert_eq!(
        service
            .git_diff("session-1", "src/lib.rs", GitDiffSource::Staged)
            .expect("diff")
            .source,
        GitDiffSource::Staged
    );
    service.list_logs(&query).expect("logs");
    assert_eq!(
        service.export_logs(&query).expect("export").status,
        "cancelled"
    );

    assert_eq!(
        *calls.lock().expect("calls"),
        vec![
            "query:directory:session-1:src",
            "query:documents:session-1",
            "query:file:session-1:readme.md",
            "query:text-file:session-1:readme.md",
            "query:file:session-1:readme.md",
            "query:git-status:session-1",
            "query:git-diff:session-1:src/lib.rs:Staged",
            "query:logs:session-1:ready",
            "query:export:session-1",
        ]
    );
}
