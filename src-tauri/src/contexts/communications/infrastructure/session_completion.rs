use crate::contexts::communications::application::{
    AgentExecutionResult, CommunicationsApplicationError,
};
use crate::contexts::sessions::api::{RuntimeMessageSnapshot, SessionsApi};
use std::thread;
use std::time::{Duration, Instant};

const COMPLETION_TIMEOUT: Duration = Duration::from_secs(30 * 60);
const COMPLETION_POLL_INTERVAL: Duration = Duration::from_millis(100);

pub(crate) fn wait_for_completion(
    sessions: &SessionsApi,
    _session_id: &str,
    message_id: &str,
) -> Result<AgentExecutionResult, CommunicationsApplicationError> {
    wait_for_completion_with(
        || {
            sessions
                .runtime_message(message_id)
                .map_err(|_| CommunicationsApplicationError::failure("completion-read-failed"))
        },
        COMPLETION_TIMEOUT,
        COMPLETION_POLL_INTERVAL,
    )
}

fn wait_for_completion_with(
    mut load_message: impl FnMut() -> Result<
        Option<RuntimeMessageSnapshot>,
        CommunicationsApplicationError,
    >,
    timeout: Duration,
    poll_interval: Duration,
) -> Result<AgentExecutionResult, CommunicationsApplicationError> {
    let deadline = Instant::now() + timeout;
    loop {
        let message = load_message()?
            .ok_or_else(|| CommunicationsApplicationError::failure("completion-message-missing"))?;
        match message.status.as_str() {
            "completed" => {
                return Ok(AgentExecutionResult {
                    reply: message.content,
                    message_id: message.id,
                });
            }
            "failed" => {
                return Err(CommunicationsApplicationError::user_visible(
                    "agent-generation-failed",
                    "The Agent could not complete this request.",
                ));
            }
            "cancelled" => {
                return Err(CommunicationsApplicationError::user_visible(
                    "agent-generation-cancelled",
                    "The Agent request was cancelled.",
                ));
            }
            _ if Instant::now() >= deadline => {
                return Err(CommunicationsApplicationError::user_visible(
                    "agent-generation-timeout",
                    "The Agent did not finish before the IM response timeout.",
                ));
            }
            _ => thread::sleep(poll_interval),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn runtime_message(status: &str, content: &str) -> RuntimeMessageSnapshot {
        RuntimeMessageSnapshot {
            id: "message-1".to_string(),
            session_id: "session-1".to_string(),
            role: "assistant".to_string(),
            status: status.to_string(),
            content: content.to_string(),
            thinking_content: None,
            tool_use: Vec::new(),
            rich_blocks: Vec::new(),
            token_usage: None,
            file_references: Vec::new(),
            error: None,
            created_at: "2026-07-18T00:00:00Z".to_string(),
            updated_at: "2026-07-18T00:00:01Z".to_string(),
        }
    }

    #[test]
    fn persisted_completed_message_maps_to_the_final_reply() {
        let outcome = wait_for_completion_with(
            || Ok(Some(runtime_message("completed", "done"))),
            Duration::ZERO,
            Duration::ZERO,
        )
        .expect("completed reply");

        assert_eq!(
            outcome,
            AgentExecutionResult {
                reply: "done".to_string(),
                message_id: "message-1".to_string(),
            }
        );
    }

    #[test]
    fn terminal_timeout_and_missing_states_keep_safe_codes() {
        for (status, expected_code) in [
            ("failed", "agent-generation-failed"),
            ("cancelled", "agent-generation-cancelled"),
            ("streaming", "agent-generation-timeout"),
        ] {
            let error = wait_for_completion_with(
                || Ok(Some(runtime_message(status, "partial"))),
                Duration::ZERO,
                Duration::ZERO,
            )
            .expect_err("terminal error");
            assert_eq!(error.safe_code(), expected_code);
            assert!(error.user_message().is_some());
        }

        let missing = wait_for_completion_with(|| Ok(None), Duration::ZERO, Duration::ZERO)
            .expect_err("missing message");
        assert_eq!(missing.safe_code(), "completion-message-missing");
        assert!(missing.user_message().is_none());
    }
}
