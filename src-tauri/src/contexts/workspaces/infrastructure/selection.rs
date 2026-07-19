use crate::contexts::workspaces::application::{
    ProjectDirectorySelectionPort, WorkspaceApplicationError,
};
use tauri::AppHandle;
use tauri_plugin_dialog::DialogExt;

#[derive(Clone)]
pub(crate) struct TauriProjectDirectorySelection {
    app: AppHandle,
}

impl TauriProjectDirectorySelection {
    pub(crate) fn new(app: AppHandle) -> Self {
        Self { app }
    }
}

impl ProjectDirectorySelectionPort for TauriProjectDirectorySelection {
    fn select_directory(&self) -> Result<Option<String>, WorkspaceApplicationError> {
        self.app
            .dialog()
            .file()
            .blocking_pick_folder()
            .map(|selection| {
                selection
                    .into_path()
                    .map(|path| path.to_string_lossy().to_string())
                    .map_err(|error| WorkspaceApplicationError::Selection(error.to_string()))
            })
            .transpose()
    }
}
