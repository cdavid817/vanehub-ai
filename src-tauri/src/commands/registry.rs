//! Auditable Tauri command registry grouped by bounded context.

pub(crate) fn invoke_handler(
) -> impl Fn(tauri::ipc::Invoke<tauri::Wry>) -> bool + Send + Sync + 'static {
    let builtin_tools = super::builtin_tool_registry::invoke_handler();
    let supplemental = super::supplemental_registry::invoke_handler();
    let core = super::core_registry::invoke_handler();
    move |invoke| {
        let is_builtin_tool = super::builtin_tool_registry::is_command(invoke.message.command());
        if is_builtin_tool {
            builtin_tools(invoke)
        } else if super::supplemental_registry::is_command(invoke.message.command()) {
            supplemental(invoke)
        } else {
            core(invoke)
        }
    }
}
