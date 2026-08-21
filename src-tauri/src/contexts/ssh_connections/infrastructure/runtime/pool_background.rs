use crate::contexts::ssh_connections::application::connection_pool::RemoteSshBackgroundPort;
use std::future::Future;
use std::pin::Pin;

/// Detaches transport closes onto the application runtime.
///
/// `tauri::async_runtime::spawn` rather than `tokio::spawn`: the pool is reached from synchronous
/// `#[tauri::command]` bodies, which Tauri runs inline on the IPC thread, and that thread has no
/// ambient Tokio runtime. A bare `tokio::spawn` there panics, and the panic unwinds into an
/// `extern "system"` WebView2 callback that cannot unwind — which aborted the process whenever an
/// SSH connection was deleted or edited. This helper works from any thread.
pub(crate) struct TauriRemoteSshBackground;

impl RemoteSshBackgroundPort for TauriRemoteSshBackground {
    fn detach(&self, task: Pin<Box<dyn Future<Output = ()> + Send + 'static>>) {
        tauri::async_runtime::spawn(task);
    }
}
