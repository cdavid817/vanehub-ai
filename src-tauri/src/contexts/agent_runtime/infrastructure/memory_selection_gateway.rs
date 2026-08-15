use super::api_process_adapter::{summarize_turns, wire_format_for, REQUEST_TIMEOUT};
use crate::contexts::agent_runtime::application::{
    AgentMemory, AgentMemorySelectionPort, AgentRuntimeApplicationError, ApiAgentGateway,
    ApiCredentialPort,
};
use crate::contexts::agent_runtime::domain::{
    parse_memory_selection, render_memory_age, MEMORY_SELECTION_INSTRUCTION,
};
use crate::platform::network::blocking_http_client;
use serde_json::json;
use std::collections::HashSet;
use std::sync::atomic::AtomicBool;
use std::time::SystemTime;

/// OnePiece's own stable agent id — the only credential this gateway ever resolves, mirroring the
/// extraction gateway beside it.
const ONEPIECE_AGENT_ID: &str = "onepiece";

/// Picks which stored memories are worth injecting in full for one generation
/// (`add-two-tier-memory-recall`).
///
/// Reuses OnePiece's configured model rather than introducing a cheaper utility tier: VaneHub has
/// no such tier, and adding one means a provider-config field, a settings surface, and a Web
/// adapter change — a separate concern from the injection path this belongs to.
/// Borrows its dependencies and is built per generation from ports the generation path already
/// holds — the same shape as `wire_format_for(&provider_config)` beside it. Threading a fourth
/// port down from the composition root would have added a parameter to four nested signatures for
/// something whose inputs are already in scope exactly where it is used.
pub(crate) struct RuntimeAgentMemorySelectionAdapter<'a> {
    credentials: &'a dyn ApiCredentialPort,
    config: &'a dyn ApiAgentGateway,
}

impl<'a> RuntimeAgentMemorySelectionAdapter<'a> {
    pub(crate) fn new(
        credentials: &'a dyn ApiCredentialPort,
        config: &'a dyn ApiAgentGateway,
    ) -> Self {
        Self {
            credentials,
            config,
        }
    }
}

impl AgentMemorySelectionPort for RuntimeAgentMemorySelectionAdapter<'_> {
    fn select(
        &self,
        query: &str,
        candidates: &[AgentMemory],
    ) -> Result<Vec<String>, AgentRuntimeApplicationError> {
        if candidates.is_empty() {
            return Ok(Vec::new());
        }
        let api_key = self.credentials.fetch(ONEPIECE_AGENT_ID)?.ok_or_else(|| {
            AgentRuntimeApplicationError::Credential(
                "OnePiece has no configured credential.".to_string(),
            )
        })?;
        let provider_config = self
            .config
            .provider_config(ONEPIECE_AGENT_ID)?
            .ok_or_else(|| {
                AgentRuntimeApplicationError::Credential(
                    "OnePiece has no configured provider.".to_string(),
                )
            })?;
        let wire_format = wire_format_for(&provider_config)
            .map_err(|error| AgentRuntimeApplicationError::Memory(error.to_string()))?;
        let client = blocking_http_client(REQUEST_TIMEOUT)
            .map_err(|error| AgentRuntimeApplicationError::Memory(error.to_string()))?;

        let manifest = render_selection_manifest(candidates, SystemTime::now());
        let turns = vec![json!({
            "role": "user",
            "content": format!("Request:\n{query}\n\nAvailable memories:\n{manifest}"),
        })];
        let cancelled = AtomicBool::new(false);
        let response = summarize_turns(
            &wire_format,
            &client,
            &api_key,
            &provider_config.model_id,
            None,
            &turns,
            MEMORY_SELECTION_INSTRUCTION,
            &cancelled,
        )
        .map_err(AgentRuntimeApplicationError::Memory)?;

        // No response at all is the same as selecting nothing: the call worked and judged none of
        // them clearly useful, which is the expected outcome most of the time.
        let Some(response) = response else {
            return Ok(Vec::new());
        };
        let available = candidates
            .iter()
            .map(|memory| memory.name.clone())
            .collect::<HashSet<_>>();
        Ok(parse_memory_selection(&response, &available))
    }
}

/// Type, name, age, and description — never a body.
///
/// This is what keeps the call's cost proportional to how many memories exist rather than to how
/// large they are, which is the entire reason the index and the bodies are separate surfaces.
fn render_selection_manifest(candidates: &[AgentMemory], now: SystemTime) -> String {
    candidates
        .iter()
        .map(|memory| {
            let tag = memory
                .memory_type
                .map(|memory_type| format!("[{}] ", memory_type.as_str()))
                .unwrap_or_default();
            let age = render_memory_age(memory.modified_at, now)
                .map(|age| format!(" ({age})"))
                .unwrap_or_default();
            format!("- {tag}{}{age} - {}", memory.name, memory.description)
        })
        .collect::<Vec<_>>()
        .join("\n")
}
