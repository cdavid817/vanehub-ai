use crate::contexts::operations::application::{DiagnosticLog, DiagnosticLogPort, LogSeverity};
use crate::contexts::operations::infrastructure::UnifiedLoggingAdapter;
use crate::platform::database::NativeDatabase;
use crate::platform::logging;
use std::collections::BTreeMap;
use std::error::Error;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use tauri::Manager;

const AGENT_TERMINAL_IDLE_TIMEOUT_SECONDS: i64 = 2 * 60 * 60;

pub(crate) fn run() {
    let result = tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .setup(setup)
        .on_window_event(crate::contexts::desktop::infrastructure::handle_main_window_event)
        .invoke_handler(crate::commands::invoke_handler())
        .build(tauri::generate_context!());
    match result {
        Ok(app) => app.run(|app, event| {
            if matches!(event, tauri::RunEvent::Exit)
                && app
                    .try_state::<crate::contexts::execution_observability::infrastructure::ExecutionTelemetryLifecycle>()
                    .is_some_and(|lifecycle| lifecycle.shutdown().is_err())
            {
                write_bootstrap_log(
                    &logging::fallback_log_dir(),
                    LogSeverity::Warn,
                    "execution_observability.shutdown",
                    "Execution telemetry did not flush completely before the bounded shutdown deadline",
                );
            }
        }),
        Err(error) => write_bootstrap_log(
            &logging::fallback_log_dir(),
            LogSeverity::Error,
            "runtime.failure",
            &error.to_string(),
        ),
    }
}

fn setup(app: &mut tauri::App) -> Result<(), Box<dyn Error>> {
    let data_dir = match configured_app_data_dir(std::env::var_os("VANEHUB_APP_DATA_DIR"))? {
        Some(path) => path,
        None => app.path().app_data_dir().map_err(boxed_error)?,
    };
    std::env::set_var("VANEHUB_APP_DATA_DIR", &data_dir);
    let fallback_log_directory = logging::fallback_log_dir();
    let database = NativeDatabase::new(data_dir).map_err(boxed_error)?;

    let desktop_settings_api =
        super::assemble_desktop_settings_api(database.clone(), app.handle().clone());
    let floating_assistant_api = super::assemble_floating_assistant_api(
        database.clone(),
        app.handle().clone(),
        fallback_log_directory.clone(),
    );
    if let Err(error) = desktop_settings_api.activate_configured_log_directory() {
        write_bootstrap_log(
            &fallback_log_directory,
            LogSeverity::Warn,
            "settings.log-directory.sync",
            &error.to_string(),
        );
    }
    if let Err(error) = desktop_settings_api.sync_startup_preference() {
        write_bootstrap_log(
            &fallback_log_directory,
            LogSeverity::Warn,
            "settings.autostart.sync",
            &error.to_string(),
        );
    }
    let tray_language = desktop_settings_api
        .get_settings()
        .ok()
        .map(|view| view.settings.application_language().as_str().to_string())
        .unwrap_or_else(|| "zh-CN".to_string());

    let operations_api = super::assemble_operations_api();
    let cli_parameters_api =
        super::assemble_cli_parameters_api(database.clone(), fallback_log_directory.clone());
    let mcp_api = super::assemble_mcp_api(
        database.clone(),
        operations_api.clone(),
        fallback_log_directory.clone(),
    );
    let cli_api = super::assemble_cli_api(
        database.clone(),
        operations_api.clone(),
        fallback_log_directory.clone(),
    );
    let sdk_api = super::assemble_sdk_api(
        database.clone(),
        operations_api.clone(),
        fallback_log_directory.clone(),
    );
    let extension_api = super::assemble_extension_api(
        database.clone(),
        operations_api.clone(),
        fallback_log_directory.clone(),
    );
    let plugin_integration_api =
        super::assemble_plugin_integration_api(fallback_log_directory.clone());
    let skill_api = super::assemble_skill_api(database.clone(), fallback_log_directory.clone());
    let prompt_hook_api =
        super::assemble_prompt_hook_api(database.clone(), fallback_log_directory.clone());
    let ssh_connections_api = super::assemble_ssh_connections_api(database.clone());
    let workspace_api = super::assemble_workspace_api(
        database.clone(),
        app.handle().clone(),
        fallback_log_directory.clone(),
    );
    let (sessions_api, session_runtime_adapter) = super::assemble_sessions_api(
        database.clone(),
        operations_api.clone(),
        workspace_api.clone(),
        cli_parameters_api.clone(),
        fallback_log_directory.clone(),
    );
    let super::AgentRuntimeAssembly {
        api: agent_runtime_api,
        telemetry_lifecycle,
    } = super::assemble_agent_runtime_api(super::AgentRuntimeDependencies {
        database: database.clone(),
        app: app.handle().clone(),
        operations: operations_api.clone(),
        sdk: sdk_api.clone(),
        cli: cli_api.clone(),
        cli_parameters: cli_parameters_api.clone(),
        prompts: prompt_hook_api.clone(),
        sessions: sessions_api.clone(),
        workspaces: workspace_api.clone(),
        fallback_log_directory: fallback_log_directory.clone(),
    });
    let execution_observability_api = super::assemble_execution_observability_api(database.clone());
    agent_runtime_api
        .reconcile_loop_startup()
        .map_err(boxed_message)?;
    session_runtime_adapter
        .attach_agent_runtime(agent_runtime_api.clone())
        .map_err(boxed_message)?;

    let scheduled_task_database = database.clone();
    let execution_retention_database = database.clone();
    app.manage(database.clone());
    app.manage(super::ScheduledTaskLogDirectory::new(
        fallback_log_directory.clone(),
    ));

    let communications = super::assemble_communications(super::CommunicationsDependencies {
        database,
        operations: operations_api.clone(),
        agents: agent_runtime_api.clone(),
        sessions: sessions_api.clone(),
        workspaces: workspace_api.clone(),
        desktop_settings: desktop_settings_api.clone(),
        fallback_log_directory: fallback_log_directory.clone(),
    })
    .map_err(boxed_message)?;
    let communications_api = communications.api;
    let wechat_authorization_api = communications.wechat_authorization;

    app.manage(operations_api);
    app.manage(cli_api.clone());
    app.manage(cli_parameters_api);
    app.manage(mcp_api);
    app.manage(sdk_api);
    app.manage(extension_api);
    app.manage(plugin_integration_api);
    app.manage(skill_api);
    app.manage(prompt_hook_api);
    app.manage(ssh_connections_api);
    app.manage(workspace_api);
    app.manage(sessions_api.clone());
    app.manage(agent_runtime_api.clone());
    app.manage(telemetry_lifecycle);
    app.manage(execution_observability_api);
    app.manage(communications_api.clone());
    app.manage(wechat_authorization_api);
    app.manage(desktop_settings_api.clone());
    app.manage(floating_assistant_api.clone());

    super::start_scheduled_task_jobs(
        scheduled_task_database,
        sessions_api.clone(),
        agent_runtime_api.clone(),
        fallback_log_directory.clone(),
    );
    super::start_execution_retention_job(
        execution_retention_database,
        fallback_log_directory.clone(),
    );
    super::start_session_maintenance_jobs(
        sessions_api,
        desktop_settings_api,
        fallback_log_directory.clone(),
    );
    start_agent_terminal_cleanup_job(agent_runtime_api.clone());
    let desktop_lifecycle_api = super::assemble_desktop_lifecycle_api(
        app.handle().clone(),
        &tray_language,
        agent_runtime_api.clone(),
        communications_api.clone(),
        fallback_log_directory.clone(),
    );
    app.manage(desktop_lifecycle_api.clone());
    super::initialize_desktop_runtime(
        &desktop_lifecycle_api,
        &floating_assistant_api,
        fallback_log_directory.clone(),
    );
    super::start_initial_cli_refresh(cli_api).map_err(boxed_error)?;
    tauri::async_runtime::spawn(async move {
        if let Err(error) = communications_api.start_saved_connectors().await {
            write_bootstrap_log(
                &fallback_log_directory,
                LogSeverity::Error,
                "communications.startup",
                error.safe_code(),
            );
        }
    });
    Ok(())
}

fn start_agent_terminal_cleanup_job(
    agent_runtime_api: crate::contexts::agent_runtime::api::AgentRuntimeApi,
) {
    tauri::async_runtime::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(60));
        loop {
            interval.tick().await;
            let _ =
                agent_runtime_api.cleanup_idle_agent_terminals(AGENT_TERMINAL_IDLE_TIMEOUT_SECONDS);
        }
    });
}

fn configured_app_data_dir(value: Option<OsString>) -> Result<Option<PathBuf>, Box<dyn Error>> {
    let Some(value) = value.filter(|value| !value.is_empty()) else {
        return Ok(None);
    };
    let path = PathBuf::from(value);
    if !path.is_absolute() {
        return Err(boxed_message(
            "VANEHUB_APP_DATA_DIR must be an absolute path",
        ));
    }
    Ok(Some(path))
}

fn write_bootstrap_log(
    fallback_log_directory: &Path,
    severity: LogSeverity,
    category: &str,
    message: &str,
) {
    let adapter = UnifiedLoggingAdapter::active(fallback_log_directory.to_path_buf());
    let mut context = BTreeMap::new();
    context.insert("source".to_string(), "native".to_string());
    let _ = adapter.write_diagnostic(DiagnosticLog {
        severity,
        category: category.to_string(),
        message: message.to_string(),
        context,
    });
}

fn boxed_error(error: impl Error + 'static) -> Box<dyn Error> {
    Box::new(error)
}

fn boxed_message(message: impl std::fmt::Display) -> Box<dyn Error> {
    Box::new(std::io::Error::other(message.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_an_absolute_app_data_override() {
        let path = std::env::current_dir()
            .expect("current directory")
            .join("isolated-app-data");

        assert_eq!(
            configured_app_data_dir(Some(path.clone().into_os_string()))
                .expect("absolute override"),
            Some(path)
        );
    }

    #[test]
    fn ignores_an_empty_app_data_override() {
        assert_eq!(
            configured_app_data_dir(Some(OsString::new())).expect("empty override"),
            None
        );
    }

    #[test]
    fn rejects_a_relative_app_data_override() {
        let error = configured_app_data_dir(Some(OsString::from("relative-data")))
            .expect_err("relative override");

        assert_eq!(
            error.to_string(),
            "VANEHUB_APP_DATA_DIR must be an absolute path"
        );
    }
}
