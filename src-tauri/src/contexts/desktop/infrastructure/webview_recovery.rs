use crate::contexts::operations::application::{DiagnosticLog, DiagnosticLogPort, LogSeverity};
use crate::contexts::operations::infrastructure::UnifiedLoggingAdapter;
use std::collections::BTreeMap;
use std::time::Duration;

const UNRESPONSIVE_REPEAT_WINDOW: Duration = Duration::from_secs(45);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum WebviewFailureKind {
    BrowserProcessExited,
    RenderProcessExited,
    RenderProcessUnresponsive,
    FrameRenderProcessExited,
    UtilityProcessExited,
    SandboxHelperProcessExited,
    GpuProcessExited,
    PluginProcessExited,
    UnknownProcessExited,
}

impl WebviewFailureKind {
    fn label(self) -> &'static str {
        match self {
            Self::BrowserProcessExited => "browser-process-exited",
            Self::RenderProcessExited => "render-process-exited",
            Self::RenderProcessUnresponsive => "render-process-unresponsive",
            Self::FrameRenderProcessExited => "frame-render-process-exited",
            Self::UtilityProcessExited => "utility-process-exited",
            Self::SandboxHelperProcessExited => "sandbox-helper-process-exited",
            Self::GpuProcessExited => "gpu-process-exited",
            Self::PluginProcessExited => "plugin-process-exited",
            Self::UnknownProcessExited => "unknown-process-exited",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum WebviewRecoveryAction {
    Observe,
    Reload,
    Restart,
}

impl WebviewRecoveryAction {
    fn label(self) -> &'static str {
        match self {
            Self::Observe => "observe",
            Self::Reload => "reload",
            Self::Restart => "restart",
        }
    }
}

#[derive(Default)]
struct WebviewRecoveryPolicy {
    last_unresponsive: Option<Duration>,
}

impl WebviewRecoveryPolicy {
    fn decide(&mut self, failure: WebviewFailureKind, elapsed: Duration) -> WebviewRecoveryAction {
        match failure {
            WebviewFailureKind::BrowserProcessExited => {
                self.last_unresponsive = None;
                WebviewRecoveryAction::Restart
            }
            WebviewFailureKind::RenderProcessExited => {
                self.last_unresponsive = None;
                WebviewRecoveryAction::Reload
            }
            WebviewFailureKind::RenderProcessUnresponsive => {
                if self.last_unresponsive.is_some_and(|previous| {
                    elapsed.saturating_sub(previous) <= UNRESPONSIVE_REPEAT_WINDOW
                }) {
                    self.last_unresponsive = None;
                    WebviewRecoveryAction::Reload
                } else {
                    self.last_unresponsive = Some(elapsed);
                    WebviewRecoveryAction::Observe
                }
            }
            _ => {
                self.last_unresponsive = None;
                WebviewRecoveryAction::Observe
            }
        }
    }
}

fn record_process_failure(
    logging: &dyn DiagnosticLogPort,
    failure: WebviewFailureKind,
    action: WebviewRecoveryAction,
) {
    let mut context = BTreeMap::new();
    context.insert("source".to_string(), "native".to_string());
    context.insert("failureKind".to_string(), failure.label().to_string());
    context.insert("recoveryAction".to_string(), action.label().to_string());
    let severity = match action {
        WebviewRecoveryAction::Observe => LogSeverity::Warn,
        WebviewRecoveryAction::Reload | WebviewRecoveryAction::Restart => LogSeverity::Error,
    };
    let _ = logging.write_diagnostic(DiagnosticLog {
        severity,
        category: "desktop.webview.process-failed".to_string(),
        message: "The desktop WebView reported a process failure".to_string(),
        context,
    });
}

fn record_recovery_failure(logging: &dyn DiagnosticLogPort, operation: &str, error: &str) {
    let mut context = BTreeMap::new();
    context.insert("source".to_string(), "native".to_string());
    context.insert("operation".to_string(), operation.to_string());
    context.insert("error".to_string(), error.to_string());
    context.insert("fallbackAction".to_string(), "restart".to_string());
    let _ = logging.write_diagnostic(DiagnosticLog {
        severity: LogSeverity::Error,
        category: "desktop.webview.recovery-failed".to_string(),
        message: "The desktop WebView recovery request failed".to_string(),
        context,
    });
}

#[cfg(windows)]
pub(crate) fn install_main_webview_recovery(
    app: &tauri::AppHandle,
    fallback_log_directory: std::path::PathBuf,
) -> Result<(), String> {
    use std::cell::RefCell;
    use std::rc::Rc;
    use std::time::Instant;
    use tauri::Manager;
    use webview2_com::Microsoft::Web::WebView2::Win32::{
        COREWEBVIEW2_PROCESS_FAILED_KIND, COREWEBVIEW2_PROCESS_FAILED_KIND_BROWSER_PROCESS_EXITED,
        COREWEBVIEW2_PROCESS_FAILED_KIND_FRAME_RENDER_PROCESS_EXITED,
        COREWEBVIEW2_PROCESS_FAILED_KIND_GPU_PROCESS_EXITED,
        COREWEBVIEW2_PROCESS_FAILED_KIND_PPAPI_BROKER_PROCESS_EXITED,
        COREWEBVIEW2_PROCESS_FAILED_KIND_PPAPI_PLUGIN_PROCESS_EXITED,
        COREWEBVIEW2_PROCESS_FAILED_KIND_RENDER_PROCESS_EXITED,
        COREWEBVIEW2_PROCESS_FAILED_KIND_RENDER_PROCESS_UNRESPONSIVE,
        COREWEBVIEW2_PROCESS_FAILED_KIND_SANDBOX_HELPER_PROCESS_EXITED,
        COREWEBVIEW2_PROCESS_FAILED_KIND_UNKNOWN_PROCESS_EXITED,
        COREWEBVIEW2_PROCESS_FAILED_KIND_UTILITY_PROCESS_EXITED,
    };
    use webview2_com::ProcessFailedEventHandler;

    fn normalize(kind: COREWEBVIEW2_PROCESS_FAILED_KIND) -> WebviewFailureKind {
        match kind {
            COREWEBVIEW2_PROCESS_FAILED_KIND_BROWSER_PROCESS_EXITED => {
                WebviewFailureKind::BrowserProcessExited
            }
            COREWEBVIEW2_PROCESS_FAILED_KIND_RENDER_PROCESS_EXITED => {
                WebviewFailureKind::RenderProcessExited
            }
            COREWEBVIEW2_PROCESS_FAILED_KIND_RENDER_PROCESS_UNRESPONSIVE => {
                WebviewFailureKind::RenderProcessUnresponsive
            }
            COREWEBVIEW2_PROCESS_FAILED_KIND_FRAME_RENDER_PROCESS_EXITED => {
                WebviewFailureKind::FrameRenderProcessExited
            }
            COREWEBVIEW2_PROCESS_FAILED_KIND_UTILITY_PROCESS_EXITED => {
                WebviewFailureKind::UtilityProcessExited
            }
            COREWEBVIEW2_PROCESS_FAILED_KIND_SANDBOX_HELPER_PROCESS_EXITED => {
                WebviewFailureKind::SandboxHelperProcessExited
            }
            COREWEBVIEW2_PROCESS_FAILED_KIND_GPU_PROCESS_EXITED => {
                WebviewFailureKind::GpuProcessExited
            }
            COREWEBVIEW2_PROCESS_FAILED_KIND_PPAPI_PLUGIN_PROCESS_EXITED
            | COREWEBVIEW2_PROCESS_FAILED_KIND_PPAPI_BROKER_PROCESS_EXITED => {
                WebviewFailureKind::PluginProcessExited
            }
            COREWEBVIEW2_PROCESS_FAILED_KIND_UNKNOWN_PROCESS_EXITED => {
                WebviewFailureKind::UnknownProcessExited
            }
            _ => WebviewFailureKind::UnknownProcessExited,
        }
    }

    let main_webview = app
        .get_webview_window("main")
        .ok_or_else(|| "main WebView is unavailable during recovery setup".to_string())?;
    let app_handle = app.clone();
    main_webview
        .with_webview(move |platform_webview| {
            let logging = UnifiedLoggingAdapter::active(fallback_log_directory);
            let policy = Rc::new(RefCell::new(WebviewRecoveryPolicy::default()));
            let started_at = Instant::now();
            let controller = platform_webview.controller();
            let core_webview = match unsafe { controller.CoreWebView2() } {
                Ok(webview) => webview,
                Err(error) => {
                    record_recovery_failure(&logging, "get-core-webview", &error.to_string());
                    app_handle.restart();
                }
            };
            let callback_app = app_handle.clone();
            let callback_logging = logging.clone();
            let handler = ProcessFailedEventHandler::create(Box::new(move |sender, args| {
                let Some(args) = args else {
                    record_recovery_failure(
                        &callback_logging,
                        "read-process-failure",
                        "WebView2 omitted process failure arguments",
                    );
                    return Ok(());
                };
                let mut native_kind = COREWEBVIEW2_PROCESS_FAILED_KIND::default();
                if let Err(error) = unsafe { args.ProcessFailedKind(&mut native_kind) } {
                    record_recovery_failure(
                        &callback_logging,
                        "read-process-failure-kind",
                        &error.to_string(),
                    );
                    return Ok(());
                }
                let failure = normalize(native_kind);
                let action = policy.borrow_mut().decide(failure, started_at.elapsed());
                record_process_failure(&callback_logging, failure, action);
                match action {
                    WebviewRecoveryAction::Observe => {}
                    WebviewRecoveryAction::Restart => callback_app.restart(),
                    WebviewRecoveryAction::Reload => {
                        let reload = sender
                            .ok_or_else(|| "WebView2 omitted the failed WebView sender".to_string())
                            .and_then(|webview| {
                                unsafe { webview.Reload() }.map_err(|e| e.to_string())
                            });
                        if let Err(error) = reload {
                            record_recovery_failure(&callback_logging, "reload", &error);
                            callback_app.restart();
                        }
                    }
                }
                Ok(())
            }));
            let mut token = 0;
            if let Err(error) = unsafe { core_webview.add_ProcessFailed(&handler, &mut token) } {
                record_recovery_failure(&logging, "register-process-failed", &error.to_string());
                app_handle.restart();
            }
        })
        .map_err(|error| error.to_string())
}

#[cfg(not(windows))]
pub(crate) fn install_main_webview_recovery(
    _app: &tauri::AppHandle,
    _fallback_log_directory: std::path::PathBuf,
) -> Result<(), String> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fatal_main_surface_failures_select_recovery() {
        let mut policy = WebviewRecoveryPolicy::default();

        assert_eq!(
            policy.decide(WebviewFailureKind::RenderProcessExited, Duration::ZERO),
            WebviewRecoveryAction::Reload
        );
        assert_eq!(
            policy.decide(
                WebviewFailureKind::BrowserProcessExited,
                Duration::from_secs(1)
            ),
            WebviewRecoveryAction::Restart
        );
    }

    #[test]
    fn auto_recoverable_failures_are_observed_without_disruption() {
        for failure in [
            WebviewFailureKind::FrameRenderProcessExited,
            WebviewFailureKind::UtilityProcessExited,
            WebviewFailureKind::SandboxHelperProcessExited,
            WebviewFailureKind::GpuProcessExited,
            WebviewFailureKind::PluginProcessExited,
            WebviewFailureKind::UnknownProcessExited,
        ] {
            let mut policy = WebviewRecoveryPolicy::default();
            assert_eq!(
                policy.decide(failure, Duration::ZERO),
                WebviewRecoveryAction::Observe
            );
        }
    }

    #[test]
    fn repeated_unresponsiveness_within_the_window_reloads_once() {
        let mut policy = WebviewRecoveryPolicy::default();

        assert_eq!(
            policy.decide(
                WebviewFailureKind::RenderProcessUnresponsive,
                Duration::from_secs(10)
            ),
            WebviewRecoveryAction::Observe
        );
        assert_eq!(
            policy.decide(
                WebviewFailureKind::RenderProcessUnresponsive,
                Duration::from_secs(40)
            ),
            WebviewRecoveryAction::Reload
        );
        assert_eq!(
            policy.decide(
                WebviewFailureKind::RenderProcessUnresponsive,
                Duration::from_secs(41)
            ),
            WebviewRecoveryAction::Observe
        );
    }

    #[test]
    fn expired_or_interrupted_unresponsiveness_starts_a_new_window() {
        let mut policy = WebviewRecoveryPolicy::default();
        assert_eq!(
            policy.decide(
                WebviewFailureKind::RenderProcessUnresponsive,
                Duration::from_secs(5)
            ),
            WebviewRecoveryAction::Observe
        );
        assert_eq!(
            policy.decide(
                WebviewFailureKind::RenderProcessUnresponsive,
                Duration::from_secs(51)
            ),
            WebviewRecoveryAction::Observe
        );
        assert_eq!(
            policy.decide(
                WebviewFailureKind::GpuProcessExited,
                Duration::from_secs(52)
            ),
            WebviewRecoveryAction::Observe
        );
        assert_eq!(
            policy.decide(
                WebviewFailureKind::RenderProcessUnresponsive,
                Duration::from_secs(53)
            ),
            WebviewRecoveryAction::Observe
        );
    }
}
