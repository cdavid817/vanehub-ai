//! Serial turn taking for a multi-seat session.
//!
//! A completed reply has to be read for `@` mentions and, when one names a teammate, that seat has
//! to be invoked. The sink cannot do this itself: it holds ports rather than the service, and
//! starting a generation from inside a terminal handler would nest one generation's lifecycle
//! inside another's. The Loop runtime solved the same shape by having its sink deliver a terminal
//! and leaving the decision to a separate coordinator; seat turns follow that precedent.
//!
//! This module holds the decisions. Driving them — taking terminals off the queue and waiting for
//! each seat in turn — belongs to `infrastructure::seat_turn_coordinator`.

use super::service::MessageGenerationInput;
use super::{
    AgentChatConfiguration, AgentEvent, AgentRuntimeApplicationError,
    AgentRuntimeApplicationService, AgentSession, SeatTurnOwnership, SeatTurnStatus,
    SeatTurnTerminal,
};
use crate::contexts::agent_runtime::domain::{
    apply_human_handoff, build_seat_briefing, build_seat_context, derive_mentions,
    next_turn_targets, normalize_model_family, parse_human_handoff, route_user_message,
    ChainEndReason, SeatBriefingEntry, SeatContextMode, SeatTurn as SeatContextTurn,
};
use crate::contexts::agent_runtime::domain::{AgentDefinition, InteractionMode};
use uuid::Uuid;

/// Inherited defaults that have not been measured against this runtime, so they live here as
/// one obvious place to change rather than being threaded through configuration nobody has
/// asked for yet.
const MAX_CHAIN_DEPTH: usize = 15;
const MAX_MENTIONS_PER_REPLY: usize = 2;

/// How much prior conversation a seat is given when it cannot resume. Characters, not bytes —
/// these threads are mostly Chinese.
const SEAT_CONTEXT_BUDGET_CHARS: usize = 4000;

/// How far back the thread is read when building a seat's context. Larger than the character
/// budget can hold, so trimming is decided by the budget rather than by this number.
const SEAT_CONTEXT_MESSAGE_LIMIT: i64 = 40;

const USER_SPEAKER: &str = "用户";

/// One seat of a session, resolved from its stored role and Agent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SeatRosterEntry {
    pub(crate) seat_id: String,
    pub(crate) seat_index: usize,
    pub(crate) agent_id: String,
    pub(crate) briefing: SeatBriefingEntry,
}

/// Why a round stopped, when it stopped for a reason worth telling the user about.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum SeatTurnStop {
    /// An Agent asked the human to decide; nothing further runs until they answer.
    AwaitingHuman,
    /// An Agent declared the round finished.
    RoundComplete,
    /// The reply named nobody, which is an ordinary ending rather than a failure.
    NobodyMentioned,
    Bounded(ChainEndReason),
    /// The seat's generation failed. Chaining past a failure would hand the next seat a turn
    /// whose premise never happened.
    TurnFailed,
}

/// What the coordinator should do next with a completed turn.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SeatTurnDecision {
    pub(crate) next: Vec<SeatTurnAssignment>,
    pub(crate) stop: Option<SeatTurnStop>,
}

/// A seat the coordinator is to invoke, and how deep into the chain that turn sits.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SeatTurnAssignment {
    pub(crate) seat_id: String,
    pub(crate) seat_index: usize,
    pub(crate) depth: usize,
    pub(crate) round_id: String,
    pub(crate) parent_execution_run_id: Option<String>,
}

/// The seat that spoke a message, when an active seat did.
///
/// `speaker_seat_id` is what new writes carry: `start_generation` records the stable seat id and
/// deliberately leaves `seat_index` null, so anything that reads a *live* thread by index sees
/// every message as unattributed. The index is still read here because rows written before
/// migration 59 have only that, and a thread from before the migration still has to render and
/// route.
fn seat_speaker<'a>(
    roster: &'a [SeatRosterEntry],
    message: &super::AgentMessage,
) -> Option<&'a SeatRosterEntry> {
    if let Some(seat_id) = message.speaker_seat_id.as_deref() {
        if let Some(entry) = roster.iter().find(|entry| entry.seat_id == seat_id) {
            return Some(entry);
        }
    }
    message
        .seat_index
        .and_then(|index| roster.iter().find(|entry| entry.seat_index == index))
}

impl AgentRuntimeApplicationService {
    /// Whether a session has more than one participant, and therefore a turn to hand off.
    ///
    /// A missing session answers `false`: there is nothing to coordinate, and reporting the lookup
    /// failure here would turn an unrelated error into a failed send.
    pub(crate) fn is_multi_seat_session(&self, session_id: &str) -> bool {
        self.require_session(session_id).is_ok_and(|session| {
            session
                .seats
                .iter()
                .filter(|seat| seat.left_at.is_none())
                .count()
                > 1
        })
    }

    /// Assembles who is in the session and how each is addressed.
    ///
    /// Built fresh per turn rather than snapshotted when the round starts, because a seat added
    /// mid-session has to be routable from the next turn onward.
    pub(crate) fn seat_roster(
        &self,
        session: &AgentSession,
    ) -> Result<Vec<SeatRosterEntry>, AgentRuntimeApplicationError> {
        let roles = self.ports.expert_roles.list()?;
        let mut role_names = Vec::with_capacity(session.seats.len());
        let mut resolved = Vec::with_capacity(session.seats.len());

        for (seat_index, seat) in session.seats.iter().enumerate() {
            if seat.left_at.is_some() {
                continue;
            }
            let agent = self.require_agent(&seat.agent_id)?;
            let role = seat
                .role_id
                .as_ref()
                .and_then(|role_id| roles.iter().find(|role| &role.id == role_id));
            // A seat with no role is a plain Agent participating under its own name, which is what
            // every single-Agent session is.
            let role_name = role
                .map(|role| role.display_name.clone())
                .unwrap_or_else(|| agent.display_name().to_string());
            role_names.push(role_name.clone());
            resolved.push((
                seat.seat_id.clone(),
                seat_index,
                seat.agent_id.clone(),
                agent,
                role,
                role_name,
            ));
        }

        let mentions = derive_mentions(&role_names);
        Ok(resolved
            .into_iter()
            .zip(mentions)
            .map(
                |((seat_id, seat_index, agent_id, agent, role, role_name), mention)| {
                    SeatRosterEntry {
                        seat_id,
                        seat_index,
                        agent_id,
                        briefing: SeatBriefingEntry {
                            mention,
                            role_name,
                            agent_name: agent.display_name().to_string(),
                            model_family: normalize_model_family(
                                agent.id().as_str(),
                                agent.provider(),
                                None,
                            ),
                            responsibility: role
                                .map(|role| role.responsibility.clone())
                                .unwrap_or_default(),
                            instruction: role
                                .map(|role| role.instruction.clone())
                                .unwrap_or_default(),
                        },
                    }
                },
            )
            .collect())
    }

    /// The seat that most recently held the turn, as its handle.
    ///
    /// Read from the thread rather than tracked as session state: the turn holder is already
    /// recorded, message by message, and a second copy of it would be one more thing to keep
    /// correct across restarts, seat changes and a round that ended mid-flight. A seat that has
    /// since left resolves to nobody, which sends an unaddressed message back to the first seat
    /// instead of to someone no longer in the room.
    fn last_turn_holder(
        &self,
        session_id: &str,
        roster: &[SeatRosterEntry],
    ) -> Result<Option<String>, AgentRuntimeApplicationError> {
        let messages = self
            .ports
            .history
            .recent_messages(session_id, SEAT_CONTEXT_MESSAGE_LIMIT)?;
        Ok(messages
            .iter()
            .rev()
            .find_map(|message| seat_speaker(roster, message))
            .map(|entry| entry.briefing.mention.clone()))
    }

    /// Gives the first reply in a multi-seat round the same identity and instructions as every
    /// later handoff, and decides which seat that reply comes from. Without this, the first
    /// assistant message has no speaker and its mentions cannot enter the seat-turn coordinator.
    ///
    /// The human addresses a seat the same way the Agents address each other -- a line-leading
    /// `@handle` -- rather than through a picker, so the message itself is what selects the
    /// speaker. Routing every user message to the first seat, as this did before, meant a group
    /// chat had one answerable participant and the rest could only ever be reached second-hand.
    pub(crate) fn initial_seat_turn_context(
        &self,
        session: &AgentSession,
        content: &str,
    ) -> Result<Option<(SeatRosterEntry, SeatTurnOwnership, String)>, AgentRuntimeApplicationError>
    {
        let roster = self.seat_roster(session)?;
        if roster.len() <= 1 {
            return Ok(None);
        }
        let Some(first) = roster.first() else {
            return Ok(None);
        };
        let mentions: Vec<String> = roster
            .iter()
            .map(|entry| entry.briefing.mention.clone())
            .collect();
        let last_holder = self.last_turn_holder(&session.id, &roster)?;
        let addressed = route_user_message(
            content,
            &mentions,
            last_holder.as_deref(),
            &first.briefing.mention,
        );
        let seat = roster
            .iter()
            .find(|entry| entry.briefing.mention == addressed)
            .unwrap_or(first);
        let others = roster
            .iter()
            .filter(|entry| entry.seat_index != seat.seat_index)
            .map(|entry| entry.briefing.clone())
            .collect::<Vec<_>>();
        let briefing = build_seat_briefing(
            &seat.briefing,
            &others,
            MAX_CHAIN_DEPTH,
            MAX_MENTIONS_PER_REPLY,
        );
        Ok(Some((
            seat.clone(),
            SeatTurnOwnership {
                seat_id: seat.seat_id.clone(),
                seat_index: seat.seat_index,
                seat_mention: seat.briefing.mention.clone(),
                depth: 1,
                round_id: format!("seat-round-{}", Uuid::new_v4()),
                parent_execution_run_id: None,
            },
            briefing,
        )))
    }

    /// Reads a completed turn and decides who speaks next.
    ///
    /// Only a blocking handoff or a completion stops the round; an informational one leaves the
    /// turn with the Agents. That separation is the point — with a single "notify the human"
    /// action every notification would block, so Agents would learn to stop notifying and the
    /// human would lose the visibility the intents exist to provide.
    pub(crate) fn decide_seat_turn(
        &self,
        terminal: &SeatTurnTerminal,
    ) -> Result<SeatTurnDecision, AgentRuntimeApplicationError> {
        let Some(reply) = terminal.reply.as_deref() else {
            return Ok(SeatTurnDecision {
                next: Vec::new(),
                stop: Some(SeatTurnStop::TurnFailed),
            });
        };

        if let Some(intent) = parse_human_handoff(reply) {
            let effect = apply_human_handoff(intent);
            if effect.round_complete {
                self.announce_turn_status(
                    &terminal.session_id,
                    SeatTurnStatus::RoundComplete {
                        seat_id: terminal.seat_id.clone(),
                        seat_index: terminal.seat_index,
                        mention: terminal.seat_mention.clone(),
                    },
                );
                return Ok(SeatTurnDecision {
                    next: Vec::new(),
                    stop: Some(SeatTurnStop::RoundComplete),
                });
            }
            if effect.turn_holder_is_human {
                self.announce_turn_status(
                    &terminal.session_id,
                    SeatTurnStatus::WaitingHuman {
                        seat_id: terminal.seat_id.clone(),
                        seat_index: terminal.seat_index,
                        mention: terminal.seat_mention.clone(),
                        since: self.ports.clock.now(),
                    },
                );
                return Ok(SeatTurnDecision {
                    next: Vec::new(),
                    stop: Some(SeatTurnStop::AwaitingHuman),
                });
            }
        }

        let session = self.require_session(&terminal.session_id)?;
        let roster = self.seat_roster(&session)?;
        let mentions: Vec<String> = roster
            .iter()
            .map(|entry| entry.briefing.mention.clone())
            .collect();
        let routed = next_turn_targets(
            reply,
            &mentions,
            &terminal.seat_mention,
            terminal.depth,
            MAX_CHAIN_DEPTH,
            MAX_MENTIONS_PER_REPLY,
        );

        let next: Vec<SeatTurnAssignment> = routed
            .targets
            .iter()
            .filter_map(|mention| {
                roster
                    .iter()
                    .find(|entry| &entry.briefing.mention == mention)
                    .map(|entry| SeatTurnAssignment {
                        seat_id: entry.seat_id.clone(),
                        seat_index: entry.seat_index,
                        depth: terminal.depth + 1,
                        round_id: terminal.round_id.clone(),
                        parent_execution_run_id: Some(terminal.execution_run_id.clone()),
                    })
            })
            .collect();

        let stop = match (&routed.ended_reason, next.is_empty()) {
            (Some(reason), _) => Some(SeatTurnStop::Bounded(reason.clone())),
            (None, true) => Some(SeatTurnStop::NobodyMentioned),
            (None, false) => None,
        };
        Ok(SeatTurnDecision { next, stop })
    }

    /// How a seat's Agent is configured for a turn.
    ///
    /// A seat runs its own Agent, not the session's, so this is built around that Agent rather
    /// than read from the session's saved chat settings — which belong to whoever the picker last
    /// selected and would carry one participant's model onto another's turn.
    pub(super) fn seat_chat_configuration(
        &self,
        session: &AgentSession,
        agent: &AgentDefinition,
    ) -> Result<AgentChatConfiguration, AgentRuntimeApplicationError> {
        let interaction_mode = if agent.supports(InteractionMode::Cli) {
            InteractionMode::Cli
        } else {
            InteractionMode::Api
        };
        self.ports.sessions.validate_seat_configuration(
            session,
            AgentChatConfiguration {
                agent_id: agent.id().as_str().to_string(),
                interaction_mode,
                execution_mode: "inherit".to_string(),
                provider_id: None,
                model_id: None,
                reasoning_depth: None,
                streaming: true,
                thinking: false,
                long_context: false,
            },
        )
    }

    /// Starts one seat's turn: its role in the CLI's own system-prompt channel, the thread it
    /// missed in the prompt.
    pub(crate) fn start_seat_turn(
        &self,
        session_id: &str,
        assignment: &SeatTurnAssignment,
    ) -> Result<(), AgentRuntimeApplicationError> {
        let session = self.require_session(session_id)?;
        let roster = self.seat_roster(&session)?;
        let seat = roster
            .iter()
            .find(|entry| entry.seat_id == assignment.seat_id)
            .ok_or_else(|| {
                AgentRuntimeApplicationError::Validation(format!(
                    "Seat {} is no longer part of this session.",
                    assignment.seat_index
                ))
            })?;
        let agent = self.require_agent(&seat.agent_id)?;
        let configuration = self.seat_chat_configuration(&session, &agent)?;

        let others: Vec<SeatBriefingEntry> = roster
            .iter()
            .filter(|entry| entry.seat_index != seat.seat_index)
            .map(|entry| entry.briefing.clone())
            .collect();
        let briefing = build_seat_briefing(
            &seat.briefing,
            &others,
            MAX_CHAIN_DEPTH,
            MAX_MENTIONS_PER_REPLY,
        );

        self.announce_turn_status(
            session_id,
            SeatTurnStatus::Agent {
                seat_id: seat.seat_id.clone(),
                seat_index: seat.seat_index,
                mention: seat.briefing.mention.clone(),
                depth: assignment.depth,
                max_depth: MAX_CHAIN_DEPTH,
            },
        );
        let prompt = self.seat_turn_prompt(session_id, &roster, seat)?;
        let result = self.start_message_generation(
            &session,
            &agent,
            MessageGenerationInput {
                source: super::AgentMessageSource::Desktop,
                configuration,
                content: prompt,
                file_references: Vec::new(),
                role_briefing: Some(briefing),
                seat_ownership: Some(SeatTurnOwnership {
                    seat_id: seat.seat_id.clone(),
                    seat_index: seat.seat_index,
                    seat_mention: seat.briefing.mention.clone(),
                    depth: assignment.depth,
                    round_id: assignment.round_id.clone(),
                    parent_execution_run_id: assignment.parent_execution_run_id.clone(),
                }),
                record_user_message: false,
                // A seat turn is written by the runtime to hand off between participants, not
                // typed by someone waiting on the thread (`add-agent-user-question`).
                interactive: false,
                runner: super::RunnerSelection::local(),
            },
        );
        result.map(|_| ())
    }

    /// A failed announcement must not fail the turn: the bar is a display, and losing it is a
    /// smaller loss than losing the round.
    fn announce_turn_status(&self, session_id: &str, status: SeatTurnStatus) {
        let _ = self.ports.events.publish(AgentEvent::TurnStatusChanged {
            session_id: session_id.to_string(),
            status,
        });
    }

    /// Builds what the seat reads when its turn starts.
    ///
    /// Prior turns are always injected in a multi-seat session, never resumed. Each seat is a
    /// separate CLI process with its own context, so resuming a seat's own provider session would
    /// replay what *it* said and omit everything its teammates said since — exactly the part it is
    /// being handed the turn to act on.
    fn seat_turn_prompt(
        &self,
        session_id: &str,
        roster: &[SeatRosterEntry],
        seat: &SeatRosterEntry,
    ) -> Result<String, AgentRuntimeApplicationError> {
        let messages = self
            .ports
            .history
            .recent_messages(session_id, SEAT_CONTEXT_MESSAGE_LIMIT)?;
        let turns: Vec<SeatContextTurn> = messages
            .iter()
            .filter(|message| !message.content.trim().is_empty())
            .map(|message| SeatContextTurn {
                speaker: seat_speaker(roster, message)
                    .map(|entry| entry.briefing.mention.clone())
                    .unwrap_or_else(|| USER_SPEAKER.to_string()),
                content: message.content.clone(),
            })
            .collect();

        let context = build_seat_context(&turns, None, SEAT_CONTEXT_BUDGET_CHARS);
        let handoff = format!(
            "轮到你（@{}）发言。请基于上面的对话继续。",
            seat.briefing.mention
        );
        Ok(match context.mode {
            SeatContextMode::Inject if !context.text.is_empty() => {
                format!("{}\n\n{handoff}", context.text)
            }
            _ => handoff,
        })
    }
}
