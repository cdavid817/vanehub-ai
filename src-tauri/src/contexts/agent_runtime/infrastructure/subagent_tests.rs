use super::*;
use serde_json::json;

fn call(name: &str, input: Value) -> ToolUseBlock {
    ToolUseBlock {
        id: format!("call-{name}"),
        name: name.to_owned(),
        input: Some(input),
        output: None,
        status: "pending".to_owned(),
        skill_provenance: None,
    }
}

/// The child's authority is structural: these three definitions are all it is offered, and
/// `execute_child_tool` is the only dispatcher. There is no allowlist to get wrong.
#[test]
fn the_child_catalog_is_exactly_three_read_only_tools() {
    let names: Vec<String> = child_tool_catalog(false)
        .into_iter()
        .map(|tool| tool.name)
        .collect();
    assert_eq!(names, vec!["file", "grep", "glob"]);
}

/// A mutating child gains exactly `edit` and a writable `file`, and nothing else. In particular it
/// still cannot run a command, reach the network, ask a question, or delegate.
#[test]
fn a_mutating_child_gains_only_write_tools() {
    let names: Vec<String> = child_tool_catalog(true)
        .into_iter()
        .map(|tool| tool.name)
        .collect();
    let mut sorted = names.clone();
    sorted.sort_unstable();
    assert_eq!(sorted, vec!["edit", "file", "glob", "grep"]);

    // The writable `file` definition replaced the plan-mode one, which cannot write.
    let file = child_tool_catalog(true)
        .into_iter()
        .find(|tool| tool.name == "file")
        .expect("file tool");
    assert_eq!(
        file.input_schema["properties"]["operation"]["enum"],
        json!(["read", "write"])
    );

    for forbidden in [
        "shell",
        "shell_kill",
        "ask_user_question",
        "delegate_subagent",
        "delegate_cli",
        "apply_delegation_changes",
    ] {
        assert!(!names.contains(&forbidden.to_owned()), "{forbidden}");
    }
}

/// The read-only dispatcher is a separate function, not a branch inside the mutating one, so a
/// read-only child has no code path to a write regardless of any flag.
#[test]
fn the_mutating_dispatcher_writes_and_the_read_only_one_still_cannot() {
    let directory = crate::test_support::TempDirectory::new("subagent-mutating-dispatch");
    let root = directory.path().to_string_lossy().to_string();
    std::fs::write(directory.path().join("target.txt"), "before\n").expect("fixture");

    let write = call(
        "file",
        json!({"operation": "write", "path": "target.txt", "content": "after\n"}),
    );

    let (_output, is_error) = execute_child_tool(&write, &root);
    assert!(!is_error);
    assert_eq!(
        std::fs::read_to_string(directory.path().join("target.txt")).expect("read"),
        "before\n",
        "the read-only dispatcher must not write, whatever the operation says"
    );

    let (_output, is_error) = execute_mutating_child_tool(&write, &root);
    assert!(!is_error);
    assert_eq!(
        std::fs::read_to_string(directory.path().join("target.txt")).expect("read"),
        "after\n",
        "the mutating dispatcher writes"
    );
}

#[test]
fn the_mutating_dispatcher_delegates_reads_to_the_read_only_one() {
    let directory = crate::test_support::TempDirectory::new("subagent-mutating-read");
    let root = directory.path().to_string_lossy().to_string();
    std::fs::write(directory.path().join("a.txt"), "needle\n").expect("fixture");

    let (output, is_error) =
        execute_mutating_child_tool(&call("file", json!({"path": "a.txt"})), &root);
    assert!(!is_error, "{output}");
    assert!(output.contains("needle"), "{output}");

    // Still refuses anything outside the child surface.
    let (refused, is_error) =
        execute_mutating_child_tool(&call("shell", json!({"command": "echo hi"})), &root);
    assert!(is_error);
    assert!(refused.contains("not available to a subagent"), "{refused}");
}

/// A captured status letter must land on the right change kind, or a reviewer reads "modified"
/// for a file that was deleted.
#[test]
fn captured_status_letters_map_to_change_kinds() {
    let kind = |status: char| {
        change_set_file(&CapturedFile {
            path: "a.txt".to_owned(),
            status,
            new_hash: None,
            binary: false,
        })
        .change_kind
    };
    assert_eq!(kind('?'), FileChangeKind::Add);
    assert_eq!(kind('A'), FileChangeKind::Add);
    assert_eq!(kind('D'), FileChangeKind::Delete);
    assert_eq!(kind('R'), FileChangeKind::Rename);
    assert_eq!(kind('M'), FileChangeKind::Modify);
}

#[test]
fn the_child_catalog_excludes_every_effectful_and_delegating_tool() {
    let names: Vec<String> = child_tool_catalog(false)
        .into_iter()
        .map(|tool| tool.name)
        .collect();
    for forbidden in [
        "shell",
        "shell_output",
        "shell_kill",
        "edit",
        "remember",
        "todo_write",
        "ask_user_question",
        "delegate_subagent",
        "delegate_cli",
        "apply_delegation_changes",
        "browser",
        "web_search",
        "web_fetch",
        "code_execution",
    ] {
        assert!(!names.contains(&forbidden.to_owned()), "{forbidden}");
    }
}

/// The file tool is dispatched with a hardcoded "read". A model asking for a write reaches the
/// read path, not the write path -- the write is unreachable rather than rejected.
#[test]
fn a_child_write_request_still_reaches_only_the_read_path() {
    let directory = crate::test_support::TempDirectory::new("subagent-write-attempt");
    let workspace = directory.path().to_string_lossy().to_string();
    let target = directory.path().join("untouched.txt");
    std::fs::write(&target, "original").expect("fixture");

    let (_output, _is_error) = execute_child_tool(
        &call(
            "file",
            json!({"operation": "write", "path": "untouched.txt", "content": "overwritten"}),
        ),
        &workspace,
    );

    assert_eq!(
        std::fs::read_to_string(&target).expect("read back"),
        "original",
        "a child must not be able to write, whatever it puts in `operation`"
    );
}

#[test]
fn a_child_can_read_search_and_glob_inside_its_workspace() {
    let directory = crate::test_support::TempDirectory::new("subagent-read");
    let workspace = directory.path().to_string_lossy().to_string();
    std::fs::write(directory.path().join("a.txt"), "needle here\n").expect("fixture");

    let (read, read_error) =
        execute_child_tool(&call("file", json!({"path": "a.txt"})), &workspace);
    assert!(!read_error, "{read}");
    assert!(read.contains("needle"), "{read}");

    let (found, grep_error) =
        execute_child_tool(&call("grep", json!({"pattern": "needle"})), &workspace);
    assert!(!grep_error, "{found}");
    assert!(found.contains("a.txt"), "{found}");

    let (listed, glob_error) =
        execute_child_tool(&call("glob", json!({"pattern": "*.txt"})), &workspace);
    assert!(!glob_error, "{listed}");
    assert!(listed.contains("a.txt"), "{listed}");
}

#[test]
fn a_tool_outside_the_child_surface_is_refused_by_the_dispatcher() {
    let directory = crate::test_support::TempDirectory::new("subagent-unknown-tool");
    let workspace = directory.path().to_string_lossy().to_string();

    for name in ["shell", "edit", "delegate_subagent", "ask_user_question"] {
        let (output, is_error) =
            execute_child_tool(&call(name, json!({"command": "echo hi"})), &workspace);
        assert!(is_error, "{name} must be refused");
        assert!(output.contains("not available to a subagent"), "{output}");
    }
}

#[test]
fn a_child_read_cannot_escape_its_workspace() {
    let directory = crate::test_support::TempDirectory::new("subagent-escape");
    let workspace = directory.path().to_string_lossy().to_string();

    let (output, is_error) =
        execute_child_tool(&call("file", json!({"path": "../outside.txt"})), &workspace);
    assert!(is_error, "{output}");
}

#[test]
fn an_answer_is_trimmed_and_capped() {
    assert_eq!(bounded("  the answer  ").as_deref(), Some("the answer"));
    assert_eq!(bounded("   ").as_deref(), None);
    assert_eq!(bounded("").as_deref(), None);

    let long = "x".repeat(MAX_CHILD_RESULT_CHARS + 500);
    assert_eq!(
        bounded(&long).expect("capped").chars().count(),
        MAX_CHILD_RESULT_CHARS
    );
}

/// An empty answer is a failure, not an empty success: returning one would read to the parent as
/// "investigated, found nothing".
#[test]
fn an_empty_answer_is_reported_as_a_failure() {
    let envelope = succeeded("   ", 3);
    assert_eq!(envelope.status, NativeToolResultStatus::Failed);
    assert_eq!(
        envelope.error_code,
        Some(NativeToolErrorCode::ExternalFailure)
    );
}

#[test]
fn a_capped_answer_reports_truncation() {
    let long = "y".repeat(MAX_CHILD_RESULT_CHARS + 1);
    let envelope = succeeded(&long, 5);
    assert_eq!(envelope.status, NativeToolResultStatus::Succeeded);
    assert!(envelope.truncated);
    assert_eq!(envelope.metadata["tool_calls"], json!(5));

    let short = succeeded("brief", 1);
    assert!(!short.truncated);
    assert_eq!(short.output.expect("output")["summary"], json!("brief"));
}

/// Counts and timing only: the child's turns, tool inputs, and tool outputs never reach anywhere
/// the parent can read them.
#[test]
fn result_metadata_carries_counts_and_nothing_else() {
    let envelope = succeeded("done", 7);
    assert_eq!(
        envelope.metadata.keys().cloned().collect::<Vec<_>>(),
        vec!["tool_calls".to_owned()]
    );
    assert_eq!(envelope.metadata["tool_calls"], json!(7));
}

#[test]
fn terminal_outcomes_map_to_their_error_codes() {
    assert_eq!(
        terminal(NativeToolResultStatus::LimitExceeded, None, 0).error_code,
        Some(NativeToolErrorCode::LimitExceeded)
    );
    assert_eq!(
        terminal(NativeToolResultStatus::Cancelled, None, 0).error_code,
        Some(NativeToolErrorCode::Cancelled)
    );
}

/// A refused claim never terminates a running child to make room.
#[test]
fn the_concurrency_cap_is_per_session_and_releases_slots() {
    let slots = ConcurrencySlots::default();

    for index in 0..MAX_CONCURRENT_CHILDREN {
        assert!(slots.claim("session-a"), "claim {index}");
    }
    assert!(
        !slots.claim("session-a"),
        "a session at its cap must be refused"
    );
    assert!(
        slots.claim("session-b"),
        "another session has its own budget"
    );

    slots.release("session-a");
    assert!(
        slots.claim("session-a"),
        "a released slot becomes available again"
    );
}

/// Child spend must be recorded, but a caller with no accounting handle (Web/mock, tests) must
/// degrade rather than panic.
#[test]
fn a_child_invocation_without_accounting_declines_instead_of_failing() {
    let config = crate::contexts::agent_runtime::application::ApiProviderConfig {
        source_provider_id: Some("openai".to_owned()),
        model_id: "gpt-5.4".to_owned(),
        interface_format: "openai-compatible".to_owned(),
        base_url: Some("https://api.openai.com/v1".to_owned()),
        auto_approve_tools: false,
    };
    let invocation = super::super::api_process_adapter::begin_child_invocation(
        None,
        super::super::api_process_adapter::ChildInvocationIdentity {
            call_id: "call-1",
            session_id: "session-1",
            agent_id: "onepiece",
            operation_id: "operation-1",
        },
        &config,
        0,
        &super::super::SystemAgentRuntimeClock,
    );
    assert!(invocation.is_none());
}

/// The handler advertises a duration ceiling; the dispatcher enforces the native-tool request
/// timeout. If they drift apart, the limit profile promises time a child never gets.
#[test]
fn the_declared_duration_matches_the_enforced_deadline() {
    use crate::contexts::agent_runtime::application::MAX_SUBAGENT_DURATION_MS;

    assert_eq!(
        u128::from(MAX_SUBAGENT_DURATION_MS),
        super::super::api_process_adapter::REQUEST_TIMEOUT.as_millis(),
        "the advertised subagent duration must be the deadline actually applied"
    );
}

#[derive(Debug, Default)]
struct CapturingProgress {
    events: Mutex<Vec<NativeToolProgress>>,
}

impl crate::contexts::agent_runtime::application::NativeToolProgressSink for CapturingProgress {
    fn publish(&self, progress: NativeToolProgress) {
        self.events.lock().expect("events").push(progress);
    }
}

fn execution_context(
    cancelled: Arc<AtomicBool>,
    progress: Arc<CapturingProgress>,
) -> crate::contexts::agent_runtime::application::NativeToolExecutionContext {
    crate::contexts::agent_runtime::application::NativeToolExecutionContext {
        call_id: "call-1".to_owned(),
        session_id: "session-1".to_owned(),
        generation_id: "generation-1".to_owned(),
        agent_id: "onepiece".to_owned(),
        canonical_workspace: Some(std::path::PathBuf::from("C:/work")),
        deadline: std::time::Instant::now() + std::time::Duration::from_secs(60),
        cancelled,
        progress,
    }
}

/// The child shares the generation's cancellation flag, so cancelling the generation -- which is
/// what ending or archiving a session does -- stops the child at its next turn boundary. Pinned
/// because it is inherited rather than implemented here, and inherited guarantees are the ones
/// that quietly disappear.
#[test]
fn the_child_observes_the_generations_cancellation_flag() {
    let cancelled = Arc::new(AtomicBool::new(true));
    let context = execution_context(cancelled.clone(), Arc::new(CapturingProgress::default()));

    assert!(context.is_cancelled());
    cancelled.store(false, std::sync::atomic::Ordering::Release);
    assert!(!context.is_cancelled());
}

/// Progress carries counts and a fixed phrase. A file path or search pattern here would leak the
/// child's reading into a user-facing signal, which is the one thing its own context is for.
#[test]
fn progress_events_carry_counts_and_no_content() {
    let sink = Arc::new(CapturingProgress::default());
    let context = execution_context(Arc::new(AtomicBool::new(false)), sink.clone());

    publish_progress(
        &context,
        2,
        NativeToolProgressPhase::Updated,
        "Reading 3 more sources.".to_owned(),
        7,
    );

    let events = sink.events.lock().expect("events");
    let event = events.first().expect("a progress event");
    assert_eq!(event.sequence, 2);
    assert_eq!(event.metadata["tool_calls"], json!(7));
    let message = event.message.as_deref().expect("message");
    assert!(message.contains('3'), "{message}");
    for leaked in ["C:/", ".rs", "pattern", "needle"] {
        assert!(!message.contains(leaked), "{message}");
    }
}

// ---------------------------------------------------------------------------------------------
// Provider-driven coverage of the child turn loop.
//
// Everything above tests the loop's parts in isolation. These drive the loop itself against a
// scripted SSE endpoint, which is the only way to cover the turn sequence: that a tool call is
// executed and its result fed back, that the loop stops when the model stops asking for tools,
// and that the turn ceiling terminates a model that never stops.
//
// Note for anyone running these by hand: `reqwest`'s builder honours `ALL_PROXY`/`HTTP_PROXY`, so
// a SOCKS proxy in the environment will intercept the loopback request and these will fail with a
// transport error. Run with those unset.
// ---------------------------------------------------------------------------------------------

use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};

/// Serves one canned SSE body per request, in order, then closes.
struct ScriptedProvider {
    base_url: String,
    requests: Arc<Mutex<Vec<String>>>,
}

impl ScriptedProvider {
    fn start(bodies: Vec<String>) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
        let port = listener.local_addr().expect("addr").port();
        let requests = Arc::new(Mutex::new(Vec::new()));
        let recorded = requests.clone();
        std::thread::spawn(move || {
            for (index, body) in bodies.into_iter().enumerate() {
                let Ok((stream, _)) = listener.accept() else {
                    return;
                };
                if let Some(payload) = serve(stream, &body) {
                    recorded.lock().expect("requests").push(payload);
                }
                let _ = index;
            }
        });
        Self {
            base_url: format!("http://127.0.0.1:{port}"),
            requests,
        }
    }

    fn request_bodies(&self) -> Vec<String> {
        self.requests.lock().expect("requests").clone()
    }
}

/// Reads one HTTP request and replies with `body` as an SSE stream. Returns the request body so a
/// test can assert what the child actually sent back on its next turn.
fn serve(stream: TcpStream, body: &str) -> Option<String> {
    let mut reader = BufReader::new(stream.try_clone().ok()?);
    let mut length = 0_usize;
    loop {
        let mut line = String::new();
        if reader.read_line(&mut line).ok()? == 0 {
            return None;
        }
        let trimmed = line.trim_end();
        if let Some(value) = trimmed.strip_prefix("content-length:") {
            length = value.trim().parse().unwrap_or(0);
        } else if let Some(value) = trimmed.strip_prefix("Content-Length:") {
            length = value.trim().parse().unwrap_or(0);
        }
        if trimmed.is_empty() {
            break;
        }
    }
    let mut payload = vec![0_u8; length];
    std::io::Read::read_exact(&mut reader, &mut payload).ok()?;

    let mut stream = stream;
    let response = format!(
        "HTTP/1.1 200 OK\r\ncontent-type: text/event-stream\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
        body.len(),
        body
    );
    stream.write_all(response.as_bytes()).ok()?;
    stream.flush().ok()?;
    String::from_utf8(payload).ok()
}

fn text_turn(text: &str) -> String {
    let delta = json!({ "choices": [{ "delta": { "content": text } }] });
    format!("data: {delta}\n\ndata: [DONE]\n\n")
}

fn tool_call_turn(id: &str, name: &str, arguments: Value) -> String {
    let delta = json!({
        "choices": [{
            "delta": {
                "tool_calls": [{
                    "index": 0,
                    "id": id,
                    "type": "function",
                    "function": { "name": name, "arguments": arguments.to_string() }
                }]
            }
        }]
    });
    format!("data: {delta}\n\ndata: [DONE]\n\n")
}

struct StubCredentials;

impl crate::contexts::agent_runtime::application::ApiCredentialPort for StubCredentials {
    fn fetch(
        &self,
        _agent_id: &str,
    ) -> Result<
        Option<String>,
        crate::contexts::agent_runtime::application::AgentRuntimeApplicationError,
    > {
        Ok(Some("test-key".to_owned()))
    }

    fn store(
        &self,
        _agent_id: &str,
        _api_key: &str,
    ) -> Result<(), crate::contexts::agent_runtime::application::AgentRuntimeApplicationError> {
        Ok(())
    }

    fn remove(
        &self,
        _agent_id: &str,
    ) -> Result<(), crate::contexts::agent_runtime::application::AgentRuntimeApplicationError> {
        Ok(())
    }
}

/// Supplies a provider configuration whose base URL points at the scripted endpoint. Only
/// `provider_config` is reachable from the child loop; the rest of the gateway is never called.
struct StubGateway {
    base_url: String,
}

impl crate::contexts::agent_runtime::application::ApiAgentGateway for StubGateway {
    fn register(
        &self,
        _agent_id: &str,
        _input: &crate::contexts::agent_runtime::application::RegisterApiAgentInput,
    ) -> Result<
        crate::contexts::agent_runtime::domain::AgentDefinition,
        crate::contexts::agent_runtime::application::AgentRuntimeApplicationError,
    > {
        unreachable!("a child never registers an agent")
    }

    fn provider_config(
        &self,
        _agent_id: &str,
    ) -> Result<
        Option<crate::contexts::agent_runtime::application::ApiProviderConfig>,
        crate::contexts::agent_runtime::application::AgentRuntimeApplicationError,
    > {
        Ok(Some(
            crate::contexts::agent_runtime::application::ApiProviderConfig {
                source_provider_id: Some("scripted".to_owned()),
                model_id: "scripted-model".to_owned(),
                interface_format: "openai-compatible".to_owned(),
                base_url: Some(self.base_url.clone()),
                auto_approve_tools: false,
            },
        ))
    }

    fn update(
        &self,
        _agent_id: &str,
        _input: &crate::contexts::agent_runtime::application::UpdateApiAgentInput,
    ) -> Result<
        crate::contexts::agent_runtime::domain::AgentDefinition,
        crate::contexts::agent_runtime::application::AgentRuntimeApplicationError,
    > {
        unreachable!("a child never updates an agent")
    }

    fn delete(
        &self,
        _agent_id: &str,
    ) -> Result<(), crate::contexts::agent_runtime::application::AgentRuntimeApplicationError> {
        unreachable!("a child never deletes an agent")
    }
}

fn executor_for(
    provider: &ScriptedProvider,
    directory: &crate::test_support::TempDirectory,
) -> NativeSubagentExecutor {
    use crate::contexts::artifacts::application::ArtifactBlobStorePolicy;
    use crate::contexts::artifacts::infrastructure::{ArtifactBlobStore, SqliteArtifactCatalog};
    use crate::platform::database::NativeDatabase;

    let data_root = directory.path().join("data");
    let database = NativeDatabase::new(data_root.clone()).expect("database");
    let artifacts = Arc::new(ArtifactService::new(
        Arc::new(
            ArtifactBlobStore::new(
                &data_root,
                ArtifactBlobStorePolicy {
                    max_blob_bytes: 8 * 1024 * 1024,
                    max_operation_items: 8,
                    max_operation_bytes: 16 * 1024 * 1024,
                    max_total_bytes: 64 * 1024 * 1024,
                },
            )
            .expect("blob store"),
        ),
        Arc::new(SqliteArtifactCatalog::new(database.clone())),
    ));
    NativeSubagentExecutor::new(SubagentRuntime {
        credentials: Arc::new(StubCredentials),
        config: Arc::new(StubGateway {
            base_url: provider.base_url.clone(),
        }),
        accounting: None,
        clock: Arc::new(super::super::SystemAgentRuntimeClock),
        logging: Arc::new(SilentLogging),
        artifacts,
        operations: Arc::new(SqliteNativeToolRepository::new(database)),
        operations_root: directory.path().join("worktrees"),
    })
}

#[derive(Debug, Default)]
struct SilentLogging;

impl crate::contexts::agent_runtime::application::AgentLoggingPort for SilentLogging {
    fn record(
        &self,
        _log: crate::contexts::agent_runtime::application::AgentLog,
    ) -> Result<(), crate::contexts::agent_runtime::application::AgentRuntimeApplicationError> {
        Ok(())
    }
}

fn port_request(task: &str, workspace: &std::path::Path) -> NativeToolPortRequest {
    NativeToolPortRequest {
        input: crate::contexts::agent_runtime::application::ValidatedNativeToolInput {
            value: json!({ "task": task }),
            input_hash: "hash".to_owned(),
            operation:
                crate::contexts::agent_runtime::application::NativeToolOperation::SubagentDelegate,
            resource: crate::contexts::agent_runtime::application::CanonicalToolResource {
                kind: crate::contexts::agent_runtime::application::ToolResourceKind::Subagent,
                canonical_id: "subagent/task/hash".to_owned(),
                attributes: BTreeMap::new(),
            },
        },
        context: crate::contexts::agent_runtime::application::NativeToolExecutionContext {
            call_id: "call-1".to_owned(),
            session_id: "session-1".to_owned(),
            generation_id: "generation-1".to_owned(),
            agent_id: "onepiece".to_owned(),
            canonical_workspace: Some(workspace.to_path_buf()),
            deadline: std::time::Instant::now() + std::time::Duration::from_secs(30),
            cancelled: Arc::new(AtomicBool::new(false)),
            progress: Arc::new(CapturingProgress::default()),
        },
    }
}

#[test]
fn a_child_that_answers_without_tools_returns_its_answer() {
    let workspace = crate::test_support::TempDirectory::new("subagent-loop-answer");
    let runtime = crate::test_support::TempDirectory::new("subagent-loop-answer-runtime");
    let provider = ScriptedProvider::start(vec![text_turn("The parser lives in parser.rs.")]);
    let executor = executor_for(&provider, &runtime);

    let envelope =
        executor.execute_subagent(port_request("Where is the parser?", workspace.path()));

    assert_eq!(envelope.status, NativeToolResultStatus::Succeeded);
    assert_eq!(
        envelope.output.expect("output")["summary"],
        json!("The parser lives in parser.rs.")
    );
}

/// The turn sequence: the child asks for a tool, the loop executes it, and the next request
/// carries the result back. This is the part no unit test above can reach.
#[test]
fn a_tool_call_is_executed_and_its_result_fed_back() {
    let workspace = crate::test_support::TempDirectory::new("subagent-loop-tool");
    let runtime = crate::test_support::TempDirectory::new("subagent-loop-tool-runtime");
    std::fs::write(workspace.path().join("parser.rs"), "fn parse() {}\n").expect("fixture");
    let provider = ScriptedProvider::start(vec![
        tool_call_turn("call_a", "grep", json!({ "pattern": "fn parse" })),
        text_turn("Found it in parser.rs."),
    ]);
    let executor = executor_for(&provider, &runtime);

    let envelope =
        executor.execute_subagent(port_request("Find the parse function.", workspace.path()));

    assert_eq!(envelope.status, NativeToolResultStatus::Succeeded);
    assert_eq!(
        envelope.output.expect("output")["summary"],
        json!("Found it in parser.rs.")
    );
    assert_eq!(envelope.metadata["tool_calls"], json!(1));

    let requests = provider.request_bodies();
    assert_eq!(requests.len(), 2, "the loop took two turns");
    assert!(
        requests[1].contains("parser.rs"),
        "the second turn must carry the tool's result back: {}",
        requests[1]
    );
}

/// A model that never stops asking for tools is stopped by the turn ceiling rather than looping
/// until the parent's deadline.
#[test]
fn a_child_that_never_concludes_is_stopped_by_the_turn_ceiling() {
    let workspace = crate::test_support::TempDirectory::new("subagent-loop-ceiling");
    let runtime = crate::test_support::TempDirectory::new("subagent-loop-ceiling-runtime");
    std::fs::write(workspace.path().join("a.txt"), "x\n").expect("fixture");
    let bodies = (0..MAX_CHILD_TURNS)
        .map(|index| {
            tool_call_turn(
                &format!("call_{index}"),
                "glob",
                json!({ "pattern": "*.txt" }),
            )
        })
        .collect();
    let provider = ScriptedProvider::start(bodies);
    let executor = executor_for(&provider, &runtime);

    let envelope = executor.execute_subagent(port_request("Keep looking.", workspace.path()));

    assert_eq!(envelope.status, NativeToolResultStatus::LimitExceeded);
    assert_eq!(
        envelope.error_code,
        Some(NativeToolErrorCode::LimitExceeded)
    );
    assert_eq!(envelope.metadata["tool_calls"], json!(MAX_CHILD_TURNS));
}
