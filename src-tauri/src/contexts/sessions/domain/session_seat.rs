//! One participant in a session: an Agent playing an expert role.
//!
//! Seats live in a JSON column rather than a joined table because `SESSION_SELECT` is the hot path
//! for list, search, and get; a join there would cost every read for a feature most sessions do not
//! use.

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SessionSeat {
    pub(crate) agent_id: String,
    /// `None` for a plain single-Agent session, which has no role assigned.
    pub(crate) role_id: Option<String>,
}

pub(crate) fn encode_seats(seats: &[SessionSeat]) -> String {
    let values: Vec<serde_json::Value> = seats
        .iter()
        .map(|seat| {
            serde_json::json!({
                "agentId": seat.agent_id,
                "roleId": seat.role_id,
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
pub(crate) fn decode_seats(stored: &str, agent_id: &str) -> Vec<SessionSeat> {
    let fallback = vec![SessionSeat {
        agent_id: agent_id.to_string(),
        role_id: None,
    }];
    let Ok(serde_json::Value::Array(entries)) = serde_json::from_str::<serde_json::Value>(stored)
    else {
        return fallback;
    };
    let seats: Vec<SessionSeat> = entries
        .iter()
        .filter_map(|entry| {
            let agent_id = entry.get("agentId")?.as_str()?.trim();
            if agent_id.is_empty() {
                return None;
            }
            Some(SessionSeat {
                agent_id: agent_id.to_string(),
                role_id: entry
                    .get("roleId")
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

#[cfg(test)]
mod tests {
    use super::*;

    fn seat(agent_id: &str, role_id: Option<&str>) -> SessionSeat {
        SessionSeat {
            agent_id: agent_id.to_string(),
            role_id: role_id.map(str::to_string),
        }
    }

    #[test]
    fn round_trips_seats_in_order() {
        let seats = vec![
            seat("claude-code", Some("role-architect")),
            seat("codex-cli", Some("role-reviewer")),
        ];
        assert_eq!(decode_seats(&encode_seats(&seats), "claude-code"), seats);
    }

    #[test]
    fn keeps_a_seat_without_a_role() {
        let seats = vec![seat("claude-code", None)];
        assert_eq!(decode_seats(&encode_seats(&seats), "claude-code"), seats);
    }

    /// Every session predating seats stores `[]`, and each must open as its single Agent.
    #[test]
    fn presents_an_empty_list_as_the_one_seat_case() {
        assert_eq!(
            decode_seats("[]", "claude-code"),
            vec![seat("claude-code", None)]
        );
    }

    /// A corrupted column must cost the seat list, not the session.
    #[test]
    fn degrades_unreadable_storage_to_the_one_seat_case() {
        for stored in ["", "not json", "{}", "null"] {
            assert_eq!(
                decode_seats(stored, "codex-cli"),
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
            decode_seats(stored, "codex-cli"),
            vec![seat("claude-code", None)]
        );
    }
}
