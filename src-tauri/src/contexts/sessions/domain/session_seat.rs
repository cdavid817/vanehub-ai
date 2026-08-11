//! One participant in a session: an Agent playing an expert role.
//!
//! Seats live in a JSON column rather than a joined table because `SESSION_SELECT` is the hot path
//! for list, search, and get; a join there would cost every read for a feature most sessions do not
//! use.

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SessionSeatRoleSnapshot {
    pub(crate) role_name: Option<String>,
    pub(crate) avatar: String,
    pub(crate) color: String,
    pub(crate) responsibility: Option<String>,
    pub(crate) agent_name: String,
    pub(crate) model_family: String,
    pub(crate) cross_family_reviewer: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SessionSeat {
    pub(crate) seat_id: String,
    pub(crate) agent_id: String,
    /// `None` for a plain single-Agent session, which has no role assigned.
    pub(crate) role_id: Option<String>,
    pub(crate) role_snapshot: Option<SessionSeatRoleSnapshot>,
    pub(crate) joined_at: String,
    pub(crate) left_at: Option<String>,
}

impl SessionSeat {
    pub(crate) fn is_active(&self) -> bool {
        self.left_at.is_none()
    }
}

pub(crate) fn encode_seats(seats: &[SessionSeat]) -> String {
    let values: Vec<serde_json::Value> = seats
        .iter()
        .map(|seat| {
            serde_json::json!({
                "seatId": seat.seat_id,
                "agentId": seat.agent_id,
                "roleId": seat.role_id,
                "roleSnapshot": seat.role_snapshot.as_ref().map(|snapshot| serde_json::json!({
                    "roleName": snapshot.role_name,
                    "avatar": snapshot.avatar,
                    "color": snapshot.color,
                    "responsibility": snapshot.responsibility,
                    "agentName": snapshot.agent_name,
                    "modelFamily": snapshot.model_family,
                    "crossFamilyReviewer": snapshot.cross_family_reviewer,
                })),
                "joinedAt": seat.joined_at,
                "leftAt": seat.left_at,
            })
        })
        .collect();
    serde_json::Value::Array(values).to_string()
}

/// Reads the stored seat list, presenting anything unreadable as the one-seat case.
///
/// Degrading rather than failing is deliberate: seats were added to a table full of existing rows,
/// and a session that predates them — or whose column was corrupted — must still open. Refusing to
/// read would turn a cosmetic problem into a lost session.
pub(crate) fn decode_seats(
    stored: &str,
    session_id: &str,
    agent_id: &str,
    created_at: &str,
) -> Vec<SessionSeat> {
    let fallback = vec![SessionSeat {
        seat_id: legacy_seat_id(session_id, 0),
        agent_id: agent_id.to_string(),
        role_id: None,
        role_snapshot: None,
        joined_at: created_at.to_string(),
        left_at: None,
    }];
    let Ok(serde_json::Value::Array(entries)) = serde_json::from_str::<serde_json::Value>(stored)
    else {
        return fallback;
    };
    let seats: Vec<SessionSeat> = entries
        .iter()
        .enumerate()
        .filter_map(|(index, entry)| {
            let agent_id = entry.get("agentId")?.as_str()?.trim();
            if agent_id.is_empty() {
                return None;
            }
            Some(SessionSeat {
                seat_id: entry
                    .get("seatId")
                    .and_then(serde_json::Value::as_str)
                    .filter(|value| !value.trim().is_empty())
                    .map(str::to_string)
                    .unwrap_or_else(|| legacy_seat_id(session_id, index)),
                agent_id: agent_id.to_string(),
                role_id: entry
                    .get("roleId")
                    .and_then(serde_json::Value::as_str)
                    .map(str::to_string),
                role_snapshot: entry.get("roleSnapshot").and_then(decode_role_snapshot),
                joined_at: entry
                    .get("joinedAt")
                    .and_then(serde_json::Value::as_str)
                    .filter(|value| !value.trim().is_empty())
                    .unwrap_or(created_at)
                    .to_string(),
                left_at: entry
                    .get("leftAt")
                    .and_then(serde_json::Value::as_str)
                    .map(str::to_string),
            })
        })
        .collect();
    if seats.is_empty() {
        return fallback;
    }
    seats
}

pub(crate) fn legacy_seat_id(session_id: &str, index: usize) -> String {
    format!("{session_id}:seat:{index}")
}

fn decode_role_snapshot(value: &serde_json::Value) -> Option<SessionSeatRoleSnapshot> {
    Some(SessionSeatRoleSnapshot {
        role_name: value
            .get("roleName")
            .and_then(serde_json::Value::as_str)
            .map(str::to_string),
        avatar: value.get("avatar")?.as_str()?.to_string(),
        color: value.get("color")?.as_str()?.to_string(),
        responsibility: value
            .get("responsibility")
            .and_then(serde_json::Value::as_str)
            .map(str::to_string),
        agent_name: value.get("agentName")?.as_str()?.to_string(),
        model_family: value.get("modelFamily")?.as_str()?.to_string(),
        cross_family_reviewer: value
            .get("crossFamilyReviewer")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn seat(agent_id: &str, role_id: Option<&str>) -> SessionSeat {
        SessionSeat {
            seat_id: "session-1:seat:0".to_string(),
            agent_id: agent_id.to_string(),
            role_id: role_id.map(str::to_string),
            role_snapshot: None,
            joined_at: "2026-08-10T00:00:00Z".to_string(),
            left_at: None,
        }
    }

    #[test]
    fn round_trips_seats_in_order() {
        let seats = vec![
            seat("claude-code", Some("role-architect")),
            SessionSeat {
                seat_id: "session-1:seat:1".to_string(),
                ..seat("codex-cli", Some("role-reviewer"))
            },
        ];
        assert_eq!(
            decode_seats(
                &encode_seats(&seats),
                "session-1",
                "claude-code",
                "2026-08-10T00:00:00Z"
            ),
            seats
        );
    }

    #[test]
    fn keeps_a_seat_without_a_role() {
        let seats = vec![seat("claude-code", None)];
        assert_eq!(
            decode_seats(
                &encode_seats(&seats),
                "session-1",
                "claude-code",
                "2026-08-10T00:00:00Z"
            ),
            seats
        );
    }

    /// Every session predating seats stores `[]`, and each must open as its single Agent.
    #[test]
    fn presents_an_empty_list_as_the_one_seat_case() {
        assert_eq!(
            decode_seats("[]", "session-1", "claude-code", "2026-08-10T00:00:00Z"),
            vec![seat("claude-code", None)]
        );
    }

    /// A corrupted column must cost the seat list, not the session.
    #[test]
    fn degrades_unreadable_storage_to_the_one_seat_case() {
        for stored in ["", "not json", "{}", "null"] {
            assert_eq!(
                decode_seats(stored, "session-1", "codex-cli", "2026-08-10T00:00:00Z"),
                vec![seat("codex-cli", None)],
                "failed for {stored:?}"
            );
        }
    }

    /// An entry without a usable Agent cannot be routed to, so it is not a seat.
    #[test]
    fn drops_entries_with_no_agent() {
        let stored = r#"[{"agentId":"claude-code"},{"agentId":"  "},{"roleId":"role-1"}]"#;
        assert_eq!(
            decode_seats(stored, "session-1", "codex-cli", "2026-08-10T00:00:00Z"),
            vec![seat("claude-code", None)]
        );
    }

    #[test]
    fn assigns_distinct_deterministic_ids_to_legacy_entries() {
        let seats = decode_seats(
            r#"[{"agentId":"claude-code"},{"agentId":"codex-cli"}]"#,
            "session-legacy",
            "claude-code",
            "2026-08-10T00:00:00Z",
        );
        assert_eq!(seats[0].seat_id, "session-legacy:seat:0");
        assert_eq!(seats[1].seat_id, "session-legacy:seat:1");
        assert_ne!(seats[0].seat_id, seats[1].seat_id);
    }
}
