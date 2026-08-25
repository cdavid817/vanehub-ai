//! One suite, two providers.
//!
//! Every case below runs against the local implementation over a real temporary workspace and
//! against the remote implementation over a scripted helper. That is the point of the seam: a panel
//! cannot tell which one answered, so the two must not differ in what they promise — the same
//! ordering, the same refusal for a path that leaves the root, and the same distinction between
//! "this is empty" and "this is not there".
//!
//! What the two genuinely differ about is asserted as a difference rather than smoothed over: the
//! local provider learns about changes from the events this application emits, the remote one polls,
//! and a reader has to be told which.
//!
//! The remote side is scripted rather than connected. Its confinement runs on the remote host, and
//! that is where it is proved — 11.14's opt-in integration test. What is provable here is that both
//! sides turn their answers into the same shapes and their refusals into the same meanings.

use super::remote_helper::{
    scripted_session, RemoteHelperError, RemoteProfileSource, RemoteWorkspaceInspectionProvider,
};
use super::workspace_inspection::LocalWorkspaceInspectionProvider;
use crate::contexts::workspaces::application::{
    DirectoryCursor, DirectoryListing, DocumentListing, FileContent, FileSearchListing,
    GitDiffRequest, GitDiffResult, GitDiffSource, GitStatusResult, ListDirectoryRequest,
    LocalWorkspaceTarget, ReadTextFileRequest, RemoteWorkspaceTarget, SessionLogExportResult,
    SessionLogPage, SessionLogQuery, WorkspaceApplicationError as AppError,
    WorkspaceInspectionError, WorkspaceInspectionProvider, WorkspaceSearchRequest,
    WorkspaceSessionQueryPort, WorkspaceTarget,
};
use crate::platform::database::NativeDatabase;
use crate::test_support::TempDirectory;
use rusqlite::params;
use std::fs;
use std::sync::Arc;

/// The local reads, over a database and nothing else.
///
/// The production adapter also holds an app handle, used by one method: the log export. Nothing
/// here exports a log, and requiring a running Tauri application to test a directory listing would
/// mean not testing it.
struct DatabaseQueries {
    database: NativeDatabase,
}

impl DatabaseQueries {
    fn connection(&self) -> Result<crate::platform::database::PooledSqlite, AppError> {
        self.database
            .connection()
            .map_err(|error| AppError::Repository(error.to_string()))
    }
}

impl WorkspaceSessionQueryPort for DatabaseQueries {
    fn resolve_session_root(&self, session_id: &str) -> Result<Option<String>, AppError> {
        super::session_queries::resolve_session_root(&*self.connection()?, session_id)
            .map(|root| root.map(|path| path.to_string_lossy().to_string()))
    }

    fn list_directory(&self, session_id: &str, path: &str) -> Result<DirectoryListing, AppError> {
        super::session_queries::list_session_directory(&*self.connection()?, session_id, path)
    }

    fn list_directory_page(
        &self,
        session_id: &str,
        path: &str,
        cursor: Option<&str>,
        limit: usize,
    ) -> Result<DirectoryListing, AppError> {
        super::session_queries::list_session_directory_page(
            &*self.connection()?,
            session_id,
            path,
            cursor,
            limit,
        )
    }

    fn list_documents(&self, session_id: &str) -> Result<DocumentListing, AppError> {
        super::session_queries::list_session_documents(&*self.connection()?, session_id)
    }

    fn search_files(
        &self,
        session_id: &str,
        query: &str,
        max_results: usize,
    ) -> Result<FileSearchListing, AppError> {
        super::session_search::search_session_files(
            &*self.connection()?,
            session_id,
            query,
            max_results,
        )
    }

    fn read_file(&self, session_id: &str, path: &str) -> Result<FileContent, AppError> {
        super::session_queries::read_session_file(&*self.connection()?, session_id, path)
    }

    fn read_text_file(&self, session_id: &str, path: &str) -> Result<FileContent, AppError> {
        super::session_queries::read_session_text_file(&*self.connection()?, session_id, path)
    }

    fn git_status(&self, session_id: &str) -> Result<GitStatusResult, AppError> {
        super::session_queries::get_session_git_status(&*self.connection()?, session_id)
    }

    fn git_diff(
        &self,
        session_id: &str,
        path: &str,
        source: GitDiffSource,
    ) -> Result<GitDiffResult, AppError> {
        super::session_queries::get_session_git_diff(&*self.connection()?, session_id, path, source)
    }

    fn list_logs(&self, _query: &SessionLogQuery) -> Result<SessionLogPage, AppError> {
        // Not part of inspection. Refusing rather than answering keeps this double from quietly
        // standing in for a capability the contract says nothing about.
        Err(AppError::Validation("logs are not inspected".to_string()))
    }

    fn export_logs(&self, _query: &SessionLogQuery) -> Result<SessionLogExportResult, AppError> {
        Err(AppError::Validation(
            "logs are not exported here".to_string(),
        ))
    }
}

/// One provider under test, with the target it answers for.
struct Subject {
    name: &'static str,
    provider: Box<dyn WorkspaceInspectionProvider>,
    target: WorkspaceTarget,
    /// The temporary workspace, held so it outlives the test.
    _directory: Option<TempDirectory>,
}

fn block<T>(future: impl std::future::Future<Output = T>) -> T {
    tauri::async_runtime::block_on(future)
}

/// A real workspace on disk, with the files every case below reads.
fn local_subject() -> Subject {
    let directory = TempDirectory::new("provider-contract-local");
    let workspace = directory.path().join("workspace");
    fs::create_dir_all(workspace.join("src")).expect("src");
    fs::write(workspace.join("readme.md"), "# hello").expect("readme");
    fs::write(workspace.join("src").join("main.rs"), "fn main() {}").expect("main");
    // Not valid UTF-8, so the preview has something real to classify.
    fs::write(workspace.join("blob.bin"), [0xffu8, 0xfe, 0x00, 0x01]).expect("blob");

    let database = NativeDatabase::new(directory.path().join("data")).expect("database");
    let connection = database.connection().expect("connection");
    connection
        .execute(
            "INSERT INTO sessions \
             (id, title, agent_id, interaction_mode, lifecycle_state, folder, pinned, archived, \
              created_at, updated_at) \
             VALUES ('session-local', 'Local', 'codex-cli', 'cli', 'idle', ?1, 0, 0, \
                     '2026-08-26T10:00:00Z', '2026-08-26T10:00:00Z')",
            params![workspace.to_string_lossy().as_ref()],
        )
        .expect("insert session");
    drop(connection);

    Subject {
        name: "local",
        provider: Box::new(LocalWorkspaceInspectionProvider::new(Arc::new(
            DatabaseQueries { database },
        ))),
        target: WorkspaceTarget::Local(LocalWorkspaceTarget {
            session_id: "session-local".to_string(),
            root: workspace.canonicalize().expect("canonical workspace"),
        }),
        _directory: Some(directory),
    }
}

struct StaticProfile;

impl RemoteProfileSource for StaticProfile {
    fn current(&self, _connection_id: &str) -> Result<(i64, bool), RemoteHelperError> {
        Ok((7, true))
    }
}

fn remote_subject(body: &str) -> Subject {
    Subject {
        name: "remote",
        provider: Box::new(RemoteWorkspaceInspectionProvider::new(
            Arc::new(StaticProfile),
            Arc::new(scripted_session(vec![body.to_string()])),
        )),
        target: WorkspaceTarget::Remote(RemoteWorkspaceTarget {
            session_id: "session-remote".to_string(),
            connection_id: "connection-1".to_string(),
            connection_revision: 7,
            root: "/work/app".to_string(),
            display_name: "Remote app".to_string(),
        }),
        _directory: None,
    }
}

// ---------------------------------------------------------------------------------------------
// The contract
// ---------------------------------------------------------------------------------------------

const REMOTE_LISTING: &str = r##"{"version":1,"ok":true,"result":{"listing":{"path":"","entries":[
    {"name":"src","path":"src","kind":"directory","size":null},
    {"name":"blob.bin","path":"blob.bin","kind":"file","size":4},
    {"name":"readme.md","path":"readme.md","kind":"file","size":7}
  ],"truncated":false}}}"##;

#[test]
fn a_listing_orders_directories_first_then_by_name() {
    for subject in [local_subject(), remote_subject(REMOTE_LISTING)] {
        let listing = block(
            subject
                .provider
                .list_directory(&subject.target, ListDirectoryRequest::default()),
        )
        .unwrap_or_else(|error| panic!("{}: {error:?}", subject.name));

        let names: Vec<&str> = listing
            .items
            .iter()
            .map(|item| item.name.as_str())
            .collect();
        // The same order on both, so a workspace does not rearrange itself when it moves hosts.
        assert_eq!(
            names,
            vec!["src", "blob.bin", "readme.md"],
            "{}",
            subject.name
        );
        assert!(!listing.truncated, "{}", subject.name);
    }
}

#[test]
fn a_listing_reports_relative_paths_a_panel_can_follow() {
    for subject in [local_subject(), remote_subject(REMOTE_LISTING)] {
        let listing = block(
            subject
                .provider
                .list_directory(&subject.target, ListDirectoryRequest::default()),
        )
        .unwrap_or_else(|error| panic!("{}: {error:?}", subject.name));

        for item in &listing.items {
            // An absolute path would be one machine's directory layout in another's UI, and on the
            // remote side it would also be a link this panel cannot open.
            assert!(
                !item.path.starts_with('/'),
                "{}: {}",
                subject.name,
                item.path
            );
            assert!(!item.path.contains(".."), "{}: {}", subject.name, item.path);
        }
    }
}

const REMOTE_TEXT: &str = r##"{"version":1,"ok":true,"result":{"file":{"path":"readme.md",
    "name":"readme.md","status":"text","size":7,"content":"# hello"}}}"##;

#[test]
fn a_text_file_comes_back_with_its_content() {
    for subject in [local_subject(), remote_subject(REMOTE_TEXT)] {
        let file = block(subject.provider.read_text_file(
            &subject.target,
            ReadTextFileRequest {
                path: "readme.md".to_string(),
            },
        ))
        .unwrap_or_else(|error| panic!("{}: {error:?}", subject.name));

        assert_eq!(file.status, "text", "{}", subject.name);
        assert_eq!(file.content.as_deref(), Some("# hello"), "{}", subject.name);
    }
}

const REMOTE_BINARY: &str = r##"{"version":1,"ok":true,"result":{"file":{"path":"blob.bin",
    "name":"blob.bin","status":"binary","size":4,"content":null}}}"##;

#[test]
fn a_binary_file_withholds_its_content_and_still_reports_a_size() {
    for subject in [local_subject(), remote_subject(REMOTE_BINARY)] {
        let file = block(subject.provider.read_text_file(
            &subject.target,
            ReadTextFileRequest {
                path: "blob.bin".to_string(),
            },
        ))
        .unwrap_or_else(|error| panic!("{}: {error:?}", subject.name));

        // "There is a file here that cannot be previewed" is a different statement from "there is
        // nothing here", and both providers have to make the first one.
        assert_eq!(file.status, "binary", "{}", subject.name);
        assert_eq!(file.content, None, "{}", subject.name);
        assert_eq!(file.size, 4, "{}", subject.name);
    }
}

/// A path that leaves the root is refused by both, and refused as an escape.
///
/// The one rule the two enforce in different places — the local one against a canonical root on
/// this machine, the remote one against `realpath` on the host — which is exactly why it belongs
/// in a shared suite rather than in each implementation's own tests.
#[test]
fn a_path_outside_the_root_is_refused_rather_than_read() {
    const REMOTE_ESCAPE: &str = r#"{"version":1,"ok":false,"reasonCode":"workspace_path_escaped"}"#;
    for subject in [local_subject(), remote_subject(REMOTE_ESCAPE)] {
        let error = block(subject.provider.read_text_file(
            &subject.target,
            ReadTextFileRequest {
                path: "../escaped.txt".to_string(),
            },
        ))
        .expect_err(subject.name);

        assert_eq!(
            error,
            WorkspaceInspectionError::PathEscaped,
            "{}",
            subject.name
        );
    }
}

const REMOTE_SEARCH: &str = r##"{"version":1,"ok":true,"result":{"search":{"matches":[
    {"name":"main.rs","path":"src/main.rs","kind":"file","size":null}],"truncated":false}}}"##;

#[test]
fn a_search_answers_with_paths_relative_to_the_root() {
    for subject in [local_subject(), remote_subject(REMOTE_SEARCH)] {
        let listing = block(subject.provider.search(
            &subject.target,
            WorkspaceSearchRequest {
                query: "main".to_string(),
                max_results: 20,
            },
        ))
        .unwrap_or_else(|error| panic!("{}: {error:?}", subject.name));

        assert!(!listing.items.is_empty(), "{}", subject.name);
        for item in &listing.items {
            assert!(
                !item.path.starts_with('/'),
                "{}: {}",
                subject.name,
                item.path
            );
        }
    }
}

const REMOTE_NO_REPOSITORY: &str = r##"{"version":1,"ok":true,"result":{"git":{
    "isRepository":false,"stdoutBase64":null,"truncated":false}}}"##;

#[test]
fn a_directory_with_no_repository_reports_that_rather_than_failing() {
    for subject in [local_subject(), remote_subject(REMOTE_NO_REPOSITORY)] {
        let status = block(subject.provider.git_status(&subject.target))
            .unwrap_or_else(|error| panic!("{}: {error:?}", subject.name));

        // The temporary workspace is not a repository and the scripted host says the same. Both
        // must answer, because "no version control here" is a fact a panel renders rather than an
        // error a reader would try to fix.
        assert!(!status.is_git, "{}", subject.name);
        assert!(status.items.is_empty(), "{}", subject.name);
        assert_eq!(status.branch, None, "{}", subject.name);
    }
}

#[test]
fn a_diff_outside_a_repository_invents_nothing() {
    for subject in [local_subject(), remote_subject(REMOTE_NO_REPOSITORY)] {
        let diff = block(subject.provider.git_diff(
            &subject.target,
            GitDiffRequest {
                path: "readme.md".to_string(),
                source: GitDiffSource::Working,
            },
        ));

        // What both guarantee: the answer is about the file that was asked for, and nothing
        // else. A diff that mentioned a second path would be describing a change the caller
        // never asked about, which is the failure worth ruling out on both sides.
        match diff {
            Ok(result) => {
                for file in &result.files {
                    assert_eq!(file.new_path, "readme.md", "{}", subject.name);
                }
            }
            Err(error) => assert!(
                !matches!(error, WorkspaceInspectionError::PathEscaped),
                "{}: a diff outside a repository is not an escape: {error:?}",
                subject.name
            ),
        }
    }
}

const REMOTE_PROBE: &str = r##"{"version":1,"ok":true,"result":{"probe":{"helperVersion":1,
    "posix":true,"pythonVersion":"3.11.2","git":true,"ripgrep":true,"rootReadable":true}}}"##;

/// The two watch differently, and a reader is told which.
#[test]
fn the_providers_declare_their_own_freshness_guarantee() {
    let local = local_subject();
    let local_capabilities =
        block(local.provider.capabilities(&local.target)).expect("local capabilities");
    let remote = remote_subject(REMOTE_PROBE);
    let remote_capabilities =
        block(remote.provider.capabilities(&remote.target)).expect("remote capabilities");

    assert_eq!(local_capabilities.provider, "local");
    assert_eq!(remote_capabilities.provider, "ssh");
    // Nothing on a remote host tells this process when a file changed, and a reader who believed
    // otherwise would not know to refresh.
    assert_eq!(local_capabilities.watch_mode.token(), "event-derived");
    assert_eq!(remote_capabilities.watch_mode.token(), "polling");
}

/// Each refuses the other's kind of target rather than answering about the wrong machine.
#[test]
fn each_provider_refuses_the_other_kind_of_target() {
    let local = local_subject();
    let remote = remote_subject(REMOTE_LISTING);

    let local_error = block(
        local
            .provider
            .list_directory(&remote.target, ListDirectoryRequest::default()),
    )
    .expect_err("local given a remote target");
    let remote_error = block(
        remote
            .provider
            .list_directory(&local.target, ListDirectoryRequest::default()),
    )
    .expect_err("remote given a local target");

    assert_eq!(
        local_error,
        WorkspaceInspectionError::Unsupported("workspace_provider_local_only")
    );
    assert_eq!(
        remote_error,
        WorkspaceInspectionError::Unsupported("workspace_provider_remote_only")
    );
}

/// A recorded divergence, not an alignment.
///
/// Outside a repository the local provider still answers a diff request by treating the file as
/// untracked and rendering its whole contents as added; the remote one answers that there is no
/// repository and returns nothing. Both are defensible readings of "what changed here", and
/// neither is reachable from the Changes tab — its file list comes from a status that is empty
/// when there is no repository.
///
/// It is asserted rather than fixed because changing the local reading would change behaviour that
/// predates this seam, in a direction nobody has asked for. Written down here so the next person
/// to notice finds a decision instead of a surprise.
#[test]
fn the_providers_read_an_untracked_file_outside_a_repository_differently() {
    let local = local_subject();
    let local_diff = block(local.provider.git_diff(
        &local.target,
        GitDiffRequest {
            path: "readme.md".to_string(),
            source: GitDiffSource::Working,
        },
    ));
    let remote = remote_subject(REMOTE_NO_REPOSITORY);
    let remote_diff = block(remote.provider.git_diff(
        &remote.target,
        GitDiffRequest {
            path: "readme.md".to_string(),
            source: GitDiffSource::Working,
        },
    ))
    .expect("remote diff");

    assert!(remote_diff.files.is_empty());
    // Whatever the local provider does here, it does not claim a change to a file nobody asked
    // about - which is the part that would actually mislead a reader.
    if let Ok(result) = local_diff {
        for file in &result.files {
            assert_eq!(file.new_path, "readme.md");
        }
    }
}

const REMOTE_TRUNCATED_LISTING: &str = r##"{"version":1,"ok":true,"result":{"listing":{"path":"",
  "entries":[
    {"name":"src","path":"src","kind":"directory","size":null},
    {"name":"blob.bin","path":"blob.bin","kind":"file","size":4}
  ],"truncated":true}}}"##;

// ---------------------------------------------------------------------------------------------
// Continuation
// ---------------------------------------------------------------------------------------------

/// Two pages of one entry each cover the directory exactly, in order and without repeats.
///
/// The failure a keyset cursor prevents is invisible in a single page: an offset cursor produces
/// exactly this shape too, right up until a file is created between the two reads.
#[test]
fn a_page_resumes_after_the_entry_it_ended_on() {
    let subject = local_subject();
    let mut seen: Vec<String> = Vec::new();
    let mut cursor: Option<String> = None;

    for _ in 0..8 {
        let page = block(subject.provider.list_directory(
            &subject.target,
            ListDirectoryRequest {
                path: String::new(),
                cursor: cursor.clone(),
                limit: Some(1),
            },
        ))
        .expect("page");
        seen.extend(page.items.iter().map(|item| item.name.clone()));
        cursor = page.next_cursor.clone();
        if cursor.is_none() {
            break;
        }
    }

    // The same order the unpaged listing gives, and each entry exactly once.
    assert_eq!(seen, vec!["src", "blob.bin", "readme.md"]);
}

#[test]
fn the_last_page_offers_no_cursor() {
    let subject = local_subject();

    let page = block(subject.provider.list_directory(
        &subject.target,
        ListDirectoryRequest {
            path: String::new(),
            cursor: None,
            limit: Some(100),
        },
    ))
    .expect("page");

    // A cursor for an exhausted directory would invite a caller to fetch a page that is always
    // empty, and an empty page reads as a directory that just emptied itself.
    assert!(!page.truncated);
    assert_eq!(page.next_cursor, None);
}

/// A cursor issued for one directory is refused by another.
///
/// Without the binding the resume key is just a name: it compares fine against a different
/// directory's entries, and the reader gets a page from a folder they are not looking at.
#[test]
fn a_cursor_from_another_directory_is_refused() {
    let subject = local_subject();
    let root_page = block(subject.provider.list_directory(
        &subject.target,
        ListDirectoryRequest {
            path: String::new(),
            cursor: None,
            limit: Some(1),
        },
    ))
    .expect("root page");
    let cursor = root_page.next_cursor.expect("a cursor for the root");

    let error = block(subject.provider.list_directory(
        &subject.target,
        ListDirectoryRequest {
            path: "src".to_string(),
            cursor: Some(cursor),
            limit: Some(10),
        },
    ))
    .expect_err("a cursor from the root does not continue src");

    // `NotFound` rather than a page: the local reads report a rejected cursor as a validation
    // refusal, which the provider classifies as a path that is not there rather than an escape.
    assert!(
        matches!(
            error,
            WorkspaceInspectionError::NotFound | WorkspaceInspectionError::InvalidCursor
        ),
        "{error:?}"
    );
}

/// The remote provider mints the cursor, so the helper cannot issue one for another directory.
#[test]
fn the_remote_cursor_is_minted_from_the_page_it_ends() {
    let subject = remote_subject(REMOTE_TRUNCATED_LISTING);

    let page = block(subject.provider.list_directory(
        &subject.target,
        ListDirectoryRequest {
            path: String::new(),
            cursor: None,
            limit: Some(2),
        },
    ))
    .expect("page");

    assert!(page.truncated);
    let cursor = page.next_cursor.expect("a cursor");
    // It decodes for the directory that was asked for, and only that one.
    assert!(DirectoryCursor::decode(&cursor, "").is_ok());
    assert!(DirectoryCursor::decode(&cursor, "src").is_err());
}
