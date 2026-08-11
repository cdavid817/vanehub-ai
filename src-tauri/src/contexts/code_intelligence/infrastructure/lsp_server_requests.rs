use super::json_rpc_actor::{JsonRpcErrorObject, ServerRequestHandler};
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Mutex;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct LspClientRequestLimits {
    max_configuration_items: usize,
    max_registrations: usize,
    max_progress_tokens: usize,
    max_configuration_bytes: usize,
}

impl LspClientRequestLimits {
    pub(crate) fn new(
        max_configuration_items: usize,
        max_registrations: usize,
        max_progress_tokens: usize,
        max_configuration_bytes: usize,
    ) -> Result<Self, JsonRpcErrorObject> {
        if [
            max_configuration_items,
            max_registrations,
            max_progress_tokens,
            max_configuration_bytes,
        ]
        .contains(&0)
        {
            return Err(invalid_params());
        }
        Ok(Self {
            max_configuration_items,
            max_registrations,
            max_progress_tokens,
            max_configuration_bytes,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct LspServerRequestSnapshot {
    pub(crate) registration_count: usize,
    pub(crate) progress_token_count: usize,
}

#[derive(Default)]
struct HandlerState {
    registrations: BTreeMap<String, String>,
    progress_tokens: BTreeSet<String>,
}

pub(crate) struct LspServerRequestHandler {
    configuration: BTreeMap<String, Value>,
    limits: LspClientRequestLimits,
    state: Mutex<HandlerState>,
}

impl LspServerRequestHandler {
    pub(crate) fn new(
        configuration: BTreeMap<String, Value>,
        limits: LspClientRequestLimits,
    ) -> Result<Self, JsonRpcErrorObject> {
        if configuration.keys().any(|section| section.is_empty()) {
            return Err(invalid_params());
        }
        let configuration_bytes = serde_json::to_vec(&configuration)
            .map_err(|_| invalid_params())?
            .len();
        if configuration_bytes > limits.max_configuration_bytes {
            return Err(limit_exceeded());
        }
        Ok(Self {
            configuration,
            limits,
            state: Mutex::new(HandlerState::default()),
        })
    }

    pub(crate) fn snapshot(&self) -> Result<LspServerRequestSnapshot, JsonRpcErrorObject> {
        let state = self.state.lock().map_err(|_| internal_error())?;
        Ok(LspServerRequestSnapshot {
            registration_count: state.registrations.len(),
            progress_token_count: state.progress_tokens.len(),
        })
    }

    fn workspace_configuration(&self, params: Value) -> Result<Value, JsonRpcErrorObject> {
        let items = params
            .get("items")
            .and_then(Value::as_array)
            .ok_or_else(invalid_params)?;
        if items.len() > self.limits.max_configuration_items {
            return Err(limit_exceeded());
        }
        let values = items
            .iter()
            .map(|item| {
                item.get("section")
                    .and_then(Value::as_str)
                    .and_then(|section| self.configuration.get(section))
                    .cloned()
                    .unwrap_or(Value::Null)
            })
            .collect::<Vec<_>>();
        Ok(Value::Array(values))
    }

    fn register_capabilities(&self, params: Value) -> Result<Value, JsonRpcErrorObject> {
        let registrations = params
            .get("registrations")
            .and_then(Value::as_array)
            .ok_or_else(invalid_params)?;
        if registrations.len() > self.limits.max_registrations {
            return Err(limit_exceeded());
        }
        let mut additions = BTreeMap::new();
        for registration in registrations {
            let id = required_string(registration, "id")?;
            let method = required_string(registration, "method")?;
            additions.insert(id.to_owned(), method.to_owned());
        }
        let mut state = self.state.lock().map_err(|_| internal_error())?;
        let new_count = additions
            .keys()
            .filter(|id| !state.registrations.contains_key(*id))
            .count();
        if state.registrations.len() + new_count > self.limits.max_registrations {
            return Err(limit_exceeded());
        }
        state.registrations.extend(additions);
        Ok(Value::Null)
    }

    fn unregister_capabilities(&self, params: Value) -> Result<Value, JsonRpcErrorObject> {
        let unregistrations = params
            .get("unregisterations")
            .or_else(|| params.get("unregistrations"))
            .and_then(Value::as_array)
            .ok_or_else(invalid_params)?;
        if unregistrations.len() > self.limits.max_registrations {
            return Err(limit_exceeded());
        }
        let mut ids = Vec::with_capacity(unregistrations.len());
        for unregistration in unregistrations {
            ids.push(required_string(unregistration, "id")?.to_owned());
            let _ = required_string(unregistration, "method")?;
        }
        let mut state = self.state.lock().map_err(|_| internal_error())?;
        for id in ids {
            state.registrations.remove(&id);
        }
        Ok(Value::Null)
    }

    fn create_progress(&self, params: Value) -> Result<Value, JsonRpcErrorObject> {
        let token = params.get("token").ok_or_else(invalid_params)?;
        let key = progress_token_key(token)?;
        let mut state = self.state.lock().map_err(|_| internal_error())?;
        if !state.progress_tokens.contains(&key)
            && state.progress_tokens.len() >= self.limits.max_progress_tokens
        {
            return Err(limit_exceeded());
        }
        state.progress_tokens.insert(key);
        Ok(Value::Null)
    }
}

impl ServerRequestHandler for LspServerRequestHandler {
    fn handle(&self, method: &str, params: Value) -> Result<Value, JsonRpcErrorObject> {
        match method {
            "workspace/configuration" => self.workspace_configuration(params),
            "client/registerCapability" => self.register_capabilities(params),
            "client/unregisterCapability" => self.unregister_capabilities(params),
            "window/workDoneProgress/create" => self.create_progress(params),
            "window/showMessageRequest" => Ok(Value::Null),
            "workspace/applyEdit" => Ok(json!({
                "applied": false,
                "failureReason": "read_only_client",
            })),
            _ => Err(JsonRpcErrorObject::method_not_found()),
        }
    }
}

fn required_string<'a>(value: &'a Value, field: &str) -> Result<&'a str, JsonRpcErrorObject> {
    value
        .get(field)
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .ok_or_else(invalid_params)
}

fn progress_token_key(token: &Value) -> Result<String, JsonRpcErrorObject> {
    let key = if let Some(value) = token.as_str() {
        format!("s:{value}")
    } else if let Some(value) = token.as_i64() {
        format!("n:{value}")
    } else if let Some(value) = token.as_u64() {
        format!("n:{value}")
    } else {
        return Err(invalid_params());
    };
    if key.len() > 256 {
        return Err(limit_exceeded());
    }
    Ok(key)
}

const fn invalid_params() -> JsonRpcErrorObject {
    JsonRpcErrorObject::new(-32602, "Invalid params")
}

const fn internal_error() -> JsonRpcErrorObject {
    JsonRpcErrorObject::new(-32603, "Internal error")
}

const fn limit_exceeded() -> JsonRpcErrorObject {
    JsonRpcErrorObject::new(-32001, "Client request limit exceeded")
}
