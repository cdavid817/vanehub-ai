use super::{
    NativeToolDispatchError, NativeToolDispatchRequest, NativeToolErrorCode,
    NativeToolExecutionMode, NativeToolHandler,
};

pub(super) fn revalidate_context(
    handler: &dyn NativeToolHandler,
    request: &NativeToolDispatchRequest,
) -> Result<(), NativeToolDispatchError> {
    let authority = &request.authority;
    let execution = &request.execution;
    if authority.agent_id != execution.agent_id
        || authority.session_id != execution.session_id
        || authority.generation_id != execution.generation_id
    {
        return Err(dispatch_error(
            NativeToolErrorCode::Ineligible,
            "The native tool execution ownership changed.",
        ));
    }
    if authority.canonical_workspace != execution.canonical_workspace {
        return Err(dispatch_error(
            NativeToolErrorCode::Conflict,
            "The native tool workspace changed.",
        ));
    }
    if authority.execution_mode == NativeToolExecutionMode::Plan
        && !handler.definition().plan_mode_compatible
    {
        return Err(dispatch_error(
            NativeToolErrorCode::PermissionDenied,
            "The native tool is unavailable in plan mode.",
        ));
    }
    if execution.is_cancelled() {
        return Err(dispatch_error(
            NativeToolErrorCode::Cancelled,
            "The native tool call was cancelled.",
        ));
    }
    if execution.deadline_reached() {
        return Err(dispatch_error(
            NativeToolErrorCode::DeadlineExceeded,
            "The native tool deadline was reached.",
        ));
    }
    Ok(())
}

fn dispatch_error(code: NativeToolErrorCode, message: &str) -> NativeToolDispatchError {
    NativeToolDispatchError {
        code,
        safe_message: message.to_owned(),
    }
}
