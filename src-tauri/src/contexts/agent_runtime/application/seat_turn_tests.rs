use super::tests::{seat_turn_world, service};
use super::{
    AgentChatConfiguration, AgentMessageSource, GenerationProcessEvent, SeatTurnAssignment,
    SeatTurnStatus, SeatTurnStop, SeatTurnTerminal, SendMessageRequest,
};
use crate::contexts::agent_runtime::domain::{
    AgentAvailability, AvailabilityAssessment, ChainEndReason, InteractionMode,
};
use crate::contexts::execution_observability::api::CapturedTelemetryRecord;

fn terminal(reply: Option<&str>, speaker: &str, depth: usize) -> SeatTurnTerminal {
    SeatTurnTerminal {
        source: AgentMessageSource::Desktop,
        session_id: "session-1".to_string(),
        message_id: "message-1".to_string(),
        seat_id: "seat-1".to_string(),
        seat_index: 0,
        seat_mention: speaker.to_string(),
        depth,
        round_id: "round-1".to_string(),
        execution_run_id: "run-1".to_string(),
        reply: reply.map(str::to_string),
    }
}

/// Marks a seat as having left, the way removing it from the roster does.
fn seat_leaves(world: &super::tests::FakeWorld, seat_index: usize) {
    let mut sessions = world.sessions.lock().expect("sessions");
    sessions.get_mut("session-1").expect("session").seats[seat_index].left_at =
        Some("2026-08-07T01:00:00+00:00".to_string());
}

fn agent_becomes_unavailable(world: &super::tests::FakeWorld, agent_id: &str) {
    let mut agents = world.agents.lock().expect("agents");
    let agent = agents
        .iter_mut()
        .find(|agent| agent.id().as_str() == agent_id)
        .expect("agent");
    *agent = agent.clone().with_availability(AvailabilityAssessment::new(
        AgentAvailability::Unavailable,
        Some("fixture unavailable".to_string()),
    ));
}

/// Rewrites the last seeded row into pre-migration-59 shape: the numeric index, no stable seat id.
fn make_last_message_legacy(world: &super::tests::FakeWorld, seat_index: usize) {
    let mut messages = world.messages.lock().expect("messages");
    let row = messages.values_mut().next_back().expect("a seeded message");
    row.speaker_seat_id = None;
    row.seat_index = Some(seat_index);
}

fn assignment(seat_index: usize, depth: usize) -> SeatTurnAssignment {
    SeatTurnAssignment {
        source: AgentMessageSource::Desktop,
        seat_id: format!("seat-{}", seat_index + 1),
        seat_index,
        depth,
        round_id: "round-1".to_string(),
        parent_execution_run_id: Some("run-1".to_string()),
    }
}

#[test]
fn a_line_leading_mention_routes_the_turn_to_that_seat() {
    let service = service(seat_turn_world());
    let decision = service
        .decide_seat_turn(&terminal(
            Some("方案写好了。\n@代码审查 帮我看下"),
            "架构师",
            1,
        ))
        .expect("decide");
    assert_eq!(decision.next, [assignment(1, 2)]);
    assert_eq!(decision.stop, None);
}

#[test]
fn serial_handoff_keeps_one_round_and_allocates_a_new_child_run() {
    let world = seat_turn_world();
    let service = service(world.clone());
    let first = service
        .send_message(SendMessageRequest {
            source: AgentMessageSource::Desktop,
            session_id: "session-1".to_string(),
            content: "开始".to_string(),
            configuration: AgentChatConfiguration {
                agent_id: "claude-code".to_string(),
                interaction_mode: InteractionMode::Cli,
                execution_mode: "inherit".to_string(),
                provider_id: None,
                model_id: None,
                reasoning_depth: None,
                streaming: true,
                thinking: false,
                long_context: false,
            },
            file_references: Vec::new(),
        })
        .expect("start first seat");
    let first_run = first.execution_run_id.clone().expect("first run");
    let first_sink = world
        .generation_sinks
        .lock()
        .expect("generation sinks")
        .get("process-1")
        .cloned()
        .expect("first sink");
    first_sink
        .handle(GenerationProcessEvent::Token("@代码审查 继续".to_string()))
        .expect("first reply");
    first_sink
        .handle(GenerationProcessEvent::Completed(None))
        .expect("complete first seat");

    let terminal = service
        .take_seat_turn_completion("session-1")
        .expect("take terminal")
        .expect("terminal");
    assert_eq!(terminal.execution_run_id, first_run);
    let assignment = service
        .decide_seat_turn(&terminal)
        .expect("decide")
        .next
        .into_iter()
        .next()
        .expect("next seat");
    assert_eq!(assignment.round_id, terminal.round_id);
    assert_eq!(
        assignment.parent_execution_run_id.as_deref(),
        Some(first_run.as_str())
    );

    service
        .start_seat_turn("session-1", &assignment)
        .expect("start second seat");
    let second_run = service
        .active_generation_correlation("session-1")
        .expect("active correlation")
        .and_then(|correlation| correlation.execution_run_id)
        .expect("second run");
    assert_ne!(second_run, first_run);
}

#[test]
fn a_mention_inside_a_sentence_does_not_route() {
    let service = service(seat_turn_world());
    let decision = service
        .decide_seat_turn(&terminal(Some("做完了，让 @代码审查 看一下"), "架构师", 1))
        .expect("decide");
    assert!(decision.next.is_empty());
    assert_eq!(decision.stop, Some(SeatTurnStop::NobodyMentioned));
}

/// A chain that simply runs out of mentions is a finished round, not a failure.
#[test]
fn a_reply_naming_nobody_ends_the_round_quietly() {
    let service = service(seat_turn_world());
    let decision = service
        .decide_seat_turn(&terminal(Some("做完了。"), "架构师", 1))
        .expect("decide");
    assert_eq!(decision.stop, Some(SeatTurnStop::NobodyMentioned));
}

#[test]
fn the_chain_depth_limit_stops_the_round_and_says_why() {
    let service = service(seat_turn_world());
    let decision = service
        .decide_seat_turn(&terminal(Some("@代码审查 继续"), "架构师", 15))
        .expect("decide");
    assert!(decision.next.is_empty());
    assert_eq!(
        decision.stop,
        Some(SeatTurnStop::Bounded(ChainEndReason::MaxDepth))
    );
}

/// Only a blocking handoff interrupts. A single blocking "notify the human" action would teach
/// Agents to stop notifying, which is the visibility loss the three intents exist to prevent.
#[test]
fn an_informational_handoff_leaves_the_turn_with_the_agents() {
    let service = service(seat_turn_world());
    let decision = service
        .decide_seat_turn(&terminal(
            Some("@用户 fyi 顺带一提\n@代码审查 接着看"),
            "架构师",
            1,
        ))
        .expect("decide");
    assert_eq!(decision.next, [assignment(1, 2)]);
    assert_eq!(decision.stop, None);
}

#[test]
fn a_blocking_handoff_stops_the_round_even_when_a_teammate_is_mentioned() {
    let service = service(seat_turn_world());
    let decision = service
        .decide_seat_turn(&terminal(
            Some("@用户 handoff 你定一下\n@代码审查 之后再看"),
            "架构师",
            1,
        ))
        .expect("decide");
    assert!(decision.next.is_empty());
    assert_eq!(decision.stop, Some(SeatTurnStop::AwaitingHuman));
}

#[test]
fn a_completion_handoff_ends_the_round() {
    let service = service(seat_turn_world());
    let decision = service
        .decide_seat_turn(&terminal(Some("@用户 done 完成"), "架构师", 1))
        .expect("decide");
    assert_eq!(decision.stop, Some(SeatTurnStop::RoundComplete));
}

/// Chaining past a failure would hand the next seat a turn whose premise never happened.
#[test]
fn a_failed_turn_ends_the_chain() {
    let service = service(seat_turn_world());
    let decision = service
        .decide_seat_turn(&terminal(None, "架构师", 1))
        .expect("decide");
    assert!(decision.next.is_empty());
    assert_eq!(decision.stop, Some(SeatTurnStop::TurnFailed));
}

#[test]
fn a_seat_cannot_hand_the_turn_back_to_itself() {
    let service = service(seat_turn_world());
    let decision = service
        .decide_seat_turn(&terminal(Some("@架构师 继续"), "架构师", 1))
        .expect("decide");
    assert!(decision.next.is_empty());
}

#[test]
fn a_reply_naming_more_seats_than_allowed_is_truncated_with_a_reason() {
    let world = seat_turn_world();
    let service = service(world);
    let decision = service
        .decide_seat_turn(&terminal(
            Some("@代码审查 a\n@实现者 b\n@测试 c"),
            "架构师",
            1,
        ))
        .expect("decide");
    assert_eq!(decision.next.len(), 2);
    assert_eq!(
        decision.stop,
        Some(SeatTurnStop::Bounded(ChainEndReason::TooManyMentions))
    );
}

/// A seat's handle comes from its role, and every seat must be addressable by exactly one handle.
#[test]
fn the_roster_names_every_seat() {
    let service = service(seat_turn_world());
    let session = service.require_session("session-1").expect("session");
    let roster = service.seat_roster(&session).expect("roster");
    let mentions: Vec<&str> = roster
        .iter()
        .map(|entry| entry.briefing.mention.as_str())
        .collect();
    assert_eq!(mentions, ["架构师", "代码审查", "实现者", "测试"]);
}

#[test]
fn a_single_seat_session_is_not_coordinated() {
    let service = service(super::tests::test_world());
    assert!(!service.is_multi_seat_session("session-1"));
}

#[test]
fn a_multi_seat_session_is_coordinated() {
    let service = service(seat_turn_world());
    assert!(service.is_multi_seat_session("session-1"));
}

#[test]
fn the_first_reply_owns_the_first_stable_seat_and_receives_its_briefing() {
    let service = service(seat_turn_world());
    let session = service.require_session("session-1").expect("session");
    let source = AgentMessageSource::InstantMessage {
        connector_id: "feishu".to_string(),
    };
    let (_, ownership, briefing) = service
        .initial_seat_turn_context(&session, "开始吧", &source)
        .expect("initial context")
        .expect("multi-seat context");

    assert_eq!(ownership.source, source);
    assert_eq!(ownership.seat_id, "seat-1");
    assert_eq!(ownership.seat_index, 0);
    assert_eq!(ownership.seat_mention, "架构师");
    assert_eq!(ownership.depth, 1);
    assert!(briefing.starts_with("你是架构师。"));
    assert!(briefing.contains("@代码审查"));
}

/// The human addresses a seat the way the Agents address each other. Before this, every user
/// message went to the first seat, so a group chat had exactly one answerable participant.
#[test]
fn a_user_message_is_answered_by_the_seat_it_names() {
    let service = service(seat_turn_world());
    let session = service.require_session("session-1").expect("session");
    let (seat, ownership, briefing) = service
        .initial_seat_turn_context(
            &session,
            "@实现者 按方案写一版",
            &AgentMessageSource::Desktop,
        )
        .expect("initial context")
        .expect("multi-seat context");

    assert_eq!(ownership.seat_id, "seat-3");
    assert_eq!(ownership.seat_mention, "实现者");
    // The seat runs its own Agent rather than the one the session mirrors, which is what keeps the
    // reply from being answered by one participant under another's name.
    assert_eq!(seat.agent_id, "gemini-cli");
    assert!(briefing.starts_with("你是实现者。"));
    // Its own handle is not in the roster it is given: a seat cannot hand off to itself.
    assert!(!briefing.contains("@实现者"));
    assert!(briefing.contains("@架构师"));
}

#[test]
fn an_unaddressed_user_message_continues_with_the_seat_that_last_spoke() {
    let world = seat_turn_world();
    world.seed_message("用户", None, "改下登录");
    world.seed_message("代码审查", Some(1), "有两处要改");
    let service = service(world);
    let session = service.require_session("session-1").expect("session");
    let source = AgentMessageSource::InstantMessage {
        connector_id: "feishu".to_string(),
    };
    let (_, ownership, _) = service
        .initial_seat_turn_context(&session, "继续", &source)
        .expect("initial context")
        .expect("multi-seat context");

    assert_eq!(ownership.source, source);
    assert_eq!(ownership.seat_id, "seat-2");
    assert_eq!(ownership.seat_mention, "代码审查");
}

/// A thread that predates migration 59 has only the numeric index, and still has to route.
#[test]
fn a_thread_attributed_only_by_index_still_names_its_last_speaker() {
    let world = seat_turn_world();
    world.seed_message("代码审查", Some(1), "有两处要改");
    make_last_message_legacy(&world, 1);
    let service = service(world);
    let session = service.require_session("session-1").expect("session");
    let (_, ownership, _) = service
        .initial_seat_turn_context(&session, "继续", &AgentMessageSource::Desktop)
        .expect("initial context")
        .expect("multi-seat context");

    assert_eq!(ownership.seat_id, "seat-2");
}

#[test]
fn a_user_message_naming_a_departed_seat_is_rejected_with_valid_mentions() {
    let world = seat_turn_world();
    seat_leaves(&world, 2);
    let service = service(world);
    let session = service.require_session("session-1").expect("session");
    let error = service
        .initial_seat_turn_context(
            &session,
            "@实现者 按方案写一版",
            &AgentMessageSource::Desktop,
        )
        .expect_err("departed seat must not reroute");

    assert_eq!(
        error,
        super::AgentRuntimeApplicationError::InvalidSeatMention {
            valid_mentions: vec![
                "架构师".to_string(),
                "代码审查".to_string(),
                "测试".to_string(),
            ],
        }
    );
}

#[test]
fn a_user_message_naming_an_unavailable_seat_is_rejected_with_other_valid_mentions() {
    let world = seat_turn_world();
    agent_becomes_unavailable(&world, "gemini-cli");
    let service = service(world);
    let session = service.require_session("session-1").expect("session");
    let error = service
        .initial_seat_turn_context(
            &session,
            "@实现者 按方案写一版",
            &AgentMessageSource::Desktop,
        )
        .expect_err("unavailable seat must not reroute");

    assert!(matches!(
        error,
        super::AgentRuntimeApplicationError::InvalidSeatMention { valid_mentions }
            if valid_mentions == ["架构师", "代码审查", "测试"]
    ));
}

/// A seat added mid-session has to act on work it never witnessed, so its first prompt carries the
/// thread rather than starting from nothing.
#[test]
fn a_seat_starting_its_turn_is_given_the_preceding_turns_attributed_by_speaker() {
    let world = seat_turn_world();
    world.seed_message("用户", None, "改下登录");
    world.seed_message("架构师", Some(0), "方案如下\n@实现者 你来写");
    let service = service(world.clone());

    service
        .start_seat_turn("session-1", &assignment(2, 2))
        .expect("start seat turn");

    let requests = world.generation_requests.lock().expect("requests");
    let request = requests.last().expect("a generation was started");
    assert!(request.effective_prompt.contains("[用户 说] 改下登录"));
    assert!(request.effective_prompt.contains("[架构师 说] 方案如下"));
    assert!(request.effective_prompt.contains("轮到你（@实现者）发言"));
}

/// The seat's own Agent runs the turn, not the session's mirrored one.
#[test]
fn a_seat_turn_runs_the_seats_own_agent() {
    let world = seat_turn_world();
    let service = service(world.clone());

    service
        .start_seat_turn("session-1", &assignment(1, 2))
        .expect("start seat turn");

    let requests = world.generation_requests.lock().expect("requests");
    let request = requests.last().expect("a generation was started");
    assert_eq!(request.agent.id, "codex-cli");
}

/// End of the send path, not just the routing decision: an addressed message has to reach the
/// named seat's Agent. The session mirrors its first seat, so invoking `session.agent_id` here
/// would run Claude Code and label the reply as Gemini's seat.
#[test]
fn a_user_message_naming_a_seat_runs_that_seats_agent() {
    let world = seat_turn_world();
    let service = service(world.clone());

    service
        .send_message(SendMessageRequest {
            source: AgentMessageSource::Desktop,
            session_id: "session-1".to_string(),
            content: "@实现者 按方案写一版".to_string(),
            configuration: AgentChatConfiguration {
                agent_id: "claude-code".to_string(),
                interaction_mode: InteractionMode::Cli,
                execution_mode: "inherit".to_string(),
                provider_id: None,
                model_id: None,
                reasoning_depth: None,
                streaming: true,
                thinking: false,
                long_context: false,
            },
            file_references: Vec::new(),
        })
        .expect("send");

    let requests = world.generation_requests.lock().expect("requests");
    let request = requests.last().expect("a generation was started");
    assert_eq!(request.agent.id, "gemini-cli");
}

#[test]
fn a_feishu_mention_and_handoff_keep_stable_seats_and_im_origin() {
    let world = seat_turn_world();
    let service = service(world.clone());
    let source = AgentMessageSource::InstantMessage {
        connector_id: "feishu".to_string(),
    };

    service
        .send_message(SendMessageRequest {
            source: source.clone(),
            session_id: "session-1".to_string(),
            content: "@实现者 按方案写一版".to_string(),
            configuration: AgentChatConfiguration {
                agent_id: "claude-code".to_string(),
                interaction_mode: InteractionMode::Cli,
                execution_mode: "inherit".to_string(),
                provider_id: None,
                model_id: None,
                reasoning_depth: None,
                streaming: true,
                thinking: false,
                long_context: false,
            },
            file_references: Vec::new(),
        })
        .expect("start mentioned Feishu seat");

    let first_request = world.generation_requests.lock().expect("requests")[0].clone();
    assert_eq!(first_request.agent.id, "gemini-cli");
    let first_sink = world
        .generation_sinks
        .lock()
        .expect("generation sinks")
        .get("process-1")
        .cloned()
        .expect("first sink");
    first_sink
        .handle(GenerationProcessEvent::Token(
            "@代码审查 请复核".to_string(),
        ))
        .expect("handoff reply");
    first_sink
        .handle(GenerationProcessEvent::Completed(None))
        .expect("complete first seat");

    let terminal = service
        .take_seat_turn_completion("session-1")
        .expect("take completion")
        .expect("seat completion");
    assert_eq!(terminal.source, source);
    assert_eq!(terminal.seat_id, "seat-3");
    let next = service
        .decide_seat_turn(&terminal)
        .expect("route handoff")
        .next
        .into_iter()
        .next()
        .expect("next seat");
    assert_eq!(next.source, source);
    assert_eq!(next.seat_id, "seat-2");
    assert_eq!(next.depth, 2);
    assert_eq!(next.round_id, terminal.round_id);

    service
        .start_seat_turn("session-1", &next)
        .expect("start handed-off seat");
    let requests = world.generation_requests.lock().expect("requests");
    assert_eq!(requests.len(), 2);
    assert_eq!(requests[1].agent.id, "codex-cli");
    assert!(!requests[1].interactive);
}

#[test]
fn a_multi_agent_im_completion_delivers_only_the_terminal_seat_reply() {
    let world = seat_turn_world();
    let service = service(world.clone());
    let started = service
        .send_message_with_completion(SendMessageRequest {
            source: AgentMessageSource::InstantMessage {
                connector_id: "feishu".to_string(),
            },
            session_id: "session-1".to_string(),
            content: "@实现者 按方案写一版".to_string(),
            configuration: AgentChatConfiguration {
                agent_id: "claude-code".to_string(),
                interaction_mode: InteractionMode::Cli,
                execution_mode: "inherit".to_string(),
                provider_id: None,
                model_id: None,
                reasoning_depth: None,
                streaming: true,
                thinking: false,
                long_context: false,
            },
            file_references: Vec::new(),
        })
        .expect("start IM round");

    let first_sink = world
        .generation_sinks
        .lock()
        .expect("generation sinks")
        .get("process-1")
        .cloned()
        .expect("first sink");
    first_sink
        .handle(GenerationProcessEvent::Token(
            "@代码审查 请复核".to_string(),
        ))
        .expect("handoff reply");
    first_sink
        .handle(GenerationProcessEvent::Completed(None))
        .expect("complete first seat");
    let first_terminal = service
        .take_seat_turn_completion("session-1")
        .expect("take first completion")
        .expect("first terminal");
    let next = service
        .decide_seat_turn(&first_terminal)
        .expect("route next seat")
        .next
        .into_iter()
        .next()
        .expect("next seat");
    service
        .start_seat_turn("session-1", &next)
        .expect("start final seat");

    let final_sink = world
        .generation_sinks
        .lock()
        .expect("generation sinks")
        .get("process-1")
        .cloned()
        .expect("final sink");
    final_sink
        .handle(GenerationProcessEvent::Token("最终答复".to_string()))
        .expect("final reply");
    final_sink
        .handle(GenerationProcessEvent::Completed(None))
        .expect("complete final seat");
    let final_terminal = service
        .take_seat_turn_completion("session-1")
        .expect("take final completion")
        .expect("final terminal");
    let final_decision = service
        .decide_seat_turn(&final_terminal)
        .expect("finish round");
    assert!(final_decision.next.is_empty());
    service
        .complete_seat_round(&final_terminal)
        .expect("deliver final completion");

    let delivered = started
        .terminal
        .recv_timeout(std::time::Duration::ZERO)
        .expect("terminal-only completion");
    assert_ne!(delivered.message_id, started.message.id);
    assert_eq!(delivered.message_id, final_terminal.message_id);
    assert_eq!(
        delivered.outcome,
        super::AgentMessageTerminalOutcome::Completed
    );
    assert_eq!(delivered.content.as_deref(), Some("最终答复"));
}

/// The briefing is the only channel through which an Agent learns who else is in the room and that
/// mentions route only at the start of a line.
#[test]
fn a_seat_turn_carries_the_roster_and_the_handoff_rules() {
    let world = seat_turn_world();
    let service = service(world.clone());

    service
        .start_seat_turn("session-1", &assignment(0, 1))
        .expect("start seat turn");

    let requests = world.generation_requests.lock().expect("requests");
    let briefing = requests
        .last()
        .expect("a generation was started")
        .role_briefing
        .as_deref()
        .expect("a multi-seat turn carries a briefing");
    assert!(briefing.starts_with("你是架构师。"));
    assert!(briefing.contains("@代码审查"));
    assert!(briefing.contains("行首"));
    // Its own handle must not appear in the roster it reads, or it will hand off to itself.
    assert!(!briefing.contains("@架构师"));
}

/// A handoff prompt is written by the runtime. Recording it as a user message would put words the
/// human never typed into the thread under their name.
#[test]
fn a_seat_turn_records_no_user_message() {
    let world = seat_turn_world();
    let service = service(world.clone());

    service
        .start_seat_turn("session-1", &assignment(1, 2))
        .expect("start seat turn");

    let created = world.created_messages.lock().expect("created messages");
    assert!(created.iter().all(|message| message.role != "user"));
    let assistant = created
        .iter()
        .find(|message| message.role == "assistant")
        .expect("an assistant message");
    assert_eq!(assistant.speaker_seat_id.as_deref(), Some("seat-2"));
    assert_eq!(assistant.seat_index, None);
}

/// Removing a seat has to stop it being invoked, including by a turn already queued for it.
#[test]
fn starting_a_turn_for_a_removed_seat_is_rejected() {
    let world = seat_turn_world();
    let service = service(world.clone());
    assert!(service
        .start_seat_turn("session-1", &assignment(9, 2),)
        .is_err());
}

/// The execution trace stays session-scoped and shows a whole round including the handoffs, so it
/// distinguishes seats by marking each Agent span with the seat that ran it.
#[test]
fn a_seat_turn_marks_its_agent_span_with_the_seat() {
    use crate::contexts::execution_observability::api::SafeAttributeValue;

    let world = seat_turn_world();
    let (service, telemetry) = super::tests::service_with_telemetry(world);

    service
        .start_seat_turn("session-1", &assignment(1, 2))
        .expect("start seat turn");

    let agent_span = telemetry
        .records()
        .expect("records")
        .into_iter()
        .find_map(|record| match record {
            CapturedTelemetryRecord::SpanStarted(span) if span.name.starts_with("invoke_agent") => {
                Some(span)
            }
            _ => None,
        })
        .expect("an agent span");
    assert_eq!(
        agent_span.attributes.entries().get("vanehub.seat.index"),
        Some(&SafeAttributeValue::String("1".to_string()))
    );
    assert_eq!(
        agent_span.attributes.entries().get("vanehub.seat.mention"),
        Some(&SafeAttributeValue::String("代码审查".to_string()))
    );
}

/// The one question a reader of a multi-seat session has is who they are waiting on, so starting a
/// seat's turn has to announce it.
#[test]
fn starting_a_seat_turn_announces_who_holds_it() {
    let world = seat_turn_world();
    let service = service(world.clone());

    service
        .start_seat_turn("session-1", &assignment(1, 3))
        .expect("start seat turn");

    let events = world.events.lock().expect("events");
    let status = events
        .iter()
        .find_map(|event| match event {
            crate::contexts::agent_runtime::application::AgentEvent::TurnStatusChanged {
                status,
                ..
            } => Some(status.clone()),
            _ => None,
        })
        .expect("a turn status event");
    assert_eq!(
        status,
        SeatTurnStatus::Agent {
            seat_id: "seat-2".to_string(),
            seat_index: 1,
            mention: "代码审查".to_string(),
            depth: 3,
            max_depth: 15,
        }
    );
}

/// A paused round has to say how long it has been paused, which starts from the moment it pauses.
#[test]
fn a_blocking_handoff_announces_that_the_turn_is_waiting_on_the_user() {
    let world = seat_turn_world();
    let service = service(world.clone());

    service
        .decide_seat_turn(&terminal(Some("@用户 handoff 你定一下"), "架构师", 1))
        .expect("decide");

    let events = world.events.lock().expect("events");
    let status = events
        .iter()
        .find_map(|event| match event {
            crate::contexts::agent_runtime::application::AgentEvent::TurnStatusChanged {
                status,
                ..
            } => Some(status.clone()),
            _ => None,
        })
        .expect("a turn status event");
    match status {
        SeatTurnStatus::WaitingHuman { mention, since, .. } => {
            assert_eq!(mention, "架构师");
            assert!(
                !since.is_empty(),
                "waiting has to start from a known moment"
            );
        }
        other => panic!("expected a waiting status, got {other:?}"),
    }
}

/// An informational handoff must not look like an interruption, or Agents get blamed for using it.
#[test]
fn an_informational_handoff_announces_no_pause() {
    let world = seat_turn_world();
    let service = service(world.clone());

    service
        .decide_seat_turn(&terminal(
            Some("@用户 fyi 顺带一提\n@代码审查 接着看"),
            "架构师",
            1,
        ))
        .expect("decide");

    let events = world.events.lock().expect("events");
    assert!(events.iter().all(|event| !matches!(
        event,
        crate::contexts::agent_runtime::application::AgentEvent::TurnStatusChanged {
            status: SeatTurnStatus::WaitingHuman { .. },
            ..
        }
    )));
}

#[test]
fn a_completion_handoff_announces_the_round_is_done() {
    let world = seat_turn_world();
    let service = service(world.clone());

    service
        .decide_seat_turn(&terminal(Some("@用户 done 完成"), "架构师", 1))
        .expect("decide");

    let events = world.events.lock().expect("events");
    assert!(events.iter().any(|event| matches!(
        event,
        crate::contexts::agent_runtime::application::AgentEvent::TurnStatusChanged {
            status: SeatTurnStatus::RoundComplete { .. },
            ..
        }
    )));
}
