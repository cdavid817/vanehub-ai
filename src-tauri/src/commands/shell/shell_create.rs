use std::sync::Mutex;

use tauri::{AppHandle, State};

use crate::{shell, AppError, RegistryStore};

#[tauri::command]
pub(crate) fn shell_create(
    app: AppHandle,
    state: State<'_, Mutex<RegistryStore>>,
    manager: State<'_, shell::ShellManager>,
    input: shell::CreateShellInput,
) -> Result<shell::ShellSession, AppError> {
    shell::shell_create(app, state, manager, input)
}
