use super::relay_failure::RelayFailure;
use super::relay_jsonrpc::{CorrelatedRequest, RelayDirection};
use super::relay_observer::{RelayObserver, RelayRequest};
use super::relay_stdio_pump::{finish_pending, ClosableWriter};
use std::io::Write;

pub(super) fn emit_timeout<W1: Write, W2: Write>(
    expired: &CorrelatedRequest<Option<RelayRequest>>,
    child_input: &ClosableWriter<W1>,
    parent_output: &ClosableWriter<W2>,
) -> Result<(), String> {
    let mut bytes = serde_json::to_vec(&serde_json::json!({
        "jsonrpc": "2.0",
        "id": expired.id.to_value(),
        "error": { "code": -32001, "message": "MCP request timed out" }
    }))
    .map_err(|error| error.to_string())?;
    bytes.push(b'\n');
    match expired.direction {
        RelayDirection::ParentToUpstream => write_response(parent_output.clone(), &bytes),
        RelayDirection::UpstreamToParent => write_response(child_input.clone(), &bytes),
    }
}

pub(super) fn emit_pending_failures<W1: Write, W2: Write>(
    failure: RelayFailure,
    pending: &[CorrelatedRequest<Option<RelayRequest>>],
    child_input: &ClosableWriter<W1>,
    parent_output: &ClosableWriter<W2>,
) -> Result<(), String> {
    for request in pending {
        let result = match request.direction {
            RelayDirection::ParentToUpstream => {
                let mut output = parent_output.clone();
                failure.write_response(&mut output, &request.id)
            }
            RelayDirection::UpstreamToParent => {
                let mut input = child_input.clone();
                failure.write_response(&mut input, &request.id)
            }
        };
        result.map_err(|error| error.to_string())?;
    }
    Ok(())
}

pub(super) fn finish_correlated(
    observer: Option<&RelayObserver>,
    pending: Vec<CorrelatedRequest<Option<RelayRequest>>>,
    classification: &'static str,
) {
    for request in pending {
        finish_pending(observer, request.pending, false, Some(classification));
    }
}

fn write_response(mut target: impl Write, bytes: &[u8]) -> Result<(), String> {
    target
        .write_all(bytes)
        .and_then(|()| target.flush())
        .map_err(|error| error.to_string())
}
