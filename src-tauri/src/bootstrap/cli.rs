use crate::contexts::tooling::cli::api::{CliApi, CliEnvironmentApi, CliEnvironmentError};

/// Launch resolution for the Agent Runtime, over the source-aware environment.
///
/// One assembly, one authority. The flat `cli_tool_status` service this replaces had its own
/// repository, its own detection adapter, and its own executable locator, which is how the runtime
/// and the CLI Management page came to disagree about which installation a tool has.
pub(crate) fn assemble_cli_api(environment: CliEnvironmentApi) -> CliApi {
    CliApi::new(environment)
}

/// Scans the environment once on first run, in the background.
///
/// Deliberately unconditional beyond the emptiness check the service makes for itself: a host that
/// has never been scanned has no snapshot for the launch path to read, so the first launch would
/// fall back to a bounded live lookup instead of the environment's own answer.
pub(crate) fn start_initial_cli_refresh(
    environment: CliEnvironmentApi,
) -> Result<(), CliEnvironmentError> {
    let prepared = environment.prepare_refresh(Vec::new(), false)?;
    tauri::async_runtime::spawn_blocking(move || {
        let _ = environment.execute_refresh(prepared);
    });
    Ok(())
}
