use super::relay_failure::RelayFailure;
use super::relay_jsonrpc::JsonRpcFrame;
use super::relay_observer::{RelayObserver, RelayRequest};

pub(super) fn observed_request(
    observer: Option<&RelayObserver>,
    frame: &JsonRpcFrame,
) -> Option<RelayRequest> {
    let method = match frame {
        JsonRpcFrame::Request { method, .. } | JsonRpcFrame::Notification { method } => method,
        JsonRpcFrame::Response { .. } => return None,
    };
    observer.and_then(|observer| observer.start_request("streamable_http", Some(method)))
}

pub(super) fn finish_observed(
    observer: Option<&RelayObserver>,
    request: Option<RelayRequest>,
    failure: Option<RelayFailure>,
) {
    if let (Some(observer), Some(request)) = (observer, request) {
        observer.finish_request(
            request,
            failure.is_none(),
            failure.map(RelayFailure::classification),
        );
    }
}
