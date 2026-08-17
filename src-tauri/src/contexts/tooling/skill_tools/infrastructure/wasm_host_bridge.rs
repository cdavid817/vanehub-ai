use crate::contexts::tooling::skill_tools::application::{
    SkillToolDispatchOutcome, SkillToolModuleHostCallPort,
};
use serde_json::{json, Value};
use std::sync::Arc;
use wasmtime::{Caller, Extern, Linker, StoreLimits};

const HOST_MODULE: &str = "vanehub";
const HOST_FUNCTION: &str = "host_call";
const INVALID_REQUEST: i32 = -1;
const RESPONSE_TOO_LARGE: i32 = -2;
const DISPATCH_FAILED: i32 = -3;

pub(crate) struct ModuleStoreState {
    pub(crate) limits: StoreLimits,
    pub(crate) host_calls: Arc<dyn SkillToolModuleHostCallPort>,
    pub(crate) maximum_payload_bytes: usize,
}

pub(crate) fn install_host_call(linker: &mut Linker<ModuleStoreState>) -> Result<(), String> {
    linker
        .func_wrap(
            HOST_MODULE,
            HOST_FUNCTION,
            |mut caller: Caller<'_, ModuleStoreState>,
             request_pointer: i32,
             request_length: i32,
             response_pointer: i32,
             response_capacity: i32|
             -> i32 {
                host_call(
                    &mut caller,
                    request_pointer,
                    request_length,
                    response_pointer,
                    response_capacity,
                )
            },
        )
        .map(|_| ())
        .map_err(|error| error.to_string())
}

fn host_call(
    caller: &mut Caller<'_, ModuleStoreState>,
    request_pointer: i32,
    request_length: i32,
    response_pointer: i32,
    response_capacity: i32,
) -> i32 {
    let Some((request_pointer, request_length, response_pointer, response_capacity)) =
        checked_ranges(
            request_pointer,
            request_length,
            response_pointer,
            response_capacity,
            caller.data().maximum_payload_bytes,
        )
    else {
        return INVALID_REQUEST;
    };
    let Some(memory) = caller.get_export("memory").and_then(Extern::into_memory) else {
        return INVALID_REQUEST;
    };
    let mut request_bytes = vec![0; request_length];
    if memory
        .read(&*caller, request_pointer, &mut request_bytes)
        .is_err()
    {
        return INVALID_REQUEST;
    }
    let Ok(request) = serde_json::from_slice::<Value>(&request_bytes) else {
        return INVALID_REQUEST;
    };
    let outcome = match caller.data().host_calls.call(&request) {
        Ok(outcome) => outcome,
        Err(_) => return DISPATCH_FAILED,
    };
    let Ok(response) = serde_json::to_vec(&outcome_value(outcome)) else {
        return DISPATCH_FAILED;
    };
    if response.len() > response_capacity || response.len() > caller.data().maximum_payload_bytes {
        return RESPONSE_TOO_LARGE;
    }
    if memory.write(caller, response_pointer, &response).is_err() {
        return INVALID_REQUEST;
    }
    i32::try_from(response.len()).unwrap_or(RESPONSE_TOO_LARGE)
}

fn checked_ranges(
    request_pointer: i32,
    request_length: i32,
    response_pointer: i32,
    response_capacity: i32,
    maximum: usize,
) -> Option<(usize, usize, usize, usize)> {
    let values = [
        request_pointer,
        request_length,
        response_pointer,
        response_capacity,
    ];
    if values.iter().any(|value| *value < 0) {
        return None;
    }
    let request_length = usize::try_from(request_length).ok()?;
    let response_capacity = usize::try_from(response_capacity).ok()?;
    if request_length > maximum || response_capacity > maximum {
        return None;
    }
    Some((
        usize::try_from(request_pointer).ok()?,
        request_length,
        usize::try_from(response_pointer).ok()?,
        response_capacity,
    ))
}

fn outcome_value(outcome: SkillToolDispatchOutcome) -> Value {
    match outcome {
        SkillToolDispatchOutcome::Completed(value) => {
            json!({"status": "completed", "value": value})
        }
        SkillToolDispatchOutcome::Denied { reason } => {
            json!({"status": "denied", "reason": reason})
        }
        SkillToolDispatchOutcome::Failed { code } => json!({"status": "failed", "code": code}),
        SkillToolDispatchOutcome::Cancelled => json!({"status": "cancelled"}),
    }
}
