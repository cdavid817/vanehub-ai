use super::api_process_adapter::{summarize_turns, wire_format_for, REQUEST_TIMEOUT};
use crate::contexts::agent_runtime::application::{
    AgentMemoryExtractionPort, AgentRuntimeApplicationError, ApiAgentGateway, ApiCredentialPort,
};
use crate::contexts::agent_runtime::domain::{
    parse_memory_actions, ParsedMemoryActions, MEMORY_ACTIONS_INSTRUCTION,
};
use crate::platform::network::blocking_http_client;
use serde_json::json;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

/// OnePiece's own stable agent id (`add-personalization-settings`) — the only credential/provider
/// this gateway ever resolves, regardless of which CLI-wrapped agent's turn triggered extraction.
const ONEPIECE_AGENT_ID: &str = "onepiece";

/// Independent, on-demand memory extraction for CLI-wrapped agents (`add-cli-memory-support`
/// design.md D3). Unlike OnePiece's own `extract_memories`, which reuses credentials already
/// resolved for an in-flight OnePiece generation, this adapter resolves OnePiece's credentials
/// itself on every call — CLI-wrapped agents have no generation-scoped credential to reuse.
#[derive(Clone)]
pub(crate) struct RuntimeAgentMemoryExtractionAdapter {
    credentials: Arc<dyn ApiCredentialPort>,
    config: Arc<dyn ApiAgentGateway>,
}

impl RuntimeAgentMemoryExtractionAdapter {
    pub(crate) fn new(
        credentials: Arc<dyn ApiCredentialPort>,
        config: Arc<dyn ApiAgentGateway>,
    ) -> Self {
        Self {
            credentials,
            config,
        }
    }
}

impl AgentMemoryExtractionPort for RuntimeAgentMemoryExtractionAdapter {
    fn extract(
        &self,
        exchange: &str,
        existing: &str,
    ) -> Result<ParsedMemoryActions, AgentRuntimeApplicationError> {
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
        let turns = vec![json!({ "role": "user", "content": exchange })];
        let cancelled = AtomicBool::new(false);
        // The manifest is what makes an update expressible: without the existing names in front of
        // it the model can only ever propose a create, and the pool grows the way the row store's
        // did. It is appended to the instruction rather than sent as a turn so it stays out of the
        // exchange the model is being asked to summarize.
        let instruction = if existing.trim().is_empty() {
            MEMORY_ACTIONS_INSTRUCTION.to_string()
        } else {
            format!("{MEMORY_ACTIONS_INSTRUCTION}\n\nExisting memories:\n{existing}")
        };
        let response = summarize_turns(
            &wire_format,
            &client,
            &api_key,
            &provider_config.model_id,
            None,
            &turns,
            &instruction,
            &cancelled,
            // Unbounded, as before: an extraction's action list grows with how much of the
            // exchange was worth remembering, and truncating it silently drops memories.
            None,
        )
        .map_err(AgentRuntimeApplicationError::Memory)?;
        // No response at all is the same as an empty action list: the call worked and found
        // nothing. Only an unparseable response is a malfunction.
        let Some(response) = response else {
            return Ok(ParsedMemoryActions::default());
        };
        parse_memory_actions(&response).map_err(|error| {
            AgentRuntimeApplicationError::Memory(format!(
                "Memory extraction returned an unusable response: {error}"
            ))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contexts::agent_runtime::application::{
        ApiProviderConfig, RegisterApiAgentInput, StoredOnePieceProviderConfig,
        UpdateApiAgentInput, INTERFACE_FORMAT_OPENAI_COMPATIBLE,
    };
    use crate::contexts::agent_runtime::domain::AgentDefinition;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::thread;

    #[derive(Default)]
    struct FakeCredentials {
        api_key: Option<String>,
    }

    impl ApiCredentialPort for FakeCredentials {
        fn store(
            &self,
            _agent_id: &str,
            _api_key: &str,
        ) -> Result<(), AgentRuntimeApplicationError> {
            unreachable!("not exercised by this test")
        }

        fn fetch(&self, _agent_id: &str) -> Result<Option<String>, AgentRuntimeApplicationError> {
            Ok(self.api_key.clone())
        }

        fn remove(&self, _agent_id: &str) -> Result<(), AgentRuntimeApplicationError> {
            unreachable!("not exercised by this test")
        }
    }

    #[derive(Default)]
    struct FakeApiAgents {
        provider_config: Option<ApiProviderConfig>,
    }

    impl ApiAgentGateway for FakeApiAgents {
        fn register(
            &self,
            _agent_id: &str,
            _input: &RegisterApiAgentInput,
        ) -> Result<AgentDefinition, AgentRuntimeApplicationError> {
            unreachable!("not exercised by this test")
        }

        fn provider_config(
            &self,
            _agent_id: &str,
        ) -> Result<Option<ApiProviderConfig>, AgentRuntimeApplicationError> {
            Ok(self.provider_config.clone())
        }

        fn update(
            &self,
            _agent_id: &str,
            _input: &UpdateApiAgentInput,
        ) -> Result<AgentDefinition, AgentRuntimeApplicationError> {
            unreachable!("not exercised by this test")
        }

        fn delete(&self, _agent_id: &str) -> Result<(), AgentRuntimeApplicationError> {
            unreachable!("not exercised by this test")
        }
    }

    fn adapter(
        api_key: Option<&str>,
        base_url: Option<&str>,
    ) -> RuntimeAgentMemoryExtractionAdapter {
        RuntimeAgentMemoryExtractionAdapter::new(
            Arc::new(FakeCredentials {
                api_key: api_key.map(str::to_string),
            }),
            Arc::new(FakeApiAgents {
                provider_config: base_url.map(|base_url| ApiProviderConfig {
                    source_provider_id: None,
                    model_id: "deepseek-chat".to_string(),
                    interface_format: INTERFACE_FORMAT_OPENAI_COMPATIBLE.to_string(),
                    base_url: Some(base_url.to_string()),
                    auto_approve_tools: false,
                }),
            }),
        )
    }

    /// Spins up a one-shot local HTTP server returning `status`/`body` — a trimmed copy of
    /// `api_process_adapter`'s own `http_fixture` (not shared across files since its test helpers
    /// are private to that module's `mod tests`).
    fn http_fixture(status: &'static str, body: String) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind fixture");
        let address = listener.local_addr().expect("fixture address");
        thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept fixture request");
            read_fixture_request(&mut stream);
            let response = format!(
                "HTTP/1.1 {status}\r\nContent-Type: text/event-stream\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
            stream
                .write_all(response.as_bytes())
                .expect("write fixture response");
        });
        format!("http://{address}")
    }

    fn read_fixture_request(stream: &mut std::net::TcpStream) {
        let mut request = Vec::new();
        let mut buffer = [0_u8; 4096];
        loop {
            let count = stream.read(&mut buffer).expect("read fixture request");
            if count == 0 {
                break;
            }
            request.extend_from_slice(&buffer[..count]);
            let Some(header_end) = request.windows(4).position(|value| value == b"\r\n\r\n") else {
                continue;
            };
            let headers = String::from_utf8_lossy(&request[..header_end]);
            let content_length = headers
                .lines()
                .find_map(|line| {
                    let (name, value) = line.split_once(':')?;
                    name.eq_ignore_ascii_case("content-length")
                        .then(|| value.trim().parse::<usize>().ok())
                        .flatten()
                })
                .unwrap_or(0);
            if request.len() >= header_end + 4 + content_length {
                break;
            }
        }
    }

    fn sse_body(events: &[&str]) -> String {
        events
            .iter()
            .map(|event| format!("data: {event}\n\n"))
            .collect()
    }

    #[test]
    fn extract_fails_with_a_credential_error_when_onepiece_has_no_stored_key() {
        let adapter = adapter(None, Some("http://unused.invalid"));

        let error = adapter.extract("hello", "").expect_err("no credential");

        assert!(matches!(error, AgentRuntimeApplicationError::Credential(_)));
    }

    #[test]
    fn extract_fails_with_a_credential_error_when_onepiece_has_no_provider_configured() {
        let adapter = adapter(Some("sk-test"), None);

        let error = adapter.extract("hello", "").expect_err("no provider");

        assert!(matches!(error, AgentRuntimeApplicationError::Credential(_)));
    }

    #[test]
    fn extract_returns_the_parsed_actions_when_the_call_succeeds() {
        let address = http_fixture(
            "200 OK",
            sse_body(&[
                r#"{"choices":[{"index":0,"delta":{"content":"[{\"action\":\"create\",\"name\":\"npm-only\",\"description\":\"Uses npm\",\"body\":\"Never pnpm.\"}]"},"finish_reason":null}]}"#,
                "[DONE]",
            ]),
        );
        let adapter = adapter(Some("sk-test"), Some(&address));

        let extracted = adapter.extract("hello", "").expect("extract");

        assert_eq!(extracted.actions.len(), 1);
        assert_eq!(extracted.actions[0].name, "npm-only");
        assert_eq!(extracted.actions[0].body.as_deref(), Some("Never pnpm."));
    }

    #[test]
    fn extract_returns_an_empty_action_list_when_nothing_is_worth_remembering() {
        // No response body at all and an empty array both mean the call worked and found nothing.
        // Neither may surface as an error, since this is the common outcome.
        let address = http_fixture("200 OK", sse_body(&["[DONE]"]));
        let adapter = adapter(Some("sk-test"), Some(&address));

        let extracted = adapter.extract("hello", "").expect("extract");

        assert_eq!(extracted, ParsedMemoryActions::default());
    }

    #[test]
    fn an_unparseable_response_is_a_memory_error_rather_than_an_empty_result() {
        let address = http_fixture(
            "200 OK",
            sse_body(&[
                r#"{"choices":[{"index":0,"delta":{"content":"Nothing worth remembering here."},"finish_reason":null}]}"#,
                "[DONE]",
            ]),
        );
        let adapter = adapter(Some("sk-test"), Some(&address));

        let error = adapter
            .extract("hello", "")
            .expect_err("prose is not an action list");

        assert!(matches!(error, AgentRuntimeApplicationError::Memory(_)));
    }

    #[test]
    fn a_populated_manifest_does_not_break_the_call() {
        // The manifest is what makes an update expressible. That it reaches the prompt is asserted
        // where the prompt is built; here we only need the request to survive carrying one, since
        // `http_fixture` serves a single request and cannot observe two calls.
        let address = http_fixture("200 OK", sse_body(&["[DONE]"]));
        let adapter = adapter(Some("sk-test"), Some(&address));

        let extracted = adapter
            .extract("hello", "- [project] npm-only — Uses npm")
            .expect("populated manifest");

        assert_eq!(extracted, ParsedMemoryActions::default());
    }

    #[test]
    fn extract_fails_with_a_memory_error_when_the_call_itself_fails() {
        let address = http_fixture("500 Internal Server Error", String::new());
        let adapter = adapter(Some("sk-test"), Some(&address));

        let error = adapter.extract("hello", "").expect_err("call failure");

        assert!(matches!(error, AgentRuntimeApplicationError::Memory(_)));
    }

    /// Confirms the two failure classes are genuinely distinguishable — a missing credential
    /// never surfaces as `Memory`, and a real call failure never surfaces as `Credential`. This
    /// distinction is what lets the caller (`service.rs`'s CLI completion hook) log "OnePiece
    /// isn't configured" separately from "the extraction call itself failed".
    #[test]
    fn credential_and_call_failures_are_distinguishable_error_variants() {
        let missing_credential = adapter(None, Some("http://unused.invalid"))
            .extract("hello", "")
            .expect_err("no credential");
        let address = http_fixture("500 Internal Server Error", String::new());
        let call_failure = adapter(Some("sk-test"), Some(&address))
            .extract("hello", "")
            .expect_err("call failure");

        assert!(matches!(
            missing_credential,
            AgentRuntimeApplicationError::Credential(_)
        ));
        assert!(matches!(
            call_failure,
            AgentRuntimeApplicationError::Memory(_)
        ));
    }

    #[test]
    fn stored_onepiece_provider_config_default_still_returns_agent_not_found_for_this_fake() {
        // Sanity check that `FakeApiAgents` doesn't accidentally satisfy `ApiAgentGateway`'s
        // OnePiece-profile methods with anything other than the trait's own defaults, since this
        // gateway never calls them.
        let gateway = FakeApiAgents::default();
        let result: Result<StoredOnePieceProviderConfig, _> = gateway.onepiece_provider_config();
        assert!(result.is_err());
    }
}
