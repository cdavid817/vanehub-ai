use super::dto::ResolvePendingApprovalInput;
use super::mapper::{approval_decision, parse_scope};
use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::permissions::api::ResolveApprovalUseCase;
use std::sync::Arc;
use tauri::State;

/// Validates the DTO, calls one use case, returns its typed outcome token.
///
/// It used to hold both context facades and orchestrate them: deliver the decision to the waiting
/// Agent or hook, then ask `permissions` to finalize using whether that delivery landed. That put
/// the ordering of a security-critical two-phase operation — and the decision about what
/// "delivered" meant — in a transport adapter, and it put the irreversible half first. Both now
/// belong to `ResolveApprovalUseCase`, which is the only thing that can guarantee no `Allow`
/// reaches a waiter before its evidence is durable.
#[tauri::command]
pub(crate) fn resolve_pending_approval(
    resolver: State<'_, Arc<ResolveApprovalUseCase>>,
    input: ResolvePendingApprovalInput,
) -> Result<String, CommandError> {
    resolver
        .resolve(
            &input.request_id,
            approval_decision(input.approved),
            parse_scope(&input.scope),
        )
        .map(|outcome| outcome.token().to_string())
        .map_err(map_command_error)
}
