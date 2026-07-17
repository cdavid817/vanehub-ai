use tauri::{AppHandle, State};

use crate::{shell, AppError};

#[tauri::command]
pub(crate) fn shell_kill(
    app: AppHandle,
    manager: State<'_, shell::ShellManager>,
    shell_id: String,
) -> Result<(), AppError> {
    shell::shell_kill(app, manager, shell_id)
}
