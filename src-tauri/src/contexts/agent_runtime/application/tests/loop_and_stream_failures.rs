use super::*;

#[test]
fn loop_role_generation_delivers_one_terminal_completion_and_cancellation_wins_races() {
    for cancelled in [false, true] {
        let world = test_world();
        world
            .sessions
            .lock()
            .expect("sessions")
            .get_mut("session-1")
            .expect("session")
            .loop_ownership = Some(LoopRoleGenerationOwnership {
            run_id: "run-1".to_string(),
            iteration_id: "iteration-1".to_string(),
            role: "worker".to_string(),
        });
        let service = service(world.clone());
        let message = service
            .send_message(SendMessageRequest {
                source: AgentMessageSource::Desktop,
                session_id: "session-1".to_string(),
                content: "implement".to_string(),
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

        if cancelled {
            service.stop_generation("session-1").expect("cancel");
            sink.handle(GenerationProcessEvent::Failed(
                GenerationProcessFailure::retryable("late failure"),
            ))
            .expect("late failure ignored");
        } else {
            sink.handle(GenerationProcessEvent::Token("done".to_string()))
                .expect("token");
            sink.handle(GenerationProcessEvent::Completed(None))
                .expect("complete");
            sink.handle(GenerationProcessEvent::Completed(None))
                .expect("duplicate complete ignored");
        }

        let terminal = service
            .take_loop_role_completion("session-1")
            .expect("take")
            .expect("terminal");
        assert_eq!(terminal.run_id, "run-1");
        assert_eq!(terminal.iteration_id, "iteration-1");
        assert_eq!(terminal.message_id, message.id);
        assert_eq!(
            terminal.outcome,
            if cancelled {
                LoopRoleGenerationOutcome::Cancelled
            } else {
                LoopRoleGenerationOutcome::Completed
            }
        );
        assert_eq!(terminal.content.as_deref(), (!cancelled).then_some("done"));
        assert_eq!(
            service
                .take_loop_role_completion("session-1")
                .expect("second take"),
            None
        );
    }
}

#[test]
fn loop_role_generation_for_an_api_agent_session_resolves_api_interaction_mode() {
    let world = test_world();
    world.agents.lock().expect("agents").push(api_agent(
        "trusted-api-agent",
        "Trusted API Agent",
        vec!["coding"],
    ));
    world.sessions.lock().expect("sessions").insert(
        "session-api-1".to_string(),
        AgentSession {
            id: "session-api-1".to_string(),
            agent_id: "trusted-api-agent".to_string(),
            seats: Vec::new(),
            interaction_mode: InteractionMode::Api,
            lifecycle: AgentLifecycle::Idle,
            folder: Some("C:/workspace".to_string()),
            runtime_session_id: None,
            archived: false,
            read_only: false,
            loop_ownership: Some(LoopRoleGenerationOwnership {
                run_id: "run-1".to_string(),
                iteration_id: "iteration-1".to_string(),
                role: "worker".to_string(),
            }),
        },
    );
    let service = service(world.clone());

    service
        .start_worker_generation("session-api-1", "implement")
        .expect("start worker generation");

    let requests = world
        .generation_requests
        .lock()
        .expect("generation requests");
    assert_eq!(
        requests
            .last()
            .expect("request")
            .configuration
            .interaction_mode,
        InteractionMode::Api
    );
}

#[test]
fn stream_failure_uses_safe_message_and_keeps_diagnostic_in_associated_log() {
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

    sink.handle(GenerationProcessEvent::Failed(
        GenerationProcessFailure::retryable("provider diagnostic secret"),
    ))
    .expect("failed");

    let failed = world
        .messages
        .lock()
        .expect("messages")
        .get(&message.id)
        .cloned()
        .expect("failed message");
    assert_eq!(failed.status, "failed");
    assert_eq!(failed.error.as_deref(), Some("Codex CLI command failed"));
    let log = world
        .logs
        .lock()
        .expect("logs")
        .last()
        .cloned()
        .expect("log");
    assert_eq!(log.message, "provider diagnostic secret");
    assert_eq!(log.operation_id.as_deref(), Some("generation-operation-1"));
    assert!(world
        .operations
        .lock()
        .expect("operations")
        .contains(&OperationEvent::Failed(
            "generation-operation-1".to_string()
        )));
    let prompt_reports = world.prompt_reports.lock().expect("prompt reports");
    assert_eq!(prompt_reports.len(), 1);
    assert_eq!(prompt_reports[0].invocation_id, "generation-operation-1");
    assert_eq!(prompt_reports[0].agent_id, "codex-cli");
    assert_eq!(prompt_reports[0].outcome, PromptExecutionOutcome::Failed);
    assert_eq!(prompt_reports[0].versions.len(), 2);
    assert!(prompt_reports[0].elapsed_ms >= 0);
}

#[test]
fn stream_failure_uses_provider_safe_error_without_exposing_diagnostic() {
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
    let safe_error =
        "Provider authentication failed. Check the API key in the active OnePiece configuration.";

    sink.handle(GenerationProcessEvent::Failed(
        GenerationProcessFailure::non_retryable("invalid secret credential")
            .with_safe_error(safe_error),
    ))
    .expect("failed");

    let failed = world
        .messages
        .lock()
        .expect("messages")
        .get(&message.id)
        .cloned()
        .expect("failed message");
    assert_eq!(failed.error.as_deref(), Some(safe_error));
    assert!(!failed
        .error
        .as_deref()
        .unwrap_or_default()
        .contains("secret"));
    let log = world
        .logs
        .lock()
        .expect("logs")
        .last()
        .cloned()
        .expect("log");
    assert_eq!(log.message, "invalid secret credential");
}

#[test]
fn prompt_failure_is_safe_terminal_and_stop_deduplicates_cancelled_events() {
    let failed_world = test_world();
    failed_world.prompt_failure.store(true, Ordering::SeqCst);
    let failed = service(failed_world.clone())
        .send_message(SendMessageRequest {
            source: AgentMessageSource::Desktop,
            session_id: "session-1".to_string(),
            content: "hello".to_string(),
            configuration: chat_configuration(),
            file_references: Vec::new(),
        })
        .expect("safe failed message");
    assert_eq!(failed.status, "failed");
    assert_eq!(failed.error.as_deref(), Some("Prompt Hook assembly failed"));
    assert!(failed_world
        .active_generation
        .lock()
        .expect("active generation")
        .is_none());

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
    *world.streaming_message_ids.lock().expect("streaming ids") =
        vec![message.id.clone(), "message-3".to_string()];
    let stopped = service.stop_generation("session-1").expect("stop");
    assert!(stopped.process_stopped);
    assert_eq!(
        stopped.cancelled_message_ids,
        vec!["message-2".to_string(), "message-3".to_string()]
    );
    let cancelled = world
        .events
        .lock()
        .expect("events")
        .iter()
        .filter_map(|event| match event {
            AgentEvent::MessageCancelled { message_id, .. } => Some(message_id.clone()),
            _ => None,
        })
        .collect::<BTreeSet<_>>();
    assert_eq!(
        cancelled,
        BTreeSet::from(["message-2".to_string(), "message-3".to_string()])
    );
    assert_eq!(
        *world.stopped_processes.lock().expect("stopped processes"),
        vec!["process-1".to_string()]
    );
    assert!(world
        .operations
        .lock()
        .expect("operations")
        .contains(&OperationEvent::Cancelled(
            "generation-operation-1".to_string()
        )));
    let prompt_reports = world.prompt_reports.lock().expect("prompt reports");
    assert_eq!(prompt_reports.len(), 1);
    assert_eq!(prompt_reports[0].outcome, PromptExecutionOutcome::Cancelled);
}
