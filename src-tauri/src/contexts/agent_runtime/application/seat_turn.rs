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
use crate::contexts::agent_runtime::domain::InteractionMode;
use crate::contexts::agent_runtime::domain::{
    apply_human_handoff, build_seat_briefing, build_seat_context, derive_mentions,
    next_turn_targets, normalize_model_family, parse_human_handoff, ChainEndReason,
    SeatBriefingEntry, SeatContextMode, SeatTurn as SeatContextTurn,
};

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
    pub(crate) seat_index: usize,
    pub(crate) depth: usize,
}

impl AgentRuntimeApplicationService {
    /// Whether a session has more than one participant, and therefore a turn to hand off.
    ///
    /// A missing session answers `false`: there is nothing to coordinate, and reporting the lookup
    /// failure here would turn an unrelated error into a failed send.
    pub(crate) fn is_multi_seat_session(&self, session_id: &str) -> bool {
        self.require_session(session_id)
            .is_ok_and(|session| session.seats.len() > 1)
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

        for seat in &session.seats {
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
            resolved.push((seat.agent_id.clone(), agent, role, role_name));
        }

        let mentions = derive_mentions(&role_names);
        Ok(resolved
            .into_iter()
            .zip(mentions)
            .enumerate()
            .map(
                |(seat_index, ((agent_id, agent, role, role_name), mention))| SeatRosterEntry {
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
                },
            )
            .collect())
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
                        seat_index: entry.seat_index,
                        depth: terminal.depth + 1,
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
            .find(|entry| entry.seat_index == assignment.seat_index)
            .ok_or_else(|| {
                AgentRuntimeApplicationError::Validation(format!(
                    "Seat {} is no longer part of this session.",
                    assignment.seat_index
                ))
            })?;
        let agent = self.require_agent(&seat.agent_id)?;
        // A seat runs its own Agent, not the session's, so the configuration is built around that
        // Agent rather than read from the session's saved chat settings.
        let interaction_mode = if agent.supports(InteractionMode::Cli) {
            InteractionMode::Cli
        } else {
            InteractionMode::Api
        };
        let configuration = self.ports.sessions.validate_seat_configuration(
            &session,
            AgentChatConfiguration {
                agent_id: seat.agent_id.clone(),
                interaction_mode,
                permission_mode: "default".to_string(),
                provider_id: None,
                model_id: None,
                reasoning_depth: None,
                streaming: true,
                thinking: false,
                long_context: false,
            },
        )?;

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
                seat_index: seat.seat_index,
                mention: seat.briefing.mention.clone(),
                depth: assignment.depth,
                max_depth: MAX_CHAIN_DEPTH,
            },
        );
        let prompt = self.seat_turn_prompt(session_id, &roster, seat)?;
        let lease = self.ports.generations.reserve(&session.id)?;
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
                    seat_index: seat.seat_index,
                    seat_mention: seat.briefing.mention.clone(),
                    depth: assignment.depth,
                }),
                record_user_message: false,
                orchestration_profile: None,
            },
            &lease,
        );
        if result.is_err() {
            let _ = self.ports.generations.release(&lease);
        }
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
            .into_iter()
            .filter(|message| !message.content.trim().is_empty())
            .map(|message| SeatContextTurn {
                speaker: message
                    .seat_index
                    .and_then(|index| roster.iter().find(|entry| entry.seat_index == index))
                    .map(|entry| entry.briefing.mention.clone())
                    .unwrap_or_else(|| USER_SPEAKER.to_string()),
                content: message.content,
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
