use crate::contexts::operations::application::{DiagnosticLog, DiagnosticLogPort, LogSeverity};
use crate::contexts::operations::infrastructure::UnifiedLoggingAdapter;
use crate::contexts::tooling::cli::infrastructure::NativeConfigReader;
use crate::platform::database::NativeDatabase;
use crate::platform::logging;
use std::collections::BTreeMap;
use std::error::Error;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tauri::webview::{PageLoadEvent, PageLoadPayload};
use tauri::Manager;

const AGENT_TERMINAL_IDLE_TIMEOUT_SECONDS: i64 = 2 * 60 * 60;

/// 桌面端Tauri应用启动入口（当前包内可见）
pub(crate) fn run() {
    // 1. 构建Tauri应用实例，配置各类插件、生命周期、事件、命令
    let builder = tauri::Builder::default()
        .register_uri_scheme_protocol("vanehub-capture", |context, request| {
            crate::bootstrap::screenshot_capture::protocol_response(
                context.app_handle(),
                context.webview_label(),
                request.uri().path(),
            )
        })
        // 注册弹窗对话框插件（文件选择、提示框、确认框等）
        .plugin(tauri_plugin_dialog::init())
        // External provider/help links are opened by the operating system browser.
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        // 注册开机自启插件：Mac平台使用LaunchAgent实现开机启动，无额外配置
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ));
    #[cfg(feature = "desktop-e2e")]
    let builder = builder
        .plugin(tauri_plugin_wdio::init())
        .plugin(tauri_plugin_wdio_webdriver::init());
    let result = builder
        // Keep the native window hidden until the static startup document is ready to paint.
        .on_page_load(show_main_window_after_page_load)
        // 应用初始化完成后的setup回调函数
        .setup(setup)
        // 主窗口事件统一处理器（窗口打开/关闭/缩放/焦点等事件）
        .on_window_event(crate::contexts::desktop::infrastructure::handle_main_window_event)
        // 注册前端调用后端的命令路由处理器
        .invoke_handler(crate::commands::invoke_handler())
        // 编译时读取tauri.conf.json配置，构建应用对象
        .build(tauri::generate_context!());

    // 2. 匹配应用构建结果：构建成功则启动运行，失败则打印启动错误日志
    match result {
        // 应用构建成功，启动事件循环监听运行事件
        Ok(app) => app.run(|app, event| {
            #[cfg(feature = "desktop-e2e")]
            if matches!(event, tauri::RunEvent::Exit) {
                let _ = write_desktop_e2e_process_marker("exited");
            }
            // Stop the microphone, stop playback, and shut the engine workers down. A recording
            // left running past exit keeps the OS capture indicator lit, and an orphaned Python
            // process keeps a model resident in memory.
            if matches!(event, tauri::RunEvent::Exit) {
                if let Some(lifecycle) = app.try_state::<
                    crate::contexts::skill_evolution_orchestration::infrastructure::EvolutionBackgroundLifecycle,
                >() {
                    let _ = lifecycle.shutdown();
                }
                if let Some(capture) = app.try_state::<
                    crate::bootstrap::screenshot_capture::ScreenshotCaptureState,
                >() {
                    capture.shutdown(app);
                }
                if let Some(local_media) =
                    app.try_state::<crate::contexts::local_media::api::LocalMediaApi>()
                {
                    local_media.shutdown();
                }
            }
            // 判断事件为程序退出事件，且存在遥测生命周期管理实例
            // The evidence bridge drains on a bounded deadline. Evidence describes work that has
            // already happened, so losing its tail is strictly better than refusing to close.
            if matches!(event, tauri::RunEvent::Exit) {
                if let Some(bridge) = app.try_state::<super::EvidenceBridgeShutdown>() {
                    bridge.shutdown();
                }
                // Uninstalled before the wait, and that order is the whole point: the sink holds
                // the only sender, so a shutdown that waited without releasing it waited for a
                // channel that could never close. Dropped outside the slot's lock, because the
                // destructor would otherwise run under a lock every logging call takes.
                let sink = crate::platform::log_receipts::take_append_sink();
                drop(sink);
                // Drains the receipts already queued. Bounded by the queue's own capacity now that
                // the sender is gone and the worker sees the channel close.
                if let Some(worker) =
                    app.try_state::<std::sync::Arc<super::LogIndexBridgeWorker>>()
                {
                    worker.shutdown();
                }
            }
            // Retained Shells outlive their views by design, so nothing else closes them. Joining
            // each runtime's workers here is the difference between a clean exit and a window that
            // has closed while the process waits on a thread reading a dead PTY.
            if matches!(event, tauri::RunEvent::Exit) {
                if let Some(workspaces) =
                    app.try_state::<crate::contexts::workspaces::api::WorkspaceApi>()
                {
                    let report = workspaces.shutdown_session_shells();
                    // Residual cleanup is recorded rather than waited on. Blocking here until every
                    // child died would make an unkillable process into an application that cannot
                    // be closed; logging nothing would make it into a leak nobody can see.
                    if !report.is_complete() {
                        write_bootstrap_log(
                            &logging::fallback_log_dir(),
                            LogSeverity::Warn,
                            "workspaces.shell_shutdown",
                            &format!(
                                "{} of {} retained shells were still unconfirmed at the shutdown deadline",
                                report.reaping() + report.failed(),
                                report.requested()
                            ),
                        );
                    }
                }
            }
            if matches!(event, tauri::RunEvent::Exit)
                && app
                .try_state::<crate::contexts::execution_observability::infrastructure::ExecutionTelemetryLifecycle>()
                .is_some_and(|lifecycle| lifecycle.shutdown().is_err())
            {
                // 遥测数据关闭流程执行失败，写入警告启动日志
                write_bootstrap_log(
                    &logging::fallback_log_dir(), // 兜底日志存储目录
                    LogSeverity::Warn,             // 日志级别：警告
                    "execution_observability.shutdown", // 日志分类标识
                    "Execution telemetry did not flush completely before the bounded shutdown deadline",
                );
            }
        }),
        // Tauri应用构建失败，记录错误日志
        Err(error) => write_bootstrap_log(
            &logging::fallback_log_dir(),
            LogSeverity::Error,
            "runtime.failure",
            &error.to_string(),
        ),
    }
}

fn show_main_window_after_page_load(webview: &tauri::Webview, payload: &PageLoadPayload<'_>) {
    if webview.label() != "main" || payload.event() != PageLoadEvent::Finished {
        return;
    }
    if let Err(error) = webview.window().show() {
        write_bootstrap_log(
            &logging::fallback_log_dir(),
            LogSeverity::Error,
            "runtime.main-window.show",
            &error.to_string(),
        );
    }
}

/// Tauri应用初始化回调函数
/// 负责组装所有领域API、初始化数据库、注册状态管理、启动后台任务
/// # 参数
/// * `app` - Tauri应用实例可变引用
/// # 返回
/// 初始化成功返回Ok(())，失败返回包装后的错误
fn setup(app: &mut tauri::App) -> Result<(), Box<dyn Error>> {
    let data_dir = match configured_app_data_dir(std::env::var_os("VANEHUB_APP_DATA_DIR"))? {
        Some(path) => path,
        None => app.path().app_data_dir().map_err(boxed_error)?,
    };
    std::env::set_var("VANEHUB_APP_DATA_DIR", &data_dir);
    let fallback_log_directory = logging::fallback_log_dir();
    let database = NativeDatabase::new(data_dir).map_err(boxed_error)?;
    #[cfg(feature = "desktop-e2e")]
    write_desktop_e2e_process_marker("running").map_err(boxed_error)?;
    let evidence_logging: Arc<dyn DiagnosticLogPort> = Arc::new(UnifiedLoggingAdapter::active(
        fallback_log_directory.clone(),
    ));
    let skill_evolution_evidence_api =
        crate::contexts::skill_evolution_evidence::api::SkillEvolutionEvidenceApi::new(
            database.clone(),
            evidence_logging.clone(),
        );
    let skill_evolution_orchestration_api =
        crate::contexts::skill_evolution_orchestration::api::SkillEvolutionOrchestrationApi::new(
            database.clone(),
            evidence_logging.clone(),
        );
    let evolution_background = skill_evolution_orchestration_api.background_lifecycle();
    let skill_evolution_assessment_api =
        crate::contexts::skill_evolution_assessment::api::SkillEvolutionAssessmentApi::new(
            database.clone(),
        )
        .map_err(|error| boxed_message(error.code()))?;
    let skill_evolution_generation_api =
        crate::contexts::skill_evolution_generation::api::SkillEvolutionGenerationApi::new(
            database.clone(),
        );
    // Assembled before the producers so each one can be handed the sender it publishes through.
    // The worker owns the only handle that calls the recorder; producers reach it through their own
    // port and never learn that a journal is on the other side.
    let execution_evidence_api = super::assemble_execution_evidence_api(
        database.clone(),
        app.handle().clone(),
        evidence_logging,
    );
    let (evidence_bridge, evidence_bridge_worker) =
        super::start_evidence_bridge(execution_evidence_api.clone());
    // Installed before anything else logs, so the index sees the startup records too. The sink is
    // process-wide because `write_entry` is reached from every layer, including the ones that log
    // while this function is still assembling.
    let (log_index_bridge, log_index_worker) = super::start_log_index_bridge(
        std::sync::Arc::new(
            crate::contexts::operations::infrastructure::SqliteLogIndexRepository::new(
                database.clone(),
            ),
        ),
        std::sync::Arc::new(
            crate::contexts::operations::infrastructure::TauriLogNoticePublisher::new(
                app.handle().clone(),
            ),
        ),
    );
    crate::platform::log_receipts::set_append_sink(Box::new(log_index_bridge));
    let session_log_api = super::assemble_session_log_api(
        database.clone(),
        app.handle().clone(),
        fallback_log_directory.clone(),
    );
    // Brings the index up to date with whatever the files already hold, off the startup path.
    // Queries answer with `indexing` coverage until it finishes, which is the honest report: the
    // rows are real and the set is not yet final.
    super::start_log_index_repair_job(session_log_api.clone());
    crate::contexts::desktop::infrastructure::install_main_webview_recovery(
        app.handle(),
        fallback_log_directory.clone(),
    )
    .map_err(boxed_message)?;
    let (desktop_settings_api, desktop_locale_bridge) =
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

    let operations_api = super::assemble_operations_api(database.clone());
    let local_media_api = super::assemble_local_media_api(
        database.clone(),
        operations_api.clone(),
        Arc::new(UnifiedLoggingAdapter::active(
            fallback_log_directory.clone(),
        )),
        database
            .db_path
            .parent()
            .unwrap_or_else(|| std::path::Path::new(".")),
        &super::worker_bridge_candidates(app.path().resource_dir().ok()),
    );
    // Ephemeral media from a previous run -- a recording interrupted by a crash, a staged file the
    // user never used -- is swept once here rather than accumulating until the disk notices.
    local_media_api.sweep_stale_media();
    let code_intelligence_api = super::assemble_code_intelligence_api(
        database.clone(),
        fallback_log_directory.clone(),
        // Beside the database rather than beside the logs: a language server's per-workspace index
        // is state, and it belongs with the other state this profile owns.
        database
            .db_path
            .parent()
            .unwrap_or_else(|| std::path::Path::new("."))
            .to_path_buf(),
    );
    code_intelligence_api.start_maintenance();
    let code_intelligence_responder = Arc::new(super::NativeCodeIntelligenceResponder::new(
        code_intelligence_api.clone(),
    ));
    let workspace_mutations = Arc::new(super::WorkspaceMutationFanout::new(
        code_intelligence_api.clone(),
        Arc::new(evidence_bridge.clone()),
    ));
    let cli_config_api =
        super::assemble_cli_config_api(database.clone(), fallback_log_directory.clone())
            .map_err(boxed_message)?;
    cli_config_api.synchronize_startup();
    let mcp_api = super::assemble_mcp_api(
        database.clone(),
        operations_api.clone(),
        fallback_log_directory.clone(),
    );
    let cli_environment_api = super::assemble_cli_environment_api(
        database.clone(),
        operations_api.clone(),
        fallback_log_directory.clone(),
    );
    // Launch resolution reads the same environment the CLI Management page does, so what the
    // runtime starts and what the page reports cannot be two different installations.
    let cli_api = super::assemble_cli_api(cli_environment_api.clone());
    // Assembled after `cli_api` because compatibility is read from the CLI lifecycle subdomain's
    // cached detection state rather than from a second detector. Both facades share one service.
    let (cli_parameter_runtime_api, cli_parameter_settings_api) =
        super::assemble_cli_parameter_apis(
            database.clone(),
            cli_api.clone(),
            fallback_log_directory.clone(),
        );
    let sdk_api = super::assemble_sdk_api(
        database.clone(),
        operations_api.clone(),
        fallback_log_directory.clone(),
    );
    let shared_agent_registry = super::assemble_shared_agent_registry(
        database.clone(),
        sdk_api.clone(),
        cli_api.clone(),
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
    let skill_tool_api = super::assemble_skill_tool_api(
        database.clone(),
        skill_api.clone(),
        fallback_log_directory.clone(),
    );
    skill_api.bind_runtime_observer(Arc::new(skill_tool_api.clone()));
    let prompt_hook_api =
        super::assemble_prompt_hook_api(database.clone(), fallback_log_directory.clone());
    let ssh_connections_api =
        super::assemble_ssh_connections_api(database.clone(), fallback_log_directory.clone());
    let workspace_api = super::assemble_workspace_api(
        database.clone(),
        app.handle().clone(),
        fallback_log_directory.clone(),
        Arc::new(evidence_bridge.clone()),
        ssh_connections_api.clone(),
    );
    let native_config_reader = Arc::new(NativeConfigReader::new(Arc::new(
        UnifiedLoggingAdapter::active(fallback_log_directory.clone()),
    )));
    let (sessions_api, session_runtime_adapter, session_recovery) = super::assemble_sessions_api(
        database.clone(),
        super::SessionRuntimeDependencies {
            app: app.handle().clone(),
            operations: operations_api.clone(),
            workspaces: workspace_api.clone(),
        },
        cli_parameter_runtime_api.clone(),
        native_config_reader,
        shared_agent_registry.registry.clone(),
        fallback_log_directory.clone(),
        Arc::new(evidence_bridge.clone()),
    );
    let permissions_assembly = super::assemble_permissions_api(
        database.clone(),
        desktop_settings_api.clone(),
        app.handle().clone(),
    );
    let permissions_api = permissions_assembly.api.clone();
    let runners = super::assemble_agent_runners(sessions_api.clone(), ssh_connections_api.clone())
        .map_err(boxed_message)?;
    let runner_discovery = Arc::new(
        crate::contexts::agent_runtime::infrastructure::NativeRunnerDiscovery::new(
            sessions_api.clone(),
            ssh_connections_api.clone(),
        ),
    );
    let runner_recovery = Arc::new(
        crate::contexts::agent_runtime::infrastructure::RunnerRunRecoveryAdapter::new(
            runners.clone(),
        ),
    );
    let agent_runs_api = super::assemble_agent_runs_api_with_recovery(
        database.clone(),
        runner_recovery,
        Arc::new(evidence_bridge.clone()),
    );
    agent_runs_api
        .reconcile_after_restart()
        .map_err(boxed_error)?;
    // `assemble_retrieval` below needs `agent_runtime_api` (the output of this very call), so a
    // real `RetrievalApi` cannot exist yet when `RuntimeAgentApiAdapter`'s `recall` tool is wired
    // up — this cell starts empty and is bound once the real one is ready, a few lines down.
    let deferred_retrieval = Arc::new(super::DeferredAgentRetrieval::default());
    // Personalization is assembled before the agent runtime because the runtime's memory port is a
    // projection of it. Its own retrieval coordination is deferred for the mirror-image reason:
    // retrieval is assembled after the runtime, so the real handle cannot exist yet.
    let deferred_memory_index = Arc::new(super::DeferredRetrievalIndex::default());
    let data_root = database
        .db_path
        .parent()
        .ok_or_else(|| boxed_message("Application data directory is unavailable.".to_string()))?
        .to_path_buf();
    let super::PersonalizationAssembly {
        api: personalization_api,
        maintenance: personalization_maintenance,
        resolver: _personalization_resolver,
        preview: _personalization_preview,
    } = super::assemble_personalization(
        database.clone(),
        &data_root,
        desktop_settings_api.clone(),
        shared_agent_registry.registry.clone(),
        deferred_memory_index.clone(),
        Arc::new(crate::contexts::personalization::infrastructure::SystemPersonalizationClock),
    )
    .map_err(boxed_message)?;
    let super::AgentRuntimeAssembly {
        api: agent_runtime_api,
        telemetry_lifecycle,
        completion_events,
    } = super::assemble_agent_runtime_api(super::AgentRuntimeDependencies {
        local_media: local_media_api.clone(),
        database: database.clone(),
        app: app.handle().clone(),
        operations: operations_api.clone(),
        agent_runs: agent_runs_api.clone(),
        cli: cli_api.clone(),
        cli_parameter_runtime: cli_parameter_runtime_api.clone(),
        prompts: prompt_hook_api.clone(),
        skills: skill_api.clone(),
        skill_tools: skill_tool_api.clone(),
        mcp: mcp_api.clone(),
        sessions: sessions_api.clone(),
        runners,
        runner_discovery,
        workspaces: workspace_api.clone(),
        permissions: permissions_api.clone(),
        shared_registry: shared_agent_registry,
        retrieval: deferred_retrieval.clone(),
        code_intelligence: code_intelligence_responder,
        workspace_mutations: workspace_mutations.clone(),
        desktop_settings: desktop_settings_api.clone(),
        personalization: personalization_api.clone(),
        evidence: skill_evolution_evidence_api.projector(),
        execution_evidence: Arc::new(evidence_bridge),
    })
    .map_err(boxed_message)?;
    let skill_evolution_curation_api =
        crate::contexts::skill_evolution_curation::api::SkillEvolutionCurationApi::new(
            database.clone(),
            skill_api.clone(),
            agent_runtime_api.clone(),
            Arc::new(
                crate::contexts::skill_evolution_curation::infrastructure::TauriCuratorNotificationEventAdapter::new(
                    app.handle().clone(),
                ),
            ),
            Arc::new(UnifiedLoggingAdapter::active(fallback_log_directory.clone())),
        );
    // Assembled here rather than with the rest of `permissions` because it needs `agent_runtime`,
    // which only exists at this point. The timeout sweep and the frontend command share this one
    // instance, which is what makes them two callers of one single-winner decision.
    let approval_resolver = std::sync::Arc::new(super::assemble_approval_resolver(
        &permissions_assembly,
        agent_runtime_api.clone(),
    ));
    super::start_permission_timeout_sweep_job(permissions_api.clone(), approval_resolver.clone());
    let execution_observability_api = super::assemble_execution_observability_api(database.clone());
    let evaluation_api = super::assemble_evaluation_api(
        database.clone(),
        operations_api.clone(),
        agent_runs_api.clone(),
        agent_runtime_api.clone(),
        sessions_api.clone(),
        workspace_api.clone(),
        fallback_log_directory.join("evaluation-runs"),
    );
    let super::RetrievalAssembly {
        api: retrieval_api,
        code_index_api,
        worker: retrieval_worker,
        code_retrieval,
    } = super::assemble_retrieval(
        database.clone(),
        agent_runtime_api.clone(),
        sessions_api.clone(),
        workspace_api.clone(),
        personalization_api.clone(),
    );
    deferred_retrieval.bind(retrieval_api.clone());
    // Bound before maintenance is spawned, so the derived rebuild's retrieval reconciliation reaches
    // a real worker rather than the no-op stand-in.
    deferred_memory_index.bind(retrieval_api.clone());
    super::spawn_startup_maintenance(
        personalization_maintenance,
        Arc::new(UnifiedLoggingAdapter::active(
            fallback_log_directory.clone(),
        )),
    );
    deferred_retrieval.bind_code(code_retrieval);
    workspace_mutations
        .bind_code_index(code_index_api.clone())
        .map_err(boxed_message)?;
    workspace_mutations
        .bind_workspace_changes(workspace_api.change_observer())
        .map_err(boxed_message)?;
    session_runtime_adapter
        .attach_agent_runtime(agent_runtime_api.clone())
        .map_err(boxed_message)?;
    session_recovery
        .run_startup_with_retry(100)
        .map_err(boxed_message)?;
    let agent_run_controls_api =
        super::AgentRunControlsApi::new(agent_runs_api.clone(), agent_runtime_api.clone());
    agent_runtime_api
        .reconcile_loop_startup()
        .map_err(boxed_message)?;
    let scheduled_task_database = database.clone();
    let execution_retention_database = database.clone();
    let communications_maintenance_database = database.clone();
    app.manage(database.clone());
    app.manage(personalization_api.clone());
    app.manage(skill_evolution_evidence_api);
    app.manage(skill_evolution_orchestration_api);
    app.manage(evolution_background.clone());
    app.manage(skill_evolution_assessment_api);
    app.manage(skill_evolution_generation_api);
    app.manage(skill_evolution_curation_api);
    app.manage(
        crate::contexts::skill_evolution_system_activity::api::SkillEvolutionSystemActivityApi::new(
            database.clone(),
        ),
    );
    app.manage(super::ScheduledTaskLogDirectory::new(
        fallback_log_directory.clone(),
    ));

    // Cloned before the move into communications: the file-evidence read side needs its own
    // handle, and it is assembled after this point.
    let evidence_link_database = database.clone();
    let communications = super::assemble_communications(super::CommunicationsDependencies {
        app: app.handle().clone(),
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
    completion_events
        .attach_completion_hook(Arc::new(CommunicationsCompletionHook {
            api: communications_api.clone(),
        }))
        .map_err(boxed_message)?;

    app.manage(operations_api.clone());
    app.manage(local_media_api.clone());
    app.manage(crate::bootstrap::screenshot_capture::ScreenshotCaptureState::default());
    app.manage(agent_runs_api);
    app.manage(agent_run_controls_api);
    app.manage(code_intelligence_api.clone());
    app.manage(cli_api.clone());
    app.manage(cli_environment_api.clone());
    app.manage(cli_config_api);
    app.manage(cli_parameter_settings_api);
    app.manage(mcp_api);
    app.manage(sdk_api);
    app.manage(extension_api);
    app.manage(plugin_integration_api);
    app.manage(skill_api);
    app.manage(skill_tool_api);
    app.manage(prompt_hook_api);
    app.manage(ssh_connections_api);
    super::start_session_shell_idle_job(workspace_api.clone());
    // Assembled here rather than inside either context: workspaces knows where a file is and
    // evidence knows what happened to it, and neither should learn the other half.
    app.manage(super::SessionFileEvidence::new(
        workspace_api.clone(),
        super::assemble_file_evidence_links(evidence_link_database),
    ));
    app.manage(workspace_api.clone());
    app.manage(sessions_api.clone());
    app.manage(agent_runtime_api.clone());
    app.manage(permissions_api.clone());
    // Managed as its own state rather than reached through `PermissionsApi`, because the resolver
    // is the only thing here that legitimately spans two contexts and the facade should not start
    // carrying `agent_runtime`.
    app.manage(approval_resolver);
    app.manage(retrieval_api);
    app.manage(code_index_api);
    app.manage(telemetry_lifecycle);
    app.manage(execution_observability_api.clone());
    app.manage(super::EvidenceBridgeShutdown::new(evidence_bridge_worker));
    app.manage(log_index_worker);
    app.manage(session_log_api.clone());
    super::start_evidence_maintenance_job(
        execution_evidence_api.clone(),
        fallback_log_directory.clone(),
    );
    // Assembled here rather than inside any one context: the report reads from five of them,
    // and the layer allowed to know all five is this one.
    app.manage(super::assemble_session_run_report(
        execution_evidence_api.clone(),
        execution_observability_api.clone(),
        session_log_api.clone(),
        sessions_api.clone(),
        workspace_api.clone(),
        fallback_log_directory.clone(),
    ));
    app.manage(execution_evidence_api);
    app.manage(evaluation_api);
    #[cfg(feature = "desktop-e2e")]
    app.manage(crate::contexts::communications::infrastructure::FeishuDesktopFixture::default());
    app.manage(communications_api.clone());
    app.manage(wechat_authorization_api);
    app.manage(desktop_settings_api.clone());
    app.manage(floating_assistant_api.clone());
    let desktop_update_api = crate::contexts::desktop::DesktopUpdateApi::new(
        app.handle().clone(),
        desktop_settings_api.clone(),
        operations_api.clone(),
    );
    let automatic_update_check = desktop_update_api
        .preferences()
        .map(|preferences| preferences.automatic_check)
        .unwrap_or(false);
    app.manage(desktop_update_api);
    if automatic_update_check {
        let _ = app
            .state::<crate::contexts::desktop::DesktopUpdateApi>()
            .start_check();
    }

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
    super::start_retrieval_indexing_worker(retrieval_worker, fallback_log_directory.clone());
    start_agent_terminal_cleanup_job(agent_runtime_api.clone());
    let desktop_lifecycle_api =
        super::assemble_desktop_lifecycle_api(super::DesktopLifecycleDependencies {
            app: app.handle().clone(),
            language: &tray_language,
            agents: agent_runtime_api.clone(),
            communications: communications_api.clone(),
            code_intelligence: code_intelligence_api,
            evolution_background,
            locale_bridge: desktop_locale_bridge,
            fallback_log_directory: fallback_log_directory.clone(),
        })
        .map_err(boxed_message)?;
    app.manage(desktop_lifecycle_api.clone());
    super::initialize_desktop_runtime(
        &desktop_lifecycle_api,
        &floating_assistant_api,
        fallback_log_directory.clone(),
    );
    if let Err(error) = app
        .state::<crate::contexts::skill_evolution_orchestration::infrastructure::EvolutionBackgroundLifecycle>()
        .start(std::time::Duration::from_secs(15 * 60))
    {
        write_bootstrap_log(
            &fallback_log_directory,
            LogSeverity::Warn,
            "skill-evolution.orchestration.background",
            &error,
        );
    }
    super::start_initial_cli_refresh(cli_environment_api.clone()).map_err(boxed_error)?;
    start_communications_maintenance_job(
        communications_api.clone(),
        communications_maintenance_database,
    );
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

#[cfg(feature = "desktop-e2e")]
fn write_desktop_e2e_process_marker(state: &str) -> std::io::Result<()> {
    use std::io::{Error as IoError, ErrorKind};

    let data_dir = std::env::var_os("VANEHUB_APP_DATA_DIR")
        .map(PathBuf::from)
        .ok_or_else(|| IoError::new(ErrorKind::InvalidInput, "VANEHUB_APP_DATA_DIR is required"))?;
    let run_id = std::env::var("VANEHUB_TEST_RUN_ID")
        .map_err(|_| IoError::new(ErrorKind::InvalidInput, "VANEHUB_TEST_RUN_ID is required"))?;
    let marker = serde_json::json!({
        "pid": std::process::id(),
        "runId": run_id,
        "state": state,
        "updatedAt": chrono::Utc::now().to_rfc3339(),
    });
    std::fs::write(
        data_dir.join("desktop-e2e-process.json"),
        serde_json::to_vec_pretty(&marker).map_err(IoError::other)?,
    )
}

struct CommunicationsCompletionHook {
    api: crate::contexts::communications::api::CommunicationsApi,
}

impl crate::contexts::agent_runtime::infrastructure::AgentCompletionHook
    for CommunicationsCompletionHook
{
    fn completed(&self, session_id: &str, message_id: &str, originated_from_im: bool) {
        let api = self.api.clone();
        let session_id = session_id.to_string();
        let message_id = message_id.to_string();
        tauri::async_runtime::spawn(async move {
            let _ = api
                .notify_session_completion(&session_id, &message_id, originated_from_im)
                .await;
        });
    }
}

fn start_communications_maintenance_job(
    communications_api: crate::contexts::communications::api::CommunicationsApi,
    database: crate::platform::database::NativeDatabase,
) {
    tauri::async_runtime::spawn(async move {
        let repository =
            crate::contexts::communications::infrastructure::SqliteCommunicationsRepository::new(
                database,
            );
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(6 * 60 * 60));
        loop {
            interval.tick().await;
            let _ = communications_api.maintain_deduplication();
            let _ = crate::contexts::communications::infrastructure::maintain_wechat_reply_contexts(
                &repository,
            );
        }
    });
}

/// 启动Agent终端空闲清理后台任务
/// 每分钟执行一次检查，清理超过2小时空闲的终端会话
/// # 参数
/// * `agent_runtime_api` - Agent运行时API实例
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

/// 解析并验证应用数据目录配置
/// 支持通过环境变量VANEHUB_APP_DATA_DIR自定义数据目录，必须为绝对路径
/// # 参数
/// * `value` - 环境变量值（可能为空）
/// # 返回
/// 验证通过返回Some(有效路径)，空值返回None，相对路径返回错误
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

/// 写入启动阶段日志
/// 在正式日志系统初始化前使用，用于记录启动过程中的错误和警告
/// # 参数
/// * `fallback_log_directory` - 兜底日志目录
/// * `severity` - 日志级别
/// * `category` - 日志分类标识
/// * `message` - 日志消息内容
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

/// 将任意Error类型转换为`Box<dyn Error>`特征对象
/// 用于统一错误类型，方便?运算符传播
/// # 参数
/// * `error` - 实现了Error特征的错误类型
fn boxed_error(error: impl Error + 'static) -> Box<dyn Error> {
    Box::new(error)
}

/// 将字符串消息转换为标准IO错误并装箱
/// 用于快速创建错误信息
/// # 参数
/// * `message` - 可显示的错误消息
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
