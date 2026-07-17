use tauri::State;

use crate::{shell, AppError};

#[tauri::command]
pub(crate) fn shell_cd(
    manager: State<'_, shell::ShellManager>,
    shell_id: String,
) -> Result<(), AppError> {
    shell::shell_cd(manager, shell_id)
}
