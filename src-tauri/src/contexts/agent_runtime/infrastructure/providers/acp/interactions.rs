//! Backend-owned pending interactions: permission requests, blocking vendor questions, plan
//! approvals, and host-proxied operations the policy says to ask about.
//!
//! An interaction is bound to the connection epoch, the session, the turn, the agent's RPC id,
//! and the tool call it concerns. A decision consumes it exactly once. A decision aimed at the
//! wrong session, an older epoch, or an already-consumed request is refused -- the UI never gets
//! to approve something the backend is not currently waiting on. The store is the source of
//! truth; the React tree only renders it, so a refresh recovers every open request from here.

use super::budget::INTERACTION_DEADLINE;
use super::jsonrpc::RpcId;
use crate::contexts::agent_runtime::application::ToolApprovalDecision;
use serde_json::Value;
use std::collections::BTreeMap;
use std::time::{Duration, Instant};

/// One option the agent offered on a `session/request_permission`. The id is opaque and is the
/// only thing ever sent back.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PermissionOption {
    pub(crate) option_id: String,
    pub(crate) name: String,
    pub(crate) kind: PermissionOptionKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PermissionOptionKind {
    AllowOnce,
    AllowAlways,
    RejectOnce,
    RejectAlways,
    Unknown,
}

impl PermissionOptionKind {
    pub(crate) fn parse(value: &str) -> Self {
        match value {
            "allow_once" => Self::AllowOnce,
            "allow_always" => Self::AllowAlways,
            "reject_once" => Self::RejectOnce,
            "reject_always" => Self::RejectAlways,
            _ => Self::Unknown,
        }
    }
}

/// Picks the offered option that carries exactly the user's decision, never a broader one.
///
/// "Allow" selects `allow_once`; if the agent offered only `allow_always`, that is a wider grant
/// than the user made and the request is answered with the cancelled outcome instead. "Deny"
/// prefers `reject_once` and accepts `reject_always` as the fallback: a permanent rejection is
/// narrower than what was asked, not broader.
pub(crate) fn select_permission_option(
    options: &[PermissionOption],
    approve: bool,
) -> Option<&PermissionOption> {
    let wanted: &[PermissionOptionKind] = if approve {
        &[PermissionOptionKind::AllowOnce]
    } else {
        &[
            PermissionOptionKind::RejectOnce,
            PermissionOptionKind::RejectAlways,
        ]
    };
    wanted
        .iter()
        .find_map(|kind| options.iter().find(|option| option.kind == *kind))
}

/// One entry of a `cursor/ask_question` request (Cursor's ACP extension schema).
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct CursorQuestion {
    pub(crate) id: String,
    pub(crate) prompt: String,
    pub(crate) options: Vec<CursorOption>,
    pub(crate) allow_multiple: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct CursorOption {
    pub(crate) id: String,
    pub(crate) label: String,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum InteractionKind {
    /// `session/request_permission`.
    Permission {
        tool_call_id: String,
        options: Vec<PermissionOption>,
        /// The permission action the host evaluated, for re-evaluation before replying.
        action: String,
        resource: String,
    },
    /// `cursor/ask_question`, with every question kept whole so the reply can name the option
    /// ids the agent issued instead of echoing labels back under the wrong key.
    Question {
        tool_call_id: String,
        questions: Vec<CursorQuestion>,
    },
    /// `cursor/create_plan`.
    Plan { tool_call_id: String },
    /// `fs/write_text_file` the policy asked about.
    FileWrite { path: String, content: String },
    /// `terminal/create` the policy asked about.
    TerminalCreate { request: Value },
}

impl InteractionKind {
    #[cfg(test)]
    pub(crate) fn tool_call_id(&self) -> Option<&str> {
        match self {
            Self::Permission { tool_call_id, .. }
            | Self::Question { tool_call_id, .. }
            | Self::Plan { tool_call_id } => Some(tool_call_id),
            Self::FileWrite { .. } | Self::TerminalCreate { .. } => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct PendingInteraction {
    /// The id the UI resolves by. Equal to the tool call id when there is one, otherwise a
    /// host-minted id, so approvals and questions share the existing `call_id` channel.
    pub(crate) call_id: String,
    pub(crate) rpc_id: RpcId,
    pub(crate) epoch: u64,
    pub(crate) session_id: String,
    pub(crate) turn_id: String,
    pub(crate) kind: InteractionKind,
    pub(crate) created_at: Instant,
    pub(crate) deadline: Instant,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ConsumedInteraction {
    pub(crate) interaction: PendingInteraction,
    pub(crate) decision: ToolApprovalDecision,
}

/// Why a decision was not applied.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum InteractionRejection {
    NotFound,
    /// The interaction belongs to another session.
    WrongSession,
    /// The interaction was created on a connection that has since been replaced.
    StaleEpoch,
}

#[derive(Debug, Default)]
pub(crate) struct PendingInteractionStore {
    pending: BTreeMap<String, PendingInteraction>,
}

/// Where an interaction belongs: the connection epoch, session, and turn a decision must match.
#[derive(Debug, Clone, Copy)]
pub(crate) struct InteractionScope<'a> {
    pub(crate) epoch: u64,
    pub(crate) session_id: &'a str,
    pub(crate) turn_id: &'a str,
}

impl PendingInteractionStore {
    pub(crate) fn register(
        &mut self,
        call_id: String,
        rpc_id: RpcId,
        scope: InteractionScope<'_>,
        kind: InteractionKind,
        deadline: Option<Duration>,
    ) -> PendingInteraction {
        let now = Instant::now();
        let interaction = PendingInteraction {
            call_id: call_id.clone(),
            rpc_id,
            epoch: scope.epoch,
            session_id: scope.session_id.to_string(),
            turn_id: scope.turn_id.to_string(),
            kind,
            created_at: now,
            deadline: now + deadline.unwrap_or(INTERACTION_DEADLINE),
        };
        self.pending.insert(call_id, interaction.clone());
        interaction
    }

    /// Consumes one interaction with a decision. Refuses anything the backend is not waiting on.
    pub(crate) fn consume(
        &mut self,
        call_id: &str,
        session_id: &str,
        current_epoch: u64,
        decision: ToolApprovalDecision,
    ) -> Result<ConsumedInteraction, InteractionRejection> {
        let Some(interaction) = self.pending.get(call_id) else {
            return Err(InteractionRejection::NotFound);
        };
        if interaction.session_id != session_id {
            return Err(InteractionRejection::WrongSession);
        }
        if interaction.epoch != current_epoch {
            // Removed as well: nothing on the current connection can ever answer it.
            self.pending.remove(call_id);
            return Err(InteractionRejection::StaleEpoch);
        }
        let interaction = self
            .pending
            .remove(call_id)
            .ok_or(InteractionRejection::NotFound)?;
        Ok(ConsumedInteraction {
            interaction,
            decision,
        })
    }

    /// Every interaction past its deadline, removed. The caller answers each with the protocol's
    /// cancelled outcome; none is ever approved by expiry.
    pub(crate) fn take_expired(&mut self, now: Instant) -> Vec<PendingInteraction> {
        let expired: Vec<String> = self
            .pending
            .iter()
            .filter(|(_, interaction)| interaction.deadline <= now)
            .map(|(call_id, _)| call_id.clone())
            .collect();
        expired
            .iter()
            .filter_map(|call_id| self.pending.remove(call_id))
            .collect()
    }

    /// Every interaction for a turn, removed. Used on cancellation and on connection loss.
    pub(crate) fn take_for_turn(&mut self, turn_id: &str) -> Vec<PendingInteraction> {
        let ids: Vec<String> = self
            .pending
            .iter()
            .filter(|(_, interaction)| interaction.turn_id == turn_id)
            .map(|(call_id, _)| call_id.clone())
            .collect();
        ids.iter()
            .filter_map(|call_id| self.pending.remove(call_id))
            .collect()
    }

    pub(crate) fn list(&self) -> Vec<PendingInteraction> {
        self.pending.values().cloned().collect()
    }

    pub(crate) fn get(&self, call_id: &str) -> Option<&PendingInteraction> {
        self.pending.get(call_id)
    }

    #[cfg(test)]
    pub(crate) fn is_empty(&self) -> bool {
        self.pending.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn options() -> Vec<PermissionOption> {
        vec![
            PermissionOption {
                option_id: "always".to_string(),
                name: "Always allow".to_string(),
                kind: PermissionOptionKind::AllowAlways,
            },
            PermissionOption {
                option_id: "once".to_string(),
                name: "Allow once".to_string(),
                kind: PermissionOptionKind::AllowOnce,
            },
            PermissionOption {
                option_id: "reject".to_string(),
                name: "Reject".to_string(),
                kind: PermissionOptionKind::RejectOnce,
            },
        ]
    }

    fn permission() -> InteractionKind {
        InteractionKind::Permission {
            tool_call_id: "call-1".to_string(),
            options: options(),
            action: "file.write".to_string(),
            resource: "/workspace/a.txt".to_string(),
        }
    }

    #[test]
    fn allow_once_never_widens_to_allow_always() {
        let offered = options();
        let selected = select_permission_option(&offered, true).expect("allow once");
        assert_eq!(selected.option_id, "once");
        let denied = select_permission_option(&offered, false).expect("reject once");
        assert_eq!(denied.option_id, "reject");
        let only_always = vec![options()[0].clone()];
        assert!(select_permission_option(&only_always, true).is_none());
        let only_reject_always = vec![PermissionOption {
            option_id: "never".to_string(),
            name: "Never".to_string(),
            kind: PermissionOptionKind::RejectAlways,
        }];
        assert_eq!(
            select_permission_option(&only_reject_always, false).map(|o| o.option_id.as_str()),
            Some("never")
        );
        assert_eq!(
            PermissionOptionKind::parse("weird"),
            PermissionOptionKind::Unknown
        );
    }

    #[test]
    fn a_decision_is_consumed_once_and_scoped_to_session_and_epoch() {
        let mut store = PendingInteractionStore::default();
        store.register(
            "call-1".to_string(),
            RpcId::Number(5),
            InteractionScope {
                epoch: 3,
                session_id: "session-a",
                turn_id: "turn-1",
            },
            permission(),
            None,
        );
        assert_eq!(
            store
                .consume("call-1", "session-b", 3, ToolApprovalDecision::Approved)
                .expect_err("wrong session"),
            InteractionRejection::WrongSession
        );
        let consumed = store
            .consume("call-1", "session-a", 3, ToolApprovalDecision::Approved)
            .expect("consumed");
        assert_eq!(consumed.interaction.rpc_id, RpcId::Number(5));
        assert_eq!(consumed.decision, ToolApprovalDecision::Approved);
        assert_eq!(
            store
                .consume("call-1", "session-a", 3, ToolApprovalDecision::Approved)
                .expect_err("second click"),
            InteractionRejection::NotFound
        );

        store.register(
            "call-2".to_string(),
            RpcId::Number(6),
            InteractionScope {
                epoch: 3,
                session_id: "session-a",
                turn_id: "turn-1",
            },
            permission(),
            None,
        );
        assert_eq!(
            store
                .consume("call-2", "session-a", 4, ToolApprovalDecision::Denied)
                .expect_err("stale epoch"),
            InteractionRejection::StaleEpoch
        );
        assert!(store.is_empty());
    }

    #[test]
    fn expiry_and_turn_teardown_remove_without_approving() {
        let mut store = PendingInteractionStore::default();
        store.register(
            "expired".to_string(),
            RpcId::Number(1),
            InteractionScope {
                epoch: 1,
                session_id: "s",
                turn_id: "turn-1",
            },
            permission(),
            Some(Duration::from_millis(0)),
        );
        store.register(
            "live".to_string(),
            RpcId::Number(2),
            InteractionScope {
                epoch: 1,
                session_id: "s",
                turn_id: "turn-1",
            },
            InteractionKind::Plan {
                tool_call_id: "plan-1".to_string(),
            },
            None,
        );
        store.register(
            "other-turn".to_string(),
            RpcId::Number(3),
            InteractionScope {
                epoch: 1,
                session_id: "s",
                turn_id: "turn-2",
            },
            InteractionKind::FileWrite {
                path: "/w/x".to_string(),
                content: String::new(),
            },
            None,
        );
        let expired = store.take_expired(Instant::now() + Duration::from_millis(1));
        assert_eq!(expired.len(), 1);
        assert_eq!(expired[0].call_id, "expired");
        let torn_down = store.take_for_turn("turn-1");
        assert_eq!(torn_down.len(), 1);
        assert_eq!(torn_down[0].call_id, "live");
        assert_eq!(torn_down[0].kind.tool_call_id(), Some("plan-1"));
        assert_eq!(store.list().len(), 1);
        assert_eq!(store.list()[0].kind.tool_call_id(), None);
    }
}
