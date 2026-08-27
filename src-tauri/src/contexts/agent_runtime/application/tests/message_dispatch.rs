use super::*;

#[test]
fn launch_coordinates_lifecycle_details_operations_logs_and_failure_state() {
    let world = test_world();
    let service = service(world.clone());
    service
        .select_agent("codex-cli", InteractionMode::Cli)
        .expect("select");

    let launched = service.launch_active_workflow().expect("launch");
    assert_eq!(launched.operation_id, "operation-1");
    assert_eq!(launched.workflow.lifecycle, AgentLifecycle::Running);
    assert_eq!(
        world.details.lock().expect("details").0,
        InteractionMode::Cli.as_str()
    );
    assert!(world
        .operations
        .lock()
        .expect("operations")
        .contains(&OperationEvent::Completed("operation-1".to_string())));
    assert_eq!(
        world.logs.lock().expect("logs").last().unwrap().occurred_at,
        "2026-07-18T12:00:00Z"
    );

    world.launch_failure.store(true, Ordering::SeqCst);
    assert!(matches!(
        service.launch_active_workflow(),
        Err(AgentRuntimeApplicationError::Process(_))
    ));
    assert_eq!(
        world.workflow.lock().expect("workflow").lifecycle(),
        AgentLifecycle::Failed
    );
    assert!(world
        .operations
        .lock()
        .expect("operations")
        .contains(&OperationEvent::Failed("operation-1".to_string())));
}

#[test]
fn send_message_persists_before_reserving_control_and_attaches_effective_prompt_process() {
    let world = test_world();
    let service = service(world.clone());
    let message = service
        .send_message(SendMessageRequest {
            source: AgentMessageSource::Desktop,
            session_id: "session-1".to_string(),
            content: "  explain this  ".to_string(),
            configuration: chat_configuration(),
            file_references: vec![AgentFileReference {
                id: "file-1".to_string(),
                path: "src/main.rs".to_string(),
                name: "main.rs".to_string(),
                size_bytes: Some(10),
                content_hash: Some("hash".to_string()),
                start_line: None,
                end_line: None,
            }],
        })
        .expect("send");

    assert_eq!(message.id, "message-2");
    assert_eq!(message.status, "streaming");
    assert_eq!(
        *world.generation_order.lock().expect("generation order"),
        vec!["durable-claim", "control-reserve"]
    );
    let requests = world
        .generation_requests
        .lock()
        .expect("generation requests");
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].runner, RunnerSelection::local());
    assert!(requests[0]
        .effective_prompt
        .starts_with("effective::explain this"));
    assert_eq!(requests[0].cli_profile.executable, "C:/bin/codex-cli.exe");
    drop(requests);
    assert_eq!(
        *world.lifecycle_updates.lock().expect("lifecycle updates"),
        vec![AgentLifecycle::Starting, AgentLifecycle::Running]
    );
    let active = world
        .active_generation
        .lock()
        .expect("active generation")
        .clone()
        .expect("attached generation");
    assert_eq!(active.1.as_deref(), Some("message-2"));
    assert_eq!(active.2.as_deref(), Some("process-1"));
    let coordinated_context = active.4.expect("coordinated execution context");
    let process_context = world
        .generation_requests
        .lock()
        .expect("generation requests")[0]
        .execution_context
        .clone();
    assert_eq!(coordinated_context.run_id, process_context.run_id);
    assert_eq!(coordinated_context.trace_id, process_context.trace_id);
}

#[test]
fn runner_discovery_and_selected_execution_preserve_the_service_boundary() {
    let world = test_world();
    let runtime = service(world.clone());
    let descriptors = runtime
        .list_runners("session-1", "codex-cli")
        .expect("runner discovery");
    assert_eq!(descriptors.len(), 1);
    assert_eq!(descriptors[0].selection, RunnerSelection::local());

    let selection = RunnerSelection::ssh("connection-1".to_string(), 7).expect("selection");
    runtime
        .send_message_with_runner(
            SendMessageRequest {
                source: AgentMessageSource::Desktop,
                session_id: "session-1".to_string(),
                content: "run on selected host".to_string(),
                configuration: chat_configuration(),
                file_references: Vec::new(),
            },
            selection.clone(),
        )
        .expect("send with runner");
    assert_eq!(
        world.generation_requests.lock().expect("requests")[0].runner,
        selection
    );
}

#[test]
fn invalid_runner_selection_has_no_message_or_generation_side_effect() {
    let world = test_world();
    let mut invalid = RunnerSelection::local();
    invalid.target_id = Some("unexpected".to_string());
    let result = service(world.clone()).send_message_with_runner(
        SendMessageRequest {
            source: AgentMessageSource::Desktop,
            session_id: "session-1".to_string(),
            content: "must not run".to_string(),
            configuration: chat_configuration(),
            file_references: Vec::new(),
        },
        invalid,
    );

    assert!(
        matches!(result, Err(AgentRuntimeApplicationError::Process(code)) if code == "runner_invalid_selection")
    );
    assert!(world.created_messages.lock().expect("messages").is_empty());
    assert!(world
        .generation_requests
        .lock()
        .expect("requests")
        .is_empty());
}

#[test]
fn execution_telemetry_preserves_task_agent_and_tool_topology() {
    let world = test_world();
    let (service, telemetry) = service_with_telemetry(world.clone());
    service
        .send_message(SendMessageRequest {
            source: AgentMessageSource::Desktop,
            session_id: "session-1".to_string(),
            content: "secret prompt must not be captured".to_string(),
            configuration: chat_configuration(),
            file_references: Vec::new(),
        })
        .expect("send");
    let sink = world
        .generation_sinks
        .lock()
        .expect("generation sinks")
        .get("process-1")
        .cloned()
        .expect("sink");
    for status in ["running", "completed"] {
        sink.handle(GenerationProcessEvent::ToolUse(ToolUseBlock {
            id: "provider-call-1".to_string(),
            name: "read".to_string(),
            input: None,
            output: None,
            status: status.to_string(),
            skill_provenance: None,
        }))
        .expect("tool lifecycle");
    }
    sink.handle(GenerationProcessEvent::Completed(None))
        .expect("complete");

    let records = telemetry.records().expect("telemetry records");
    let run = records
        .iter()
        .find_map(|record| match record {
            CapturedTelemetryRecord::RunStarted(run) => Some(run),
            _ => None,
        })
        .expect("run");
    let spans = records
        .iter()
        .filter_map(|record| match record {
            CapturedTelemetryRecord::SpanStarted(span) => Some(span),
            _ => None,
        })
        .collect::<Vec<_>>();
    let root = spans
        .iter()
        .find(|span| span.name == "vanehub.task.execute")
        .expect("root span");
    let prompt = spans
        .iter()
        .find(|span| span.name == "vanehub.prompt.assemble")
        .expect("prompt span");
    let agent = spans
        .iter()
        .find(|span| span.name.starts_with("invoke_agent "))
        .expect("agent span");
    let tool = spans
        .iter()
        .find(|span| span.name == "execute_tool read")
        .expect("tool span");

    assert_eq!(root.context, run.context);
    assert_eq!(prompt.parent_span_id.as_ref(), Some(&root.context.span_id));
    assert_eq!(agent.parent_span_id.as_ref(), Some(&root.context.span_id));
    assert_eq!(tool.parent_span_id.as_ref(), Some(&agent.context.span_id));
    assert_eq!(tool.fidelity, ExecutionFidelity::Inferred);
    assert!(spans
        .iter()
        .all(|span| span.context.trace_id == run.context.trace_id));
    assert!(records.iter().any(|record| matches!(
        record,
        CapturedTelemetryRecord::RunFinished {
            status: ExecutionStatus::Succeeded,
            ..
        }
    )));
    assert!(!format!("{records:?}").contains("secret prompt must not be captured"));
}

#[test]
fn telemetry_failures_are_diagnosed_without_failing_message_dispatch() {
    let world = test_world();
    let service = service_with_telemetry_port(world.clone(), Arc::new(FailingExecutionTelemetry));

    service
        .send_message(SendMessageRequest {
            source: AgentMessageSource::Desktop,
            session_id: "session-1".to_owned(),
            content: "content must not enter diagnostics".to_owned(),
            configuration: chat_configuration(),
            file_references: Vec::new(),
        })
        .expect("telemetry must remain non-authoritative");

    let logs = world.logs.lock().expect("logs");
    let telemetry_logs = logs
        .iter()
        .filter(|log| log.category == "execution_telemetry")
        .collect::<Vec<_>>();
    assert!(!telemetry_logs.is_empty());
    assert!(telemetry_logs.iter().all(|log| {
        !log.message.contains("content must not enter diagnostics")
            && log.run_id.is_some()
            && log.trace_id.is_some()
            && log.span_id.is_some()
    }));
}

#[test]
fn completion_with_reported_usage_persists_reported_accounting() {
    let world = test_world();
    let service = service(world.clone());
    service
        .send_message(SendMessageRequest {
            source: AgentMessageSource::Desktop,
            session_id: "session-1".to_string(),
            content: "explain this".to_string(),
            configuration: chat_configuration(),
            file_references: Vec::new(),
        })
        .expect("send");
    let sink = world
        .generation_sinks
        .lock()
        .expect("generation sinks")
        .get("process-1")
        .cloned()
        .expect("sink");

    sink.handle(GenerationProcessEvent::Completed(Some(
        ReportedUsageTotals {
            input_tokens: 120,
            output_tokens: 340,
            cache_read_tokens: 900,
            cache_creation_tokens: 50,
            provider_total_tokens: Some(1410),
            cache_overlap: AgentUsageOverlap::Exclusive,
            reasoning_overlap: AgentUsageOverlap::Subset,
            normalization_version: "claude-code-result-usage-v1",
            source_identity: Some("provider-step-1".to_string()),
            source_revision: Some("1720000000123".to_string()),
            ..ReportedUsageTotals::default()
        },
    )))
    .expect("complete");

    let usage = world
        .completed_invocation_usage
        .lock()
        .expect("completed invocation usage")
        .last()
        .cloned()
        .expect("usage record");
    assert_eq!(
        usage.usage.accounting_kind,
        AgentUsageAccountingKind::Reported
    );
    assert_eq!(usage.usage.input_count, 120);
    assert_eq!(usage.usage.output_count, 340);
    assert_eq!(usage.usage.cache_read_count, 900);
    assert_eq!(usage.usage.cache_creation_count, 50);
    assert_eq!(usage.usage.reasoning_output_count, 0);
    assert_eq!(usage.usage.provider_total_count, Some(1410));
    assert_eq!(usage.usage.cache_overlap, AgentUsageOverlap::Exclusive);
    assert_eq!(usage.usage.reasoning_overlap, AgentUsageOverlap::Subset);
    assert_eq!(
        usage.usage.normalization_version,
        "claude-code-result-usage-v1"
    );
    assert_eq!(usage.usage.source, "cli-reported");
    assert_eq!(usage.generation_id, usage.usage.message_id);
    assert_eq!(usage.operation_id, "generation-operation-1");
    assert_eq!(usage.source_identity.as_deref(), Some("provider-step-1"));
    assert_eq!(usage.source_revision.as_deref(), Some("1720000000123"));
    assert!(usage.invocation_id.contains("provider-step-1"));
    assert!(usage.observation_id.contains("1720000000123"));
}

#[test]
fn completion_without_reported_usage_falls_back_to_character_count_estimate() {
    let world = test_world();
    let service = service(world.clone());
    service
        .send_message(SendMessageRequest {
            source: AgentMessageSource::Desktop,
            session_id: "session-1".to_string(),
            content: "explain this".to_string(),
            configuration: chat_configuration(),
            file_references: Vec::new(),
        })
        .expect("send");
    let sink = world
        .generation_sinks
        .lock()
        .expect("generation sinks")
        .get("process-1")
        .cloned()
        .expect("sink");

    sink.handle(GenerationProcessEvent::Completed(None))
        .expect("complete");

    let usage = world
        .completed_invocation_usage
        .lock()
        .expect("completed invocation usage")
        .last()
        .cloned()
        .expect("usage record");
    assert_eq!(
        usage.usage.accounting_kind,
        AgentUsageAccountingKind::Estimated
    );
    assert_eq!(usage.usage.source, "character-count");
    assert_eq!(usage.usage.cache_read_count, 0);
    assert_eq!(usage.usage.cache_creation_count, 0);
}

#[test]
fn feishu_single_agent_turn_preserves_session_execution_context_and_completes_once() {
    let world = test_world();
    {
        let mut sessions = world.sessions.lock().expect("sessions");
        let session = sessions.get_mut("session-1").expect("session");
        session.folder = Some("C:/project/.worktrees/feishu-im".to_string());
        session.runtime_session_id = Some("provider-thread-stable".to_string());
    }
    let service = service(world.clone());
    let configuration = AgentChatConfiguration {
        agent_id: "codex-cli".to_string(),
        interaction_mode: InteractionMode::Cli,
        execution_mode: "execute".to_string(),
        provider_id: Some("openai".to_string()),
        model_id: Some("gpt-5-5".to_string()),
        reasoning_depth: Some("high".to_string()),
        streaming: true,
        thinking: true,
        long_context: false,
    };

    let started = service
        .send_message_with_completion(SendMessageRequest {
            source: AgentMessageSource::InstantMessage {
                connector_id: "feishu".to_string(),
            },
            session_id: "session-1".to_string(),
            content: "continue the bound session".to_string(),
            configuration: configuration.clone(),
            file_references: Vec::new(),
        })
        .expect("start Feishu turn");

    let requests = world
        .generation_requests
        .lock()
        .expect("generation requests");
    assert_eq!(requests.len(), 1);
    let request = &requests[0];
    assert_eq!(request.session.id, "session-1");
    assert_eq!(request.session.agent_id, "codex-cli");
    assert_eq!(request.agent.id, "codex-cli");
    assert_eq!(
        request.session.folder.as_deref(),
        Some("C:/project/.worktrees/feishu-im")
    );
    assert_eq!(request.configuration, configuration);
    assert_eq!(
        request.resume_thread_id.as_deref(),
        Some("provider-thread-stable")
    );
    assert!(!request.interactive);
    assert_eq!(request.role_briefing, None);
    drop(requests);

    let sink = world
        .generation_sinks
        .lock()
        .expect("generation sinks")
        .get("process-1")
        .cloned()
        .expect("generation sink");
    sink.handle(GenerationProcessEvent::Token("final reply".to_string()))
        .expect("terminal token");
    sink.handle(GenerationProcessEvent::Completed(None))
        .expect("terminal completion");
    sink.handle(GenerationProcessEvent::Completed(None))
        .expect("duplicate terminal completion");
    let terminal = started
        .terminal
        .recv_timeout(std::time::Duration::ZERO)
        .expect("one terminal completion");
    assert_eq!(terminal.message_id, started.message.id);
    assert_eq!(terminal.outcome, AgentMessageTerminalOutcome::Completed);
    assert_eq!(terminal.content.as_deref(), Some("final reply"));
    assert_eq!(
        world
            .operations
            .lock()
            .expect("operations")
            .iter()
            .filter(|event| matches!(event, OperationEvent::Completed(_)))
            .count(),
        1
    );
}

#[test]
fn im_completion_receiver_observes_persisted_completed_failed_and_cancelled_messages() {
    let completed_world = test_world();
    let completed_service = service(completed_world.clone());
    let completed = completed_service
        .send_message_with_completion(SendMessageRequest {
            source: AgentMessageSource::InstantMessage {
                connector_id: "managed-im".to_string(),
            },
            session_id: "session-1".to_string(),
            content: "complete this".to_string(),
            configuration: chat_configuration(),
            file_references: Vec::new(),
        })
        .expect("start completed generation");
    let completed_sink = completed_world
        .generation_sinks
        .lock()
        .expect("generation sinks")
        .get("process-1")
        .cloned()
        .expect("completed sink");
    completed_sink
        .handle(GenerationProcessEvent::Token("done".to_string()))
        .expect("token");
    completed_sink
        .handle(GenerationProcessEvent::Completed(None))
        .expect("complete");
    let completed_terminal = completed
        .terminal
        .recv_timeout(std::time::Duration::ZERO)
        .expect("completed terminal");
    assert_eq!(
        completed_terminal.outcome,
        AgentMessageTerminalOutcome::Completed
    );
    assert_eq!(completed_terminal.content.as_deref(), Some("done"));
    assert_eq!(
        completed_world
            .messages
            .lock()
            .expect("messages")
            .get(&completed_terminal.message_id)
            .expect("persisted completed")
            .status,
        "completed"
    );

    let failed_world = test_world();
    let failed_service = service(failed_world.clone());
    let failed = failed_service
        .send_message_with_completion(SendMessageRequest {
            source: AgentMessageSource::InstantMessage {
                connector_id: "managed-im".to_string(),
            },
            session_id: "session-1".to_string(),
            content: "fail this".to_string(),
            configuration: chat_configuration(),
            file_references: Vec::new(),
        })
        .expect("start failed generation");
    failed_world
        .generation_sinks
        .lock()
        .expect("generation sinks")
        .get("process-1")
        .cloned()
        .expect("failed sink")
        .handle(GenerationProcessEvent::Failed(
            GenerationProcessFailure::retryable("provider failed"),
        ))
        .expect("fail");
    let failed_terminal = failed
        .terminal
        .recv_timeout(std::time::Duration::ZERO)
        .expect("failed terminal");
    assert_eq!(failed_terminal.outcome, AgentMessageTerminalOutcome::Failed);
    assert_eq!(
        failed_world
            .messages
            .lock()
            .expect("messages")
            .get(&failed_terminal.message_id)
            .expect("persisted failed")
            .status,
        "failed"
    );

    let cancelled_world = test_world();
    let cancelled_service = service(cancelled_world.clone());
    let cancelled = cancelled_service
        .send_message_with_completion(SendMessageRequest {
            source: AgentMessageSource::InstantMessage {
                connector_id: "managed-im".to_string(),
            },
            session_id: "session-1".to_string(),
            content: "cancel this".to_string(),
            configuration: chat_configuration(),
            file_references: Vec::new(),
        })
        .expect("start cancelled generation");
    *cancelled_world
        .streaming_message_ids
        .lock()
        .expect("streaming ids") = vec![cancelled.message.id.clone()];
    cancelled_service
        .stop_generation("session-1")
        .expect("cancel generation");
    let cancelled_terminal = cancelled
        .terminal
        .recv_timeout(std::time::Duration::ZERO)
        .expect("cancelled terminal");
    assert_eq!(
        cancelled_terminal.outcome,
        AgentMessageTerminalOutcome::Cancelled
    );
}

#[test]
fn recovery_releases_a_stuck_generation_and_leaves_the_session_idle() {
    let world = test_world();
    let service = service(world.clone());
    let started = service
        .send_message_with_completion(SendMessageRequest {
            source: AgentMessageSource::Desktop,
            session_id: "session-1".to_string(),
            content: "this one gets stuck".to_string(),
            configuration: chat_configuration(),
            file_references: Vec::new(),
        })
        .expect("start generation");
    *world.streaming_message_ids.lock().expect("streaming ids") = vec![started.message.id.clone()];

    let recovered = service.recover_session("session-1").expect("recover");

    assert_eq!(recovered.lifecycle, AgentLifecycle::Idle);
    assert!(recovered
        .cancelled_message_ids
        .contains(&started.message.id));
    // Idle, not Starting: recovery restores a session that accepts the next message, it does not
    // spend the user's budget relaunching an Agent they did not ask for.
    assert_eq!(
        world
            .lifecycle_updates
            .lock()
            .expect("lifecycle updates")
            .last(),
        Some(&AgentLifecycle::Idle)
    );
}

#[test]
fn recovery_is_idempotent_and_refuses_archived_sessions() {
    let world = test_world();
    let service = service(world.clone());

    let quiet = service.recover_session("session-1").expect("recover quiet");
    assert!(quiet.cancelled_message_ids.is_empty());
    assert!(!quiet.process_stopped);
    assert_eq!(quiet.lifecycle, AgentLifecycle::Idle);

    world
        .sessions
        .lock()
        .expect("sessions")
        .get_mut("session-1")
        .expect("seeded session")
        .archived = true;
    assert!(matches!(
        service.recover_session("session-1"),
        Err(AgentRuntimeApplicationError::Validation(_))
    ));
}

#[test]
fn normalized_tool_lifecycle_deduplicates_and_marks_missing_boundaries() {
    let world = test_world();
    let (service, telemetry) = service_with_telemetry(world.clone());
    let message = service
        .send_message(SendMessageRequest {
            source: AgentMessageSource::Desktop,
            session_id: "session-1".to_string(),
            content: "observe tools".to_string(),
            configuration: chat_configuration(),
            file_references: Vec::new(),
        })
        .expect("send");
    let sink = world
        .generation_sinks
        .lock()
        .expect("generation sinks")
        .get("process-1")
        .cloned()
        .expect("sink");
    let event = |call_id: &str, phase: ToolLifecyclePhase, status: &str| {
        GenerationProcessEvent::ToolLifecycle(ToolLifecycleEvent {
            call_id: call_id.to_string(),
            phase,
            provider_timestamp: None,
            fidelity: ExecutionFidelity::Inferred,
            parent_run_id: None,
            parent_trace_id: None,
            parent_span_id: None,
            delegation_id: None,
            attempt: None,
            tool_use: ToolUseBlock {
                id: call_id.to_string(),
                name: "read".to_string(),
                input: None,
                output: None,
                status: status.to_string(),
                skill_provenance: None,
            },
        })
    };

    sink.handle(event(
        "completion-only",
        ToolLifecyclePhase::Completed,
        "completed",
    ))
    .expect("completion-only");
    sink.handle(event(
        "completion-only",
        ToolLifecyclePhase::Started,
        "running",
    ))
    .expect("late start");
    sink.handle(event("duplicate", ToolLifecyclePhase::Started, "running"))
        .expect("start");
    sink.handle(event("duplicate", ToolLifecyclePhase::Started, "running"))
        .expect("duplicate start");
    sink.handle(event("duplicate", ToolLifecyclePhase::Failed, "failed"))
        .expect("failed");
    sink.handle(event("unfinished", ToolLifecyclePhase::Started, "running"))
        .expect("unfinished");
    sink.handle(GenerationProcessEvent::Completed(None))
        .expect("agent complete");

    let records = telemetry.records().expect("telemetry records");
    let tool_spans = records
        .iter()
        .filter_map(|record| match record {
            CapturedTelemetryRecord::SpanStarted(span)
                if span.name.starts_with("execute_tool ") =>
            {
                Some(span)
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(tool_spans.len(), 3);
    assert_eq!(tool_spans[0].fidelity, ExecutionFidelity::Opaque);
    assert!(records.iter().any(|record| matches!(
        record,
        CapturedTelemetryRecord::SpanFinished {
            status: ExecutionStatus::Failed,
            error_classification: Some(classification),
            ..
        } if classification == "provider_tool_failed"
    )));
    assert!(records.iter().any(|record| matches!(
        record,
        CapturedTelemetryRecord::SpanFinished {
            status: ExecutionStatus::Incomplete,
            error_classification: Some(classification),
            ..
        } if classification == "provider_boundary_missing"
    )));
    assert_eq!(
        world.messages.lock().expect("messages")[&message.id]
            .tool_use
            .len(),
        4
    );
}

#[test]
fn streaming_tokens_are_coalesced_and_flushed_on_completion() {
    let world = test_world();
    let service = service(world.clone());
    let message = service
        .send_message(SendMessageRequest {
            source: AgentMessageSource::Desktop,
            session_id: "session-1".to_string(),
            content: "hello".to_string(),
            configuration: chat_configuration(),
            file_references: Vec::new(),
        })
        .expect("send");
    let sink = world
        .generation_sinks
        .lock()
        .expect("generation sinks")
        .get("process-1")
        .cloned()
        .expect("sink");
    let persisted_content = || {
        world.messages.lock().expect("messages")[&message.id]
            .content
            .clone()
    };

    sink.handle(GenerationProcessEvent::Token("alpha".to_string()))
        .expect("token");
    sink.handle(GenerationProcessEvent::Token("beta".to_string()))
        .expect("token");

    // Both small deltas arrive within the flush window, so persistence is coalesced
    // rather than one full-content rewrite per token (the O(N²) path we removed).
    assert!(
        persisted_content().len() < "alphabeta".len(),
        "streaming deltas must not be persisted per token, got {:?}",
        persisted_content()
    );

    sink.handle(GenerationProcessEvent::Completed(None))
        .expect("completed");

    // The terminal transition flushes the coalesced tail and the full content is durable.
    assert_eq!(persisted_content(), "alphabeta");
}

#[test]
fn stream_events_persist_complete_usage_and_operation_once() {
    let world = test_world();
    let service = service(world.clone());
    let message = service
        .send_message(SendMessageRequest {
            source: AgentMessageSource::Desktop,
            session_id: "session-1".to_string(),
            content: "hello".to_string(),
            configuration: chat_configuration(),
            file_references: Vec::new(),
        })
        .expect("send");
    let sink = world
        .generation_sinks
        .lock()
        .expect("generation sinks")
        .get("process-1")
        .cloned()
        .expect("sink");

    sink.handle(GenerationProcessEvent::RuntimeSessionId(
        "provider-session".to_string(),
    ))
    .expect("session id");
    sink.handle(GenerationProcessEvent::Token("first".to_string()))
        .expect("first token");
    sink.handle(GenerationProcessEvent::Token("second".to_string()))
        .expect("second token");
    sink.handle(GenerationProcessEvent::Thinking("plan".to_string()))
        .expect("thinking");
    sink.handle(GenerationProcessEvent::ToolUse(ToolUseBlock {
        id: "tool-1".to_string(),
        name: "read".to_string(),
        input: Some(serde_json::json!({"path":"README.md"})),
        output: None,
        status: "running".to_string(),
        skill_provenance: None,
    }))
    .expect("tool");
    sink.handle(GenerationProcessEvent::RichBlock(
        serde_json::json!({"id":"card-1","kind":"card","v":1}),
    ))
    .expect("rich block");
    sink.handle(GenerationProcessEvent::Stderr(
        "provider warning".to_string(),
    ))
    .expect("stderr");
    let barrier = Arc::new(std::sync::Barrier::new(3));
    let first_sink = sink.clone();
    let first_barrier = barrier.clone();
    let first = std::thread::spawn(move || {
        first_barrier.wait();
        first_sink.handle(GenerationProcessEvent::Completed(None))
    });
    let second_sink = sink.clone();
    let second_barrier = barrier.clone();
    let second = std::thread::spawn(move || {
        second_barrier.wait();
        second_sink.handle(GenerationProcessEvent::Completed(None))
    });
    barrier.wait();
    first
        .join()
        .expect("first terminal thread")
        .expect("first terminal");
    second
        .join()
        .expect("second terminal thread")
        .expect("second terminal");
    sink.handle(GenerationProcessEvent::Failed(
        GenerationProcessFailure::retryable("late failure must be ignored"),
    ))
    .expect("late terminal");

    let completed = world
        .messages
        .lock()
        .expect("messages")
        .get(&message.id)
        .cloned()
        .expect("completed message");
    assert_eq!(completed.status, "completed");
    assert_eq!(completed.content, "firstsecond");
    assert_eq!(completed.thinking_content.as_deref(), Some("plan"));
    assert_eq!(completed.tool_use.len(), 1);
    assert_eq!(completed.rich_blocks.len(), 1);
    assert_eq!(
        completed.token_usage,
        Some(MessageTokenUsage {
            input: "effective::hello\nfiles=0".chars().count() as i64,
            output: "firstsecond".chars().count() as i64,
        })
    );
    assert_eq!(
        world.sessions.lock().expect("sessions")["session-1"]
            .runtime_session_id
            .as_deref(),
        Some("provider-session")
    );
    assert!(world
        .active_generation
        .lock()
        .expect("active generation")
        .is_none());
    assert!(world
        .operations
        .lock()
        .expect("operations")
        .contains(&OperationEvent::Completed(
            "generation-operation-1".to_string()
        )));
    assert_eq!(
        world
            .operations
            .lock()
            .expect("operations")
            .iter()
            .filter(|event| matches!(event, OperationEvent::Completed(_)))
            .count(),
        1
    );
    let prompt_reports = world.prompt_reports.lock().expect("prompt reports");
    assert_eq!(prompt_reports.len(), 1);
    assert_eq!(
        prompt_reports[0].versions,
        [
            PromptVersionReference {
                hook_id: "system-context".to_string(),
                version: 1,
            },
            PromptVersionReference {
                hook_id: "review-focus".to_string(),
                version: 2,
            }
        ]
    );
    assert_eq!(prompt_reports[0].outcome, PromptExecutionOutcome::Succeeded);
    drop(prompt_reports);
    assert_eq!(
        world
            .logs
            .lock()
            .expect("logs")
            .last()
            .unwrap()
            .operation_id
            .as_deref(),
        Some("generation-operation-1")
    );
}
