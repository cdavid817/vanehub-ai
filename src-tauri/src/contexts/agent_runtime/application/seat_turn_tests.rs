use super::tests::{seat_turn_world, service};
use super::{SeatTurnAssignment, SeatTurnStop, SeatTurnTerminal};
use crate::contexts::agent_runtime::domain::ChainEndReason;

fn terminal(reply: Option<&str>, speaker: &str, depth: usize) -> SeatTurnTerminal {
    SeatTurnTerminal {
        session_id: "session-1".to_string(),
        message_id: "message-1".to_string(),
        seat_index: 0,
        seat_mention: speaker.to_string(),
        depth,
        reply: reply.map(str::to_string),
    }
}

#[test]
fn a_line_leading_mention_routes_the_turn_to_that_seat() {
    let service = service(seat_turn_world());
    let decision = service
        .decide_seat_turn(&terminal(Some("方案写好了。\n@代码审查 帮我看下"), "架构师", 1))
        .expect("decide");
    assert_eq!(
        decision.next,
        [SeatTurnAssignment {
            seat_index: 1,
            depth: 2,
        }]
    );
    assert_eq!(decision.stop, None);
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
    assert_eq!(
        decision.next,
        [SeatTurnAssignment {
            seat_index: 1,
            depth: 2,
        }]
    );
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
    let decision = service.decide_seat_turn(&terminal(None, "架构师", 1)).expect("decide");
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

/// A seat added mid-session has to act on work it never witnessed, so its first prompt carries the
/// thread rather than starting from nothing.
#[test]
fn a_seat_starting_its_turn_is_given_the_preceding_turns_attributed_by_speaker() {
    let world = seat_turn_world();
    world.seed_message("用户", None, "改下登录");
    world.seed_message("架构师", Some(0), "方案如下\n@实现者 你来写");
    let service = service(world.clone());

    service
        .start_seat_turn(
            "session-1",
            &SeatTurnAssignment {
                seat_index: 2,
                depth: 2,
            },
        )
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
        .start_seat_turn(
            "session-1",
            &SeatTurnAssignment {
                seat_index: 1,
                depth: 2,
            },
        )
        .expect("start seat turn");

    let requests = world.generation_requests.lock().expect("requests");
    let request = requests.last().expect("a generation was started");
    assert_eq!(request.agent.id, "codex-cli");
}

/// The briefing is the only channel through which an Agent learns who else is in the room and that
/// mentions route only at the start of a line.
#[test]
fn a_seat_turn_carries_the_roster_and_the_handoff_rules() {
    let world = seat_turn_world();
    let service = service(world.clone());

    service
        .start_seat_turn(
            "session-1",
            &SeatTurnAssignment {
                seat_index: 0,
                depth: 1,
            },
        )
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
        .start_seat_turn(
            "session-1",
            &SeatTurnAssignment {
                seat_index: 1,
                depth: 2,
            },
        )
        .expect("start seat turn");

    let created = world.created_messages.lock().expect("created messages");
    assert!(created.iter().all(|message| message.role != "user"));
    let assistant = created
        .iter()
        .find(|message| message.role == "assistant")
        .expect("an assistant message");
    assert_eq!(assistant.seat_index, Some(1));
}

/// Removing a seat has to stop it being invoked, including by a turn already queued for it.
#[test]
fn starting_a_turn_for_a_removed_seat_is_rejected() {
    let world = seat_turn_world();
    let service = service(world.clone());
    assert!(service
        .start_seat_turn(
            "session-1",
            &SeatTurnAssignment {
                seat_index: 9,
                depth: 2,
            },
        )
        .is_err());
}
