use super::api_process_adapter::{summarize_turns, wire_format_for, REQUEST_TIMEOUT};
use crate::contexts::agent_runtime::application::{
    AgentRuntimeApplicationError, ApiAgentGateway, ApiCredentialPort, ApiProviderConfig,
    OnePiecePlanningPort, OnePiecePlanningRequest, OnePiecePlanningResult,
};
use crate::platform::network::blocking_http_client;
use serde_json::json;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

const SUPPORTED_INSTRUCTION_VERSION: u32 = 2;
const MAX_PLANNER_PROMPT_CHARACTERS: usize = 64_000;
const MAX_PLANNER_RESPONSE_CHARACTERS: usize = 128_000;
const PLANNER_COMPLETION_INSTRUCTION: &str =
    "Generate the requested Plan now. Return only the JSON object required by the schema.";
const PLANNING_DISCOVERY_TOOLS: &[&str] = &[
    "file",
    "grep",
    "glob",
    "search_code",
    "lsp_definition",
    "lsp_references",
    "lsp_hover",
    "lsp_symbols",
];

#[derive(Clone)]
pub(crate) struct RuntimeOnePiecePlanningAdapter {
    credentials: Arc<dyn ApiCredentialPort>,
    profiles: Arc<dyn ApiAgentGateway>,
}

impl RuntimeOnePiecePlanningAdapter {
    pub(crate) fn new(
        credentials: Arc<dyn ApiCredentialPort>,
        profiles: Arc<dyn ApiAgentGateway>,
    ) -> Self {
        Self {
            credentials,
            profiles,
        }
    }
}

impl OnePiecePlanningPort for RuntimeOnePiecePlanningAdapter {
    fn generate(
        &self,
        request: &OnePiecePlanningRequest,
    ) -> Result<OnePiecePlanningResult, AgentRuntimeApplicationError> {
        validate_request(request)?;
        let profile = self
            .profiles
            .list_onepiece_provider_profiles()?
            .into_iter()
            .find(|profile| profile.active)
            .ok_or_else(readiness_error)?;
        let credential_key = format!("onepiece-profile:{}", profile.id);
        let credential = match self.credentials.fetch(&credential_key)? {
            Some(credential) => Some(credential),
            None => self.credentials.fetch("onepiece")?,
        }
        .ok_or_else(readiness_error)?;
        let provider = ApiProviderConfig {
            source_provider_id: profile.source_provider_id.clone(),
            model_id: profile.model_id.clone(),
            interface_format: profile.interface_format.clone(),
            base_url: profile.base_url.clone(),
            auto_approve_tools: false,
        };
        let wire_format = wire_format_for(&provider)
            .map_err(|error| AgentRuntimeApplicationError::Generation(error.to_string()))?;
        let client = blocking_http_client(REQUEST_TIMEOUT)
            .map_err(|error| AgentRuntimeApplicationError::Generation(error.to_string()))?;
        let cancelled = AtomicBool::new(false);
        let content = summarize_turns(
            &wire_format,
            &client,
            &credential,
            &profile.model_id,
            Some(request.prompt.trim()),
            &[json!({ "role": "user", "content": "Create the Plan draft." })],
            PLANNER_COMPLETION_INSTRUCTION,
            &cancelled,
        )
        .map_err(AgentRuntimeApplicationError::Generation)?
        .ok_or_else(|| {
            AgentRuntimeApplicationError::Generation(
                "OnePiece planner returned no structured content.".to_string(),
            )
        })?;
        if content.chars().count() > MAX_PLANNER_RESPONSE_CHARACTERS {
            return Err(AgentRuntimeApplicationError::Generation(
                "OnePiece planner response exceeded the bounded response size.".to_string(),
            ));
        }
        Ok(OnePiecePlanningResult {
            content,
            profile_id: profile.id,
            model_id: profile.model_id,
        })
    }
}

fn validate_request(request: &OnePiecePlanningRequest) -> Result<(), AgentRuntimeApplicationError> {
    if request.instruction_version != SUPPORTED_INSTRUCTION_VERSION {
        return Err(AgentRuntimeApplicationError::Validation(format!(
            "Unsupported OnePiece planner instruction version {}.",
            request.instruction_version
        )));
    }
    let prompt_length = request.prompt.trim().chars().count();
    if prompt_length == 0 || prompt_length > MAX_PLANNER_PROMPT_CHARACTERS {
        return Err(AgentRuntimeApplicationError::Validation(
            "OnePiece planner prompt must be non-empty and within the bounded request size."
                .to_string(),
        ));
    }
    if request.bounded_root.trim().is_empty()
        || request.tool_call_limit == 0
        || request.token_budget == 0
        || request.timeout_seconds == 0
    {
        return Err(AgentRuntimeApplicationError::Validation(
            "OnePiece discovery requires a bounded root and positive limits.".to_string(),
        ));
    }
    let expected = PLANNING_DISCOVERY_TOOLS
        .iter()
        .map(|tool| (*tool).to_string())
        .collect::<Vec<_>>();
    if request.permitted_tools != expected {
        return Err(AgentRuntimeApplicationError::Validation(
            "OnePiece discovery must use the exact read-only tool profile.".to_string(),
        ));
    }
    Ok(())
}

fn readiness_error() -> AgentRuntimeApplicationError {
    AgentRuntimeApplicationError::Credential(
        "OnePiece planning requires an active Profile with a stored credential.".to_string(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contexts::agent_runtime::application::{
        RegisterApiAgentInput, StoredOnePieceProviderProfile, UpdateApiAgentInput,
        INTERFACE_FORMAT_OPENAI_COMPATIBLE,
    };
    use crate::contexts::agent_runtime::domain::AgentDefinition;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::thread;

    struct FakeCredentials {
        credential: Option<String>,
    }

    impl ApiCredentialPort for FakeCredentials {
        fn store(
            &self,
            _agent_id: &str,
            _api_key: &str,
        ) -> Result<(), AgentRuntimeApplicationError> {
            panic!("planning must never persist a credential")
        }

        fn fetch(&self, _agent_id: &str) -> Result<Option<String>, AgentRuntimeApplicationError> {
            Ok(self.credential.clone())
        }

        fn remove(&self, _agent_id: &str) -> Result<(), AgentRuntimeApplicationError> {
            panic!("planning must never remove a credential")
        }
    }

    struct FakeProfiles {
        profile: Option<StoredOnePieceProviderProfile>,
    }

    impl ApiAgentGateway for FakeProfiles {
        fn register(
            &self,
            _agent_id: &str,
            _input: &RegisterApiAgentInput,
        ) -> Result<AgentDefinition, AgentRuntimeApplicationError> {
            unreachable!("not used by planning")
        }

        fn provider_config(
            &self,
            _agent_id: &str,
        ) -> Result<Option<ApiProviderConfig>, AgentRuntimeApplicationError> {
            unreachable!("planning captures the active profile directly")
        }

        fn update(
            &self,
            _agent_id: &str,
            _input: &UpdateApiAgentInput,
        ) -> Result<AgentDefinition, AgentRuntimeApplicationError> {
            unreachable!("not used by planning")
        }

        fn delete(&self, _agent_id: &str) -> Result<(), AgentRuntimeApplicationError> {
            unreachable!("not used by planning")
        }

        fn list_onepiece_provider_profiles(
            &self,
        ) -> Result<Vec<StoredOnePieceProviderProfile>, AgentRuntimeApplicationError> {
            Ok(self.profile.clone().into_iter().collect())
        }
    }

    fn request() -> OnePiecePlanningRequest {
        OnePiecePlanningRequest {
            instruction_version: 2,
            prompt: "Return a Plan JSON object.".to_string(),
            bounded_root: "C:\\workspace".to_string(),
            permitted_tools: PLANNING_DISCOVERY_TOOLS
                .iter()
                .map(|tool| (*tool).to_string())
                .collect(),
            tool_call_limit: 40,
            token_budget: 24_000,
            timeout_seconds: 120,
        }
    }

    fn adapter(
        profile: Option<StoredOnePieceProviderProfile>,
        credential: Option<&str>,
    ) -> RuntimeOnePiecePlanningAdapter {
        RuntimeOnePiecePlanningAdapter::new(
            Arc::new(FakeCredentials {
                credential: credential.map(str::to_string),
            }),
            Arc::new(FakeProfiles { profile }),
        )
    }

    fn active_profile(base_url: &str) -> StoredOnePieceProviderProfile {
        StoredOnePieceProviderProfile {
            id: "profile-1".to_string(),
            name: "Ready profile".to_string(),
            source_preset_id: None,
            source_provider_id: None,
            source_endpoint_type: None,
            source_preset_version: None,
            provider: "fixture".to_string(),
            model_id: "fixture-model".to_string(),
            interface_format: INTERFACE_FORMAT_OPENAI_COMPATIBLE.to_string(),
            base_url: Some(base_url.to_string()),
            active: true,
        }
    }

    #[test]
    fn rejects_unsupported_versions_and_unready_profiles_before_generation() {
        let unavailable = adapter(None, None);
        assert!(matches!(
            unavailable.generate(&request()),
            Err(AgentRuntimeApplicationError::Credential(_))
        ));

        let mut unsupported = request();
        unsupported.instruction_version = 99;
        assert!(matches!(
            unavailable.generate(&unsupported),
            Err(AgentRuntimeApplicationError::Validation(_))
        ));
    }

    #[test]
    fn discovery_profile_is_workspace_bounded_read_only_and_positive() {
        validate_request(&request()).expect("valid discovery profile");
        for prohibited in ["shell", "edit", "remember", "recall", "mcp"] {
            let mut invalid = request();
            invalid.permitted_tools.push(prohibited.to_string());
            assert!(matches!(
                validate_request(&invalid),
                Err(AgentRuntimeApplicationError::Validation(_))
            ));
        }
        let mut unbounded = request();
        unbounded.bounded_root.clear();
        assert!(validate_request(&unbounded).is_err());
        let mut unlimited = request();
        unlimited.tool_call_limit = 0;
        assert!(validate_request(&unlimited).is_err());
    }

    #[test]
    fn captures_active_profile_and_sends_a_tool_less_request() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind fixture");
        let address = listener.local_addr().expect("fixture address");
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept fixture request");
            let request = read_request(&mut stream);
            let body = concat!(
                "data: {\"choices\":[{\"index\":0,\"delta\":{\"content\":\"{\\\"subtasks\\\":[],\\\"dependencies\\\":[]}\"},\"finish_reason\":null}]}\n\n",
                "data: [DONE]\n\n"
            );
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
            stream
                .write_all(response.as_bytes())
                .expect("write response");
            request
        });
        let runtime = adapter(
            Some(active_profile(&format!("http://{address}"))),
            Some("secret-never-persisted"),
        );

        let result = runtime.generate(&request()).expect("planning result");
        let request_bytes = server.join().expect("fixture server");
        let request_text = String::from_utf8_lossy(&request_bytes);
        let body = request_text
            .split_once("\r\n\r\n")
            .map(|(_, body)| body)
            .expect("request body");
        let json: serde_json::Value = serde_json::from_str(body).expect("request json");

        assert_eq!(result.profile_id, "profile-1");
        assert_eq!(result.model_id, "fixture-model");
        assert!(json.get("tools").is_none());
        assert!(!request_text.contains("onepiece-profile:profile-1"));
    }

    fn read_request(stream: &mut std::net::TcpStream) -> Vec<u8> {
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
        request
    }
}
