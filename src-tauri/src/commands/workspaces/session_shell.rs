//! The retained Session Shell commands.
//!
//! Grouped in one file because they are one contract: eight operations on the same registry, each
//! a thin translation of a parsed request into an application call. Splitting them across eight
//! files would spread one contract over eight places without giving any of them a decision to make.

use super::session_shell_dto as dto;
use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::workspaces::api::WorkspaceApi;
use tauri::State;

#[tauri::command]
pub(crate) fn list_session_shells(
    api: State<'_, WorkspaceApi>,
    session_id: String,
) -> Result<Vec<dto::SessionShell>, CommandError> {
    Ok(api
        .list_session_shells(&session_id)
        .into_iter()
        .map(dto::descriptor_to_dto)
        .collect())
}

#[tauri::command]
pub(crate) async fn create_session_shell(
    api: State<'_, WorkspaceApi>,
    input: dto::CreateSessionShellInput,
) -> Result<dto::SessionShell, CommandError> {
    let request = dto::create_request(input).map_err(map_command_error)?;
    api.create_session_shell_blocking(request)
        .await
        .map(dto::descriptor_to_dto)
        .map_err(map_command_error)
}

#[tauri::command]
pub(crate) fn attach_session_shell(
    api: State<'_, WorkspaceApi>,
    input: dto::AttachSessionShellInput,
) -> Result<dto::ShellAttachment, CommandError> {
    let request = dto::attach_request(input).map_err(map_command_error)?;
    api.attach_session_shell(&request)
        .map(dto::attachment_to_dto)
        .map_err(map_command_error)
}

#[tauri::command]
pub(crate) fn detach_session_shell(
    api: State<'_, WorkspaceApi>,
    input: dto::ShellAttachmentInput,
) -> Result<(), CommandError> {
    let scope = dto::attachment_scope(&input).map_err(map_command_error)?;
    api.detach_session_shell(&scope).map_err(map_command_error)
}

#[tauri::command]
pub(crate) async fn write_session_shell(
    api: State<'_, WorkspaceApi>,
    input: dto::WriteSessionShellInput,
) -> Result<(), CommandError> {
    let request = dto::write_request(input).map_err(map_command_error)?;
    api.write_session_shell_blocking(request)
        .await
        .map_err(map_command_error)
}

#[tauri::command]
pub(crate) async fn resize_session_shell(
    api: State<'_, WorkspaceApi>,
    input: dto::ResizeSessionShellInput,
) -> Result<(), CommandError> {
    let request = dto::resize_request(input).map_err(map_command_error)?;
    api.resize_session_shell_blocking(request)
        .await
        .map_err(map_command_error)
}

#[tauri::command]
pub(crate) fn rename_session_shell(
    api: State<'_, WorkspaceApi>,
    input: dto::RenameSessionShellInput,
) -> Result<dto::SessionShell, CommandError> {
    let shell_id = dto::shell_id(&input.shell_id).map_err(map_command_error)?;
    api.rename_session_shell(&shell_id, &input.title)
        .map(dto::descriptor_to_dto)
        .map_err(map_command_error)
}

/// Ends a Shell for good. The only path that does: a detached, hidden, or unmounted view leaves the
/// process running, and this is what the user reaches when they mean to stop it.
#[tauri::command]
pub(crate) async fn close_session_shell(
    api: State<'_, WorkspaceApi>,
    shell_id: String,
) -> Result<(), CommandError> {
    let shell_id = dto::shell_id(&shell_id).map_err(map_command_error)?;
    api.close_session_shell_blocking(shell_id)
        .await
        .map_err(map_command_error)
}
