use tauri::State;

use crate::{shell, AppError};

#[tauri::command]
pub(crate) fn shell_input(
    manager: State<'_, shell::ShellManager>,
    shell_id: String,
    content: String,
) -> Result<(), AppError> {
    shell::shell_input(manager, shell_id, content)
}
