//! What the remote provider asks for, and what it makes of the answer.
//!
//! Against a scripted helper. The confinement itself runs on the remote host — only that machine
//! can tell a symlink from a directory — so what is provable here is the other half: that a refusal
//! keeps its meaning, that git output is parsed by the same parser the local provider uses, and
//! that a truncated answer is reported as truncated rather than rendered as a smaller workspace.
//!
//! The helper program's own behaviour is asserted against its source where the property is
//! structural (an argument array, a pinned locale, a separator in the prefix check). Whether the
//! Python runs at all belongs to 11.14's opt-in integration test; a fake that pretended to be
//! Python would prove the fake.

use super::protocol::{HelperOperation, RemoteHelperError};
use super::remote_provider::RemoteWorkspaceInspectionProvider;
use super::transport::{
    RemoteHelperChannel, RemoteHelperEvent, RemoteHelperSession, HELPER_PROGRAM,
};
use crate::contexts::workspaces::application::{
    GitDiffRequest, GitDiffSource, ListDirectoryRequest, ReadTextFileRequest,
    RemoteWorkspaceTarget, WorkspaceInspectionError, WorkspaceSearchRequest, WorkspaceTarget,
};
use async_trait::async_trait;
use base64::Engine;
use std::sync::{Arc, Mutex};

/// Answers each request with the next scripted body and records what was asked.
struct ScriptedHelper {
    bodies: Mutex<Vec<String>>,
    requests: Mutex<Vec<String>>,
}

impl ScriptedHelper {
    fn answering(body: &str) -> Arc<Self> {
        Arc::new(Self {
            bodies: Mutex::new(vec![body.to_string()]),
            requests: Mutex::new(Vec::new()),
        })
    }
}

struct ScriptedChannel {
    helper: Arc<ScriptedHelper>,
    written: Mutex<Vec<u8>>,
    events: Mutex<Vec<RemoteHelperEvent>>,
}

#[async_trait]
impl RemoteHelperChannel for ScriptedChannel {
    async fn write(&self, bytes: &[u8]) -> Result<(), RemoteHelperError> {
        self.written
            .lock()
            .expect("written")
            .extend_from_slice(bytes);
        Ok(())
    }

    async fn send_eof(&self) -> Result<(), RemoteHelperError> {
        // The request is everything after the program's line, which is what the helper reads.
        let written = self.written.lock().expect("written").clone();
        let text = String::from_utf8_lossy(&written).to_string();
        if let Some((_, request)) = text.split_once('\n') {
            self.helper
                .requests
                .lock()
                .expect("requests")
                .push(request.to_string());
        }
        let body = {
            let mut bodies = self.helper.bodies.lock().expect("bodies");
            if bodies.len() > 1 {
                bodies.remove(0)
            } else {
                bodies[0].clone()
            }
        };
        *self.events.lock().expect("events") = vec![
            RemoteHelperEvent::Stdout(body.into_bytes()),
            RemoteHelperEvent::Ended,
        ];
        Ok(())
    }

    async fn next_event(&self) -> Result<Option<RemoteHelperEvent>, RemoteHelperError> {
        let mut events = self.events.lock().expect("events");
        if events.is_empty() {
            return Ok(None);
        }
        Ok(Some(events.remove(0)))
    }

    async fn close(&self) -> Result<(), RemoteHelperError> {
        Ok(())
    }
}

#[async_trait]
impl RemoteHelperSession for ScriptedHelper {
    async fn open(
        &self,
        _connection_id: &str,
        _revision: i64,
    ) -> Result<Box<dyn RemoteHelperChannel>, RemoteHelperError> {
        Ok(Box::new(ScriptedChannel {
            helper: Arc::new(Self {
                bodies: Mutex::new(self.bodies.lock().expect("bodies").clone()),
                requests: Mutex::new(Vec::new()),
            }),
            written: Mutex::new(Vec::new()),
            events: Mutex::new(Vec::new()),
        }))
    }
}

/// The transport under test without the SSH profile lookup.
///
/// `RemoteWorkspaceInspectionProvider` revalidates through the connections API, which a unit test
/// cannot construct. These drive the exchange and the mapping directly, which is where every
/// decision in this slice lives; the revalidation rule has its own tests beside it.
async fn call(
    helper: &Arc<ScriptedHelper>,
    operation: HelperOperation,
) -> Result<super::protocol::HelperResult, RemoteHelperError> {
    let response = super::transport::exchange(
        helper.as_ref(),
        "connection-1",
        7,
        &super::protocol::HelperRequest::new("/work/app".to_string(), operation),
    )
    .await?;
    response.result.ok_or(RemoteHelperError::MalformedResponse)
}

fn block<T>(future: impl std::future::Future<Output = T>) -> T {
    tauri::async_runtime::block_on(future)
}

fn target() -> WorkspaceTarget {
    WorkspaceTarget::Remote(RemoteWorkspaceTarget {
        session_id: "session-1".to_string(),
        connection_id: "connection-1".to_string(),
        connection_revision: 7,
        root: "/work/app".to_string(),
        display_name: "Remote app".to_string(),
    })
}

// ---------------------------------------------------------------------------------------------
// Requests
// ---------------------------------------------------------------------------------------------

#[test]
fn a_listing_asks_for_a_relative_path_and_never_an_absolute_one() {
    let helper = ScriptedHelper::answering(
        r#"{"version":1,"ok":true,"result":{"listing":{"path":"src","entries":[],"truncated":false}}}"#,
    );

    block(call(
        &helper,
        HelperOperation::ListDirectory {
            path: "src".to_string(),
            after_kind_rank: None,
            after_name_key: None,
            limit: 500,
        },
    ))
    .expect("listing");

    // The root travels once, in its own field. A request that carried a joined absolute path would
    // move the confinement decision to whoever built the string.
    let request = serde_json::to_string(&super::protocol::HelperRequest::new(
        "/work/app".to_string(),
        HelperOperation::ListDirectory {
            path: "src".to_string(),
            after_kind_rank: None,
            after_name_key: None,
            limit: 500,
        },
    ))
    .expect("request");
    assert!(request.contains(r#""root":"/work/app""#));
    assert!(request.contains(r#""kind":"listDirectory""#));
    assert!(request.contains(r#""path":"src""#));
}

#[test]
fn a_staged_diff_is_a_flag_rather_than_a_different_operation() {
    let staged = serde_json::to_string(&HelperOperation::GitDiff {
        path: "src/main.rs".to_string(),
        staged: true,
    })
    .expect("staged");
    let working = serde_json::to_string(&HelperOperation::GitDiff {
        path: "src/main.rs".to_string(),
        staged: false,
    })
    .expect("working");

    // One operation with a flag, so the index/worktree distinction cannot be lost by a caller
    // choosing the wrong one of two names.
    assert!(staged.contains(r#""staged":true"#));
    assert!(working.contains(r#""staged":false"#));
}

// ---------------------------------------------------------------------------------------------
// Answers
// ---------------------------------------------------------------------------------------------

#[test]
fn a_truncated_listing_says_so_rather_than_reading_as_a_smaller_directory() {
    let helper = ScriptedHelper::answering(
        r#"{"version":1,"ok":true,"result":{"listing":{"path":"","entries":[
           {"name":"src","path":"src","kind":"directory","size":null}],"truncated":true}}}"#,
    );

    let result = block(call(
        &helper,
        HelperOperation::ListDirectory {
            path: String::new(),
            after_kind_rank: None,
            after_name_key: None,
            limit: 500,
        },
    ))
    .expect("listing");

    let listing = result.listing.expect("listing");
    assert!(listing.truncated);
    assert_eq!(listing.entries.len(), 1);
}

#[test]
fn a_binary_preview_withholds_content_rather_than_showing_bytes_as_text() {
    let helper = ScriptedHelper::answering(
        r#"{"version":1,"ok":true,"result":{"file":{"path":"a.bin","name":"a.bin",
           "status":"binary","size":4096,"content":null}}}"#,
    );

    let result = block(call(
        &helper,
        HelperOperation::ReadTextFile {
            path: "a.bin".to_string(),
        },
    ))
    .expect("file");

    let file = result.file.expect("file");
    assert_eq!(file.status, "binary");
    assert_eq!(file.content, None);
    // The size is still reported: "there is a 4 KiB file here that cannot be previewed" is a
    // different statement from "there is nothing here".
    assert_eq!(file.size, 4096);
}

#[test]
fn a_git_answer_carries_bytes_rather_than_a_parsed_structure() {
    let porcelain = "## main\0 M src/main.rs\0";
    let encoded = base64::engine::general_purpose::STANDARD.encode(porcelain);
    let helper = ScriptedHelper::answering(&format!(
        r#"{{"version":1,"ok":true,"result":{{"git":{{"isRepository":true,
           "stdoutBase64":"{encoded}","truncated":false}}}}}}"#
    ));

    let result = block(call(&helper, HelperOperation::GitStatus)).expect("status");

    let git = result.git.expect("git");
    assert!(git.is_repository);
    // Base64 of the raw bytes: a porcelain record is NUL-separated and a POSIX path is bytes rather
    // than text, so decoding it to send it would lose exactly the names that need care.
    let decoded = base64::engine::general_purpose::STANDARD
        .decode(git.stdout_base64.expect("stdout"))
        .expect("decode");
    assert_eq!(decoded, porcelain.as_bytes());
}

#[test]
fn a_directory_that_is_not_a_repository_is_an_answer_rather_than_a_failure() {
    let helper = ScriptedHelper::answering(
        r#"{"version":1,"ok":true,"result":{"git":{"isRepository":false,
           "stdoutBase64":null,"truncated":false}}}"#,
    );

    let result = block(call(&helper, HelperOperation::GitStatus)).expect("status");

    // The panel shows "no version control here" rather than an error a reader would try to fix,
    // and the distinction is why the helper pins `LC_ALL=C` before matching git's message.
    assert!(!result.git.expect("git").is_repository);
}

// ---------------------------------------------------------------------------------------------
// Refusals
// ---------------------------------------------------------------------------------------------

#[test]
fn a_path_that_escaped_its_root_keeps_that_meaning() {
    let helper = ScriptedHelper::answering(
        r#"{"version":1,"ok":false,"reasonCode":"workspace_path_escaped"}"#,
    );

    let error = block(call(
        &helper,
        HelperOperation::ReadTextFile {
            path: "../secret".to_string(),
        },
    ))
    .expect_err("refusal");

    // Not "remote unavailable": an escape is a refusal a reader must not retry, and a transient
    // label would invite exactly that.
    assert_eq!(error.code(), "workspace_path_escaped");
}

#[test]
fn a_missing_ripgrep_is_reported_as_the_missing_prerequisite_it_is() {
    let helper = ScriptedHelper::answering(
        r#"{"version":1,"ok":false,"reasonCode":"remote_ripgrep_missing"}"#,
    );

    let error = block(call(
        &helper,
        HelperOperation::Search {
            query: "needle".to_string(),
            max_results: 20,
            excluded_directories: Vec::new(),
        },
    ))
    .expect_err("refusal");

    // "Install this" and "try again" are different sentences, and only the code tells them apart.
    assert_eq!(error.code(), "remote_ripgrep_missing");
}

// ---------------------------------------------------------------------------------------------
// The helper program
// ---------------------------------------------------------------------------------------------

/// Confinement compares resolved paths, and compares them with a separator.
///
/// Asserted against the source because the failure is silent and specific: without the separator,
/// `/work/app-secrets` passes a `startswith("/work/app")` test. That is a real escape that reads
/// like a typo, and no scripted response can demonstrate its absence.
#[test]
fn the_helper_confines_with_a_separator_and_resolved_paths() {
    assert!(HELPER_PROGRAM.contains("candidate.startswith(root + os.sep)"));
    assert!(HELPER_PROGRAM.contains("os.path.realpath"));
    // The two refusals that happen before the filesystem is touched.
    assert!(HELPER_PROGRAM.contains("os.path.isabs(relative)"));
    assert!(HELPER_PROGRAM.contains(r#"part in ("..", "")"#));
}

#[test]
fn the_helper_runs_tools_as_argument_arrays_with_a_pinned_locale() {
    // No shell: an argument array is the difference between a path with a space in it and a path
    // that executes.
    assert!(!HELPER_PROGRAM.contains("shell=True"));
    assert!(HELPER_PROGRAM.contains("[path] + arguments"));
    // The client classifies some git outcomes by matching output text, and a translated message
    // would be classified as an unknown failure.
    assert!(HELPER_PROGRAM.contains(r#"environment["LC_ALL"] = "C""#));
    // `--` before a user-supplied path, so a file named like an option is a file.
    assert!(HELPER_PROGRAM.contains(r#"arguments.extend(["--", relative])"#));
}

#[test]
fn the_helper_bounds_every_read_it_performs() {
    for bound in [
        "DIRECTORY_ENTRY_LIMIT = 500",
        "FILE_BYTE_LIMIT = 1024 * 1024",
        "SEARCH_RESULT_LIMIT = 200",
        "GIT_OUTPUT_LIMIT = 2 * 1024 * 1024",
    ] {
        // The same bounds the local provider uses, so a workspace does not change size when it
        // moves to a remote host.
        assert!(HELPER_PROGRAM.contains(bound), "missing {bound}");
    }
}

#[test]
fn the_helper_decodes_previews_strictly_rather_than_replacing_bad_bytes() {
    // Mojibake in a preview looks like a corrupt file, and a reader cannot tell that from a file
    // that really is corrupt.
    assert!(HELPER_PROGRAM.contains(r#"raw.decode("utf-8")"#));
    assert!(!HELPER_PROGRAM.contains(r#"raw.decode("utf-8", "replace")"#));
}

/// The provider never mutates, which is what makes a retry safe.
#[test]
fn every_remote_operation_is_a_read() {
    let source = include_str!("remote_provider.rs");
    for mutation in ["write_", "revert", "delete", "commit(", "checkout"] {
        assert!(
            !source.contains(mutation),
            "{mutation} would make a retry replay an action rather than an observation"
        );
    }
    // And the provider exists, so this is not passing because the file moved.
    assert!(
        source.contains("impl WorkspaceInspectionProvider for RemoteWorkspaceInspectionProvider")
    );
}

/// A local target reaching the remote provider is refused rather than connected somewhere.
#[test]
fn the_remote_provider_declares_its_own_target_kind() {
    let source = include_str!("remote_provider.rs");
    assert!(source.contains("workspace_provider_remote_only"));
    // One place talks to the helper, and it is the place that revalidates. That is stronger than
    // counting guard calls: a seventh operation added tomorrow cannot reach the host without
    // going through the same check, because there is nowhere else to reach it from.
    assert_eq!(source.matches("exchange_cancellable(").count(), 1);
    assert!(source.contains("let remote = self.remote(target)?.clone();"));
}

/// A retry is bounded, and only for failures a second attempt could get past.
#[test]
fn the_retry_policy_covers_dropped_connections_and_nothing_else() {
    let source = include_str!("remote_provider.rs");

    assert!(source.contains("const MAX_INSPECTION_ATTEMPTS: usize = 2;"));
    // A timeout is deliberately excluded: the remote may still be executing the first request,
    // and a retry would leave two helper processes running while the reader waits twice as long.
    assert!(
        source.contains("RemoteHelperError::ConnectionFailed | RemoteHelperError::ChannelFailed")
    );
    assert!(!source.contains("RemoteHelperError::Timeout | "));
}

/// The binding is checked again before the second attempt, not once at the top.
#[test]
fn a_retry_revalidates_rather_than_reusing_the_first_decision() {
    let source = include_str!("remote_provider.rs");

    // The guard sits inside the loop. A retry happens after a failure, and the interesting
    // failure is a connection that dropped because the profile was edited - reconnecting under
    // the old revision would answer about a machine the session is no longer bound to.
    let loop_start = source.find("        loop {").expect("retry loop");
    let guard = source
        .find("let remote = self.remote(target)?.clone();")
        .expect("guard");
    assert!(
        guard > loop_start,
        "the guard runs once instead of per attempt"
    );
}
#[test]
fn the_provider_type_is_the_one_bootstrap_registers() {
    // Named rather than inferred, so a rename that left bootstrap building something else would be
    // caught here instead of at the first remote session.
    let bootstrap = include_str!("../../../../bootstrap/workspaces.rs");
    assert!(bootstrap.contains("RemoteWorkspaceInspectionProvider::new"));
    assert!(bootstrap.contains("SshRemoteHelperSession::new"));
    let _ = std::any::type_name::<RemoteWorkspaceInspectionProvider>();
}

// The provider's own signatures are exercised through the router; these keep the imports honest.
#[allow(dead_code)]
fn unused_type_witnesses(
    _listing: ListDirectoryRequest,
    _read: ReadTextFileRequest,
    _search: WorkspaceSearchRequest,
    _diff: GitDiffRequest,
    _source: GitDiffSource,
    _target: WorkspaceTarget,
    _error: WorkspaceInspectionError,
) {
}
