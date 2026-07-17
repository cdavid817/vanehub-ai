use tauri::State;

use crate::{shell, AppError};

#[tauri::command]
pub(crate) fn shell_resize(
    manager: State<'_, shell::ShellManager>,
    input: shell::ResizeShellInput,
) -> Result<(), AppError> {
    shell::shell_resize(manager, input)
}
