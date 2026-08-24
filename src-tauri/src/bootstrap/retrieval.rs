//! 检索上下文的装配根与后台索引 worker。
//!
//! `retrieval` 与 `agent_runtime` 的**唯一**交汇点在本文件，且是双向的：`retrieval` 需要一个
//! embedding 端点和一份记忆快照（`AgentRuntimeEmbeddingEndpoint`/`GovernedMemoryIndexSource`），
//! `agent_runtime` 的 `recall` 工具反过来需要检索能力（`DeferredAgentRetrieval`，Task 13）。两个
//! 上下文因此都不 import 对方，跨界只发生在组合根里（设计文档 §4.3）。
//!
//! 应用服务刻意不打日志、只返回结构化结果，日志格式因此只在本文件定义一处（设计文档 §8.2）。
//! **绝不落盘**：记忆内容、query 原文、凭据、provider 响应体——下面每条日志的字段都只有计数、
//! 耗时、模型 id 与错误类别。

use crate::contexts::agent_runtime::api::AgentRuntimeApi;
use crate::contexts::agent_runtime::application::{
    AgentCodeRetrievalHit, AgentCodeRetrievalOutcome, AgentCodeRetrievalPort, AgentRetrievalHit,
    AgentRetrievalOutcome, AgentRetrievalPort,
};
use crate::contexts::operations::api::{DiagnosticLog, DiagnosticLogPort, LogSeverity};
use crate::contexts::operations::infrastructure::UnifiedLoggingAdapter;
use crate::contexts::personalization::api::PersonalizationApi;
use crate::contexts::retrieval::api::{
    CodeIndexApi, CodeIndexMutationBatch, CodeIndexMutationQueue, RetrievalApi,
    RetrievalWorkerSignal,
};
use crate::contexts::retrieval::application::{
    BatchOutcome, CodeRetrievalPort, EmbeddingEndpointPort, EmbeddingFailure, EmbeddingPort,
    IndexSourcePort, IndexSourceRecord, IndexingService, ResolvedEmbeddingEndpoint,
    RetrievalConfigurationRepository, RetrievalDocumentRepository, SearchOutcome, SearchService,
    RECONCILE_POLL_INTERVAL_SECONDS, RETRY_BACKOFF_SECONDS,
};
use crate::contexts::retrieval::domain::{
    CodeIndexAuditEvent, CodeIndexMode, CodeIndexPhase, FailureCategory, RetrievalError,
};
use crate::contexts::retrieval::infrastructure::{
    code_index_repository::SqliteCodeIndexRepository, HttpEmbeddingAdapter,
    SqliteRetrievalConfigurationRepository, SqliteRetrievalDocumentRepository,
    WorkspaceFileIndexSource,
};
use crate::contexts::sessions::api::SessionsApi;
use crate::contexts::workspaces::api::WorkspaceApi;
use crate::platform::database::NativeDatabase;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, RecvTimeoutError};
use std::sync::{Arc, Mutex, OnceLock};
use std::thread;
use std::time::{Duration, Instant};

const POLL_INTERVAL: Duration = Duration::from_secs(RECONCILE_POLL_INTERVAL_SECONDS);
const DEFAULT_INTER_BATCH_INTERVAL: Duration = Duration::from_millis(250);
const MAX_PROVIDER_RETRY_AFTER: Duration = Duration::from_secs(60);
const MAX_CODE_WORKSPACES_PER_CYCLE: usize = 8;
const RECONCILE_CATEGORY: &str = "retrieval.indexing.reconcile";
const BATCH_CATEGORY: &str = "retrieval.indexing.batch";
const CODE_RECONCILE_CATEGORY: &str = "retrieval.code_index.reconcile";
const CODE_BATCH_CATEGORY: &str = "retrieval.code_index.embedding";
const WORKER_CATEGORY: &str = "retrieval.indexing.worker";

pub(crate) struct RetrievalAssembly {
    pub(crate) api: RetrievalApi,
    pub(crate) code_index_api: CodeIndexApi,
    pub(crate) worker: RetrievalIndexingWorker,
    pub(crate) code_retrieval: Arc<dyn AgentCodeRetrievalPort>,
}

/// worker 需要的全部状态。与 `RetrievalApi` 分开返回，好让组合根按既有惯例先 `manage` 门面、
/// 再和其他 background job 一起启动线程。
pub(crate) struct RetrievalIndexingWorker {
    indexing: IndexingService,
    configuration: Arc<dyn RetrievalConfigurationRepository>,
    wakeups: Receiver<()>,
    inter_batch_interval: Duration,
    code: Option<WorkspaceCodeIndexWorker>,
}

struct WorkspaceCodeIndexWorker {
    database: NativeDatabase,
    code_index: Arc<dyn crate::contexts::retrieval::application::CodeIndexRepository>,
    documents: Arc<dyn RetrievalDocumentRepository>,
    embeddings: Arc<dyn EmbeddingPort>,
    active_workspace: ActiveSessionWorkspace,
    round_robin_cursor: Mutex<usize>,
    mutations: CodeIndexMutationQueue,
}

struct ActiveSessionWorkspace {
    sessions: SessionsApi,
    workspaces: WorkspaceApi,
}

impl ActiveSessionWorkspace {
    fn root(&self) -> Result<Option<PathBuf>, RetrievalError> {
        let Some(session) = self
            .sessions
            .active()
            .map_err(|_| RetrievalError::Unavailable)?
        else {
            return Ok(None);
        };
        self.workspaces
            .resolve_session_root(session.id())
            .map(|root| root.map(PathBuf::from))
            .map_err(|_| RetrievalError::Unavailable)
    }
}

pub(crate) fn assemble_retrieval(
    database: NativeDatabase,
    agent_runtime: AgentRuntimeApi,
    sessions: SessionsApi,
    workspaces: WorkspaceApi,
    personalization: PersonalizationApi,
) -> RetrievalAssembly {
    let documents: Arc<dyn RetrievalDocumentRepository> =
        Arc::new(SqliteRetrievalDocumentRepository::new(database.clone()));
    let code_index = Arc::new(SqliteCodeIndexRepository::new(database.clone()));
    let configuration: Arc<dyn RetrievalConfigurationRepository> = Arc::new(
        SqliteRetrievalConfigurationRepository::new(database.clone()),
    );
    // The governed store, not the pre-v2 directory. Once migration completes the v1 files are gone,
    // so a source still reading them would index an empty pool and quietly stop answering recalls.
    let source: Arc<dyn IndexSourcePort> = Arc::new(GovernedMemoryIndexSource { personalization });
    let endpoint: Arc<dyn EmbeddingEndpointPort> =
        Arc::new(AgentRuntimeEmbeddingEndpoint { agent_runtime });
    let embeddings: Arc<dyn EmbeddingPort> = Arc::new(ConfiguredProfileEmbeddingAdapter {
        configuration: configuration.clone(),
        endpoint,
    });
    let (signal, wakeups) = RetrievalWorkerSignal::channel();
    let code_mutations = CodeIndexMutationQueue::new();
    let api = RetrievalApi::new(
        Arc::new(SearchService::new(
            configuration.clone(),
            documents.clone(),
            source.clone(),
            embeddings.clone(),
        )),
        documents.clone(),
        configuration.clone(),
        signal,
    );
    RetrievalAssembly {
        code_index_api: CodeIndexApi::new_with_mutations(
            code_index.clone(),
            api.clone(),
            code_mutations.clone(),
        ),
        code_retrieval: Arc::new(NativeAgentCodeRetrieval {
            code_index: code_index.clone(),
            configuration: configuration.clone(),
            documents: documents.clone(),
            embeddings: embeddings.clone(),
        }),
        api,
        worker: RetrievalIndexingWorker {
            indexing: IndexingService::new(documents.clone(), source, embeddings.clone()),
            configuration,
            wakeups,
            inter_batch_interval: DEFAULT_INTER_BATCH_INTERVAL,
            code: Some(WorkspaceCodeIndexWorker {
                database,
                code_index,
                documents,
                embeddings,
                active_workspace: ActiveSessionWorkspace {
                    sessions,
                    workspaces,
                },
                round_robin_cursor: Mutex::new(0),
                mutations: code_mutations,
            }),
        },
    }
}

struct NativeAgentCodeRetrieval {
    code_index: Arc<SqliteCodeIndexRepository>,
    configuration: Arc<dyn RetrievalConfigurationRepository>,
    documents: Arc<dyn RetrievalDocumentRepository>,
    embeddings: Arc<dyn EmbeddingPort>,
}

impl AgentCodeRetrievalPort for NativeAgentCodeRetrieval {
    fn is_available(&self, workspace_folder: &str) -> bool {
        self.code_index
            .resolve_workspace(std::path::Path::new(workspace_folder))
            .ok()
            .flatten()
            .is_some_and(|workspace| {
                workspace.enabled
                    && workspace.phase
                        != crate::contexts::retrieval::domain::CodeIndexPhase::Unavailable
            })
    }

    fn search_code(
        &self,
        workspace_folder: &str,
        query: &str,
        limit: usize,
    ) -> Result<AgentCodeRetrievalOutcome, String> {
        let workspace = self
            .code_index
            .resolve_workspace(std::path::Path::new(workspace_folder))
            .map_err(|error| error.to_string())?
            .filter(|workspace| workspace.enabled)
            .ok_or_else(|| "code indexing is not enabled for this workspace".to_string())?;
        let service =
            crate::contexts::retrieval::application::code_search_service::CodeSearchService::new(
                workspace.workspace_id,
                self.configuration.clone(),
                self.documents.clone(),
                self.code_index.clone(),
                self.embeddings.clone(),
            )
            .map_err(|error| error.to_string())?;
        service
            .search_code(&crate::contexts::retrieval::domain::CodeSearchQuery {
                text: query.to_string(),
                limit,
            })
            .map(project_code_search_outcome)
            .map_err(|error| error.to_string())
    }
}

fn project_code_search_outcome(
    outcome: crate::contexts::retrieval::domain::CodeSearchOutcome,
) -> AgentCodeRetrievalOutcome {
    AgentCodeRetrievalOutcome {
        hits: outcome
            .hits
            .into_iter()
            .map(|hit| AgentCodeRetrievalHit {
                file_path: hit.file_path,
                start_line: hit.start_line,
                end_line: hit.end_line,
                language: hit.language,
                symbol_name: hit.symbol_name,
                symbol_kind: hit.symbol_kind,
                snippet: hit.snippet,
                matched_via: hit.matched_via.as_str().to_string(),
            })
            .collect(),
        degraded: outcome
            .degraded
            .map(|degradation| degradation.as_str().to_string()),
    }
}

/// 索引 worker 的三种驱动方式（设计文档 §5.1）：启动时先跑一轮（顺带回填历史存量记忆，
/// 不需要单独的数据迁移脚本）、保存记忆后的唤醒信号、以及定时兜底轮询——信号丢失时最多延迟
/// 一个周期。
///
/// 用独立 OS 线程而不是 `tauri::async_runtime::spawn`：本循环全程是阻塞式 I/O（rusqlite 与
/// 阻塞式 HTTP 客户端），扔进异步运行时会长时间占住 tokio 的工作线程。
pub(crate) fn start_retrieval_indexing_worker(
    worker: RetrievalIndexingWorker,
    fallback_log_directory: PathBuf,
) {
    thread::spawn(move || {
        let logging = UnifiedLoggingAdapter::active(fallback_log_directory);
        // 裸 `thread::spawn` 里的一次 panic 会让索引在进程余下的生命周期里彻底停摆，且没有
        // 任何信号：设置页仍显示上一次的计数，`recall` 仍返回旧索引的结果。至少让它留下一条
        // error 级日志。panic payload 不落盘——它可能带记忆内容或 provider 文本（§8.2），
        // 只记录"发生了 panic"这件事本身。
        if std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            worker_loop(&worker, &logging);
        }))
        .is_err()
        {
            write_log(
                &logging,
                LogSeverity::Error,
                WORKER_CATEGORY,
                "Retrieval indexing worker stopped after a panic; indexing is disabled until restart",
                [],
            );
        }
    });
}

fn worker_loop(worker: &RetrievalIndexingWorker, logging: &dyn DiagnosticLogPort) -> ! {
    loop {
        run_indexing_cycle(worker, logging);
        match worker.wakeups.recv_timeout(POLL_INTERVAL) {
            Ok(()) | Err(RecvTimeoutError::Timeout) => {}
            // 发送端全没了（门面被丢弃）。`recv_timeout` 此后会立刻返回，不自己睡一觉
            // 就会把兜底轮询变成忙等。
            Err(RecvTimeoutError::Disconnected) => thread::sleep(POLL_INTERVAL),
        }
    }
}

fn run_indexing_cycle(worker: &RetrievalIndexingWorker, logging: &dyn DiagnosticLogPort) {
    match worker.indexing.reconcile() {
        Ok(outcome) if outcome.added + outcome.invalidated + outcome.orphans_removed > 0 => {
            write_log(
                logging,
                LogSeverity::Debug,
                RECONCILE_CATEGORY,
                "Retrieval index reconciled its source snapshot",
                [
                    ("added", outcome.added.to_string()),
                    ("invalidated", outcome.invalidated.to_string()),
                    ("orphansRemoved", outcome.orphans_removed.to_string()),
                ],
            );
        }
        Ok(_) => {}
        // 协调失败不终止本轮：已经排队的 pending 行仍然值得处理。
        Err(error) => write_failure_log(
            logging,
            RECONCILE_CATEGORY,
            "Retrieval index reconciliation failed; queued documents are still processed",
            [("category", error_category(&error).to_string())],
        ),
    }
    run_workspace_code_round(worker, logging);
    if let Some(model) = configured_model(worker.configuration.as_ref()) {
        drain_pending_batches(worker, &model, logging);
    }
}

fn run_workspace_code_round(worker: &RetrievalIndexingWorker, logging: &dyn DiagnosticLogPort) {
    let Some(code) = &worker.code else {
        return;
    };
    run_targeted_code_mutations(code, logging);
    let active_root = match code.active_workspace.root() {
        Ok(root) => root,
        Err(error) => {
            write_failure_log(
                logging,
                RECONCILE_CATEGORY,
                "Active workspace lookup failed; code indexing is deferred",
                [("category", error_category(&error).to_string())],
            );
            return;
        }
    };
    let workspaces = match code.code_index.list_workspaces() {
        Ok(workspaces) => eligible_code_workspaces(workspaces, active_root.as_deref()),
        Err(error) => {
            write_failure_log(
                logging,
                RECONCILE_CATEGORY,
                "Workspace code index inventory lookup failed",
                [("category", error_category(&error).to_string())],
            );
            return;
        }
    };
    let selected = {
        let mut cursor = code
            .round_robin_cursor
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        round_robin_workspaces(&workspaces, &mut cursor, MAX_CODE_WORKSPACES_PER_CYCLE)
    };
    let configured = worker.configuration.load().ok().and_then(|configuration| {
        configuration
            .resolved_model()
            .map(|(profile, model)| (profile.to_string(), model.to_string()))
    });
    for workspace in selected {
        let reconcile_started = Instant::now();
        let outcome =
            match crate::contexts::retrieval::infrastructure::code_reconciler::reconcile_workspace(
                code.code_index.as_ref(),
                &workspace,
            ) {
                Ok(outcome) => outcome,
                Err(error) => {
                    write_failure_log(
                        logging,
                        RECONCILE_CATEGORY,
                        "Workspace code index reconciliation failed",
                        [
                            ("workspaceId", workspace.workspace_id.clone()),
                            ("category", error_category(&error).to_string()),
                        ],
                    );
                    continue;
                }
            };
        let phase = if outcome.cancelled {
            CodeIndexPhase::Cancelling
        } else if outcome.failed > 0 {
            CodeIndexPhase::Degraded
        } else {
            CodeIndexPhase::Ready
        };
        write_code_reconcile_log(
            logging,
            &workspace.workspace_id,
            workspace.generation,
            &outcome,
            reconcile_started.elapsed(),
            phase,
        );
        if workspace.mode == CodeIndexMode::Local {
            continue;
        }
        let Some((profile_id, model)) = &configured else {
            continue;
        };
        if outcome.failed > 0 {
            continue;
        }
        match crate::contexts::retrieval::application::code_embedding::prepare_code_embedding(
            code.code_index.as_ref(),
            &workspace.workspace_id,
            profile_id,
            model,
            workspace.generation,
        ) {
            Ok(true) => {}
            Ok(false) => {
                write_log(
                    logging,
                    LogSeverity::Info,
                    CODE_BATCH_CATEGORY,
                    "Workspace code embedding is awaiting explicit confirmation",
                    [
                        ("sourceKind", "workspace_file".to_string()),
                        ("workspaceId", workspace.workspace_id.clone()),
                        (
                            "phase",
                            CodeIndexPhase::AwaitingEmbeddingConfirmation
                                .as_str()
                                .to_string(),
                        ),
                        ("generation", workspace.generation.to_string()),
                        ("model", model.clone()),
                    ],
                );
                continue;
            }
            Err(error) => {
                write_failure_log(
                    logging,
                    CODE_BATCH_CATEGORY,
                    "Workspace code embedding preparation failed",
                    [
                        ("workspaceId", workspace.workspace_id.clone()),
                        ("category", error_category(&error).to_string()),
                    ],
                );
                continue;
            }
        }
        let source: Arc<dyn IndexSourcePort> = Arc::new(WorkspaceFileIndexSource::new(
            code.database.clone(),
            workspace.workspace_id.clone(),
        ));
        let service = match crate::contexts::retrieval::application::code_embedding::WorkspaceCodeEmbeddingService::new(
            code.documents.clone(),
            source,
            code.embeddings.clone(),
            code.code_index.clone(),
            workspace.workspace_id.clone(),
            workspace.generation,
            profile_id.clone(),
            model.clone(),
        ) {
            Ok(service) => service,
            Err(_) => continue,
        };
        let batch_started = Instant::now();
        match service.process_pending_batch() {
            Ok(outcome) if outcome.succeeded + outcome.failed == 0 => {
                let _ = code
                    .code_index
                    .set_workspace_phase(&workspace.workspace_id, CodeIndexPhase::Ready);
                write_code_batch_log(
                    logging,
                    &workspace.workspace_id,
                    &outcome,
                    batch_started.elapsed(),
                    model,
                    CodeIndexPhase::Ready,
                );
            }
            Ok(outcome) => {
                if outcome.succeeded > 0 {
                    let _ = code.code_index.record_audit(
                        &workspace.workspace_id,
                        None,
                        CodeIndexAuditEvent::Indexed,
                        None,
                        outcome.succeeded as u64,
                    );
                }
                if outcome.failed > 0 {
                    let _ = code.code_index.record_audit(
                        &workspace.workspace_id,
                        None,
                        CodeIndexAuditEvent::Failed,
                        None,
                        outcome.failed as u64,
                    );
                }
                write_code_batch_log(
                    logging,
                    &workspace.workspace_id,
                    &outcome,
                    batch_started.elapsed(),
                    model,
                    CodeIndexPhase::Embedding,
                );
                wait_between_batches(worker, worker.inter_batch_interval);
            }
            Err(error) => {
                write_failure_log(
                    logging,
                    CODE_BATCH_CATEGORY,
                    "Workspace code embedding batch failed before completion",
                    [
                        ("workspaceId", workspace.workspace_id.clone()),
                        ("category", error_category(&error).to_string()),
                    ],
                );
            }
        }
    }
}

fn run_targeted_code_mutations(code: &WorkspaceCodeIndexWorker, logging: &dyn DiagnosticLogPort) {
    let mutations = code.mutations.drain();
    if mutations.is_empty() {
        return;
    }
    let workspaces = match code.code_index.list_workspaces() {
        Ok(workspaces) => workspaces,
        Err(error) => {
            write_failure_log(
                logging,
                CODE_RECONCILE_CATEGORY,
                "Targeted workspace inventory lookup failed",
                [("category", error_category(&error).to_string())],
            );
            return;
        }
    };
    for mutation in mutations {
        let Some(workspace) = workspace_for_mutation(&workspaces, &mutation) else {
            continue;
        };
        let changes = mutation
            .relative_paths
            .into_iter()
            .map(
                crate::contexts::retrieval::infrastructure::code_reconciler::CodePathChange::Upsert,
            )
            .collect::<Vec<_>>();
        let started = Instant::now();
        let outcome =
            match crate::contexts::retrieval::infrastructure::code_reconciler::reconcile_paths(
                code.code_index.as_ref(),
                &workspace,
                &changes,
            ) {
                Ok(outcome) => outcome,
                Err(error) => {
                    write_failure_log(
                        logging,
                        CODE_RECONCILE_CATEGORY,
                        "Targeted workspace code reconciliation failed",
                        [
                            ("workspaceId", workspace.workspace_id.clone()),
                            ("category", error_category(&error).to_string()),
                        ],
                    );
                    continue;
                }
            };
        let phase = if outcome.cancelled {
            CodeIndexPhase::Cancelling
        } else if outcome.failed > 0 {
            CodeIndexPhase::Degraded
        } else {
            CodeIndexPhase::Ready
        };
        write_code_reconcile_log(
            logging,
            &workspace.workspace_id,
            workspace.generation,
            &outcome,
            started.elapsed(),
            phase,
        );
    }
}

fn workspace_for_mutation(
    workspaces: &[crate::contexts::retrieval::domain::CodeWorkspace],
    mutation: &CodeIndexMutationBatch,
) -> Option<crate::contexts::retrieval::domain::CodeWorkspace> {
    workspaces
        .iter()
        .find(|workspace| {
            workspace.enabled
                && Path::new(&workspace.canonical_root)
                    .canonicalize()
                    .is_ok_and(|root| root == mutation.canonical_workspace)
        })
        .cloned()
}

fn eligible_code_workspaces(
    workspaces: Vec<crate::contexts::retrieval::domain::CodeWorkspace>,
    active_root: Option<&Path>,
) -> Vec<crate::contexts::retrieval::domain::CodeWorkspace> {
    let Some(active_root) = active_root.and_then(|root| root.canonicalize().ok()) else {
        return Vec::new();
    };
    workspaces
        .into_iter()
        .filter(|workspace| {
            workspace.enabled
                && Path::new(&workspace.canonical_root)
                    .canonicalize()
                    .is_ok_and(|root| root == active_root)
        })
        .collect()
}

fn round_robin_workspaces<T: Clone>(items: &[T], cursor: &mut usize, limit: usize) -> Vec<T> {
    if items.is_empty() || limit == 0 {
        *cursor = 0;
        return Vec::new();
    }
    let count = items.len().min(limit);
    let start = *cursor % items.len();
    let selected = (0..count)
        .map(|offset| items[(start + offset) % items.len()].clone())
        .collect();
    *cursor = (start + count) % items.len();
    selected
}

/// 串行处理，不并发冲击速率限制（设计文档 §5.2）。
///
/// 连续失败 `RETRY_BACKOFF_SECONDS.len()` 批就返回，把控制权交回外层轮询循环，而不是一直
/// 排到队列见底。持续可重试的失败（429/5xx/超时）下每条文档要失败 `MAX_EMBEDDING_ATTEMPTS`
/// 次才会出队，退避又会顶到 300s，几百条 pending 就能让本函数占住整个 worker 线程几小时——
/// 而 `reconcile` 与本函数共用这一个线程。那不只是"索引慢"：`retrieval_documents` 的行只由
/// `reconcile` 建立，FTS5 影子表又建在这张表上，所以停摆期间新存的记忆连**关键词**路都搜不到，
/// `recall` 会返回空且不带 `degraded`，等于告诉模型"没有这条记忆"——正是 §8.1 与
/// `Unavailable` 变体要避免的那件事。
fn drain_pending_batches(
    worker: &RetrievalIndexingWorker,
    model: &str,
    logging: &dyn DiagnosticLogPort,
) {
    let mut consecutive_failures = 0usize;
    loop {
        let started = Instant::now();
        let outcome = match worker.indexing.process_pending_batch(model) {
            Ok(outcome) => outcome,
            Err(error) => {
                write_failure_log(
                    logging,
                    BATCH_CATEGORY,
                    "Retrieval embedding batch was abandoned after a storage failure",
                    [("category", error_category(&error).to_string())],
                );
                return;
            }
        };
        if outcome.succeeded == 0 && outcome.failed == 0 {
            return;
        }
        write_batch_log(logging, &outcome, started.elapsed(), model);
        if outcome.failed == 0 {
            consecutive_failures = 0;
            wait_between_batches(worker, worker.inter_batch_interval);
            continue;
        }
        // 可重试的失败会把行留在 `pending`：不退避就会在同一批上原地打转，把 provider 的速率
        // 限制撞穿。攀升的退避表来自设计文档 §5.2，而 `attempt_count` 的上限保证本循环终会
        // 收敛——达到上限的行被标成 `failed`，不再被认领。
        write_failure_log(
            logging,
            BATCH_CATEGORY,
            "Retrieval embedding batch reported failures; backing off before the next batch",
            [
                ("failed", outcome.failed.to_string()),
                ("attempt", (consecutive_failures + 1).to_string()),
                (
                    "category",
                    outcome
                        .last_failure_category
                        .map(FailureCategory::as_str)
                        .unwrap_or("unknown")
                        .to_string(),
                ),
            ],
        );
        // 退避表用尽即返回：外层轮询循环会重新进入，而它的第一件事是 `reconcile()`——被本轮
        // 失败挤掉的新记忆因此最多晚一个轮询周期就能拿到索引行（进而被 FTS 命中），不必等到
        // 整个 pending 队列排空。
        if consecutive_failures + 1 >= RETRY_BACKOFF_SECONDS.len() {
            return;
        }
        // 退避期间也监听唤醒信号：`rebuild()`/`save_configuration()` 全靠 `notify()` 才能不等一整个
        // 轮询周期就生效，但如果这里只会 `thread::sleep`，用户在退避中途修好配置、按下"重建"，
        // 最多要等 300s 才会看到反应。收到唤醒就立刻结束退避、回到循环顶部重试；发送端没了则
        // 退化成真正的睡眠——和 `start_retrieval_indexing_worker` 对同一种情况的处理保持一致
        // （同一个理由：`recv_timeout` 在发送端消失后会立刻返回，不特殊处理就会把退避变成忙等）。
        let delay = failure_retry_delay(&outcome, consecutive_failures);
        wait_between_batches(worker, delay);
        consecutive_failures += 1;
    }
}

fn wait_between_batches(worker: &RetrievalIndexingWorker, delay: Duration) {
    if delay.is_zero() {
        return;
    }
    match worker.wakeups.recv_timeout(delay) {
        Ok(()) | Err(RecvTimeoutError::Timeout) => {}
        Err(RecvTimeoutError::Disconnected) => thread::sleep(delay),
    }
}

fn failure_retry_delay(outcome: &BatchOutcome, consecutive_failures: usize) -> Duration {
    let provider_delay = outcome
        .retry_after
        .unwrap_or_default()
        .min(MAX_PROVIDER_RETRY_AFTER);
    retry_backoff(consecutive_failures).max(provider_delay)
}

fn retry_backoff(consecutive_failures: usize) -> Duration {
    let step = consecutive_failures.min(RETRY_BACKOFF_SECONDS.len() - 1);
    Duration::from_secs(RETRY_BACKOFF_SECONDS[step])
}

/// 这里的 `.ok()` 同样把"读配置失败"与"没配置"揉成一个 `None`，但在这里是安全的：`None`
/// 只让本轮 `run_indexing_cycle` 提前返回，不落任何 `failed` 状态，下一轮轮询/唤醒会重新读
/// 配置——不像 `ConfiguredProfileEmbeddingAdapter::embed`，这里没有"一批文档被打成永久失败"
/// 的下游后果，不需要区分瞬时故障与确定性的"没配置"。
fn configured_model(configuration: &dyn RetrievalConfigurationRepository) -> Option<String> {
    configuration
        .load()
        .ok()?
        .resolved_model()
        .map(|(_profile, model)| model.to_string())
}

/// 批次完成日志的字段照设计文档 §8.2：批大小、耗时、成功/失败条数、模型 id。全部是计数与
/// 标识符，没有一项可能承载记忆内容或凭据。
fn write_batch_log(
    logging: &dyn DiagnosticLogPort,
    outcome: &BatchOutcome,
    elapsed: Duration,
    model: &str,
) {
    write_log(
        logging,
        LogSeverity::Info,
        BATCH_CATEGORY,
        "Retrieval embedding batch completed",
        [
            (
                "batchSize",
                (outcome.succeeded + outcome.failed).to_string(),
            ),
            ("succeeded", outcome.succeeded.to_string()),
            ("failed", outcome.failed.to_string()),
            ("durationMs", elapsed.as_millis().to_string()),
            ("model", model.to_string()),
        ],
    );
}

fn write_code_reconcile_log(
    logging: &dyn DiagnosticLogPort,
    workspace_id: &str,
    generation: u64,
    outcome: &crate::contexts::retrieval::infrastructure::code_reconciler::CodeReconcileOutcome,
    elapsed: Duration,
    phase: CodeIndexPhase,
) {
    write_log(
        logging,
        LogSeverity::Info,
        CODE_RECONCILE_CATEGORY,
        "Workspace code index reconciliation completed",
        [
            ("sourceKind", "workspace_file".to_string()),
            ("workspaceId", workspace_id.to_string()),
            ("phase", phase.as_str().to_string()),
            ("generation", generation.to_string()),
            ("discovered", outcome.discovered.to_string()),
            ("replaced", outcome.replaced.to_string()),
            ("deleted", outcome.deleted.to_string()),
            ("failed", outcome.failed.to_string()),
            ("durationMs", elapsed.as_millis().to_string()),
        ],
    );
}

fn write_code_batch_log(
    logging: &dyn DiagnosticLogPort,
    workspace_id: &str,
    outcome: &BatchOutcome,
    elapsed: Duration,
    model: &str,
    phase: CodeIndexPhase,
) {
    write_log(
        logging,
        LogSeverity::Info,
        CODE_BATCH_CATEGORY,
        "Workspace code embedding batch completed",
        [
            ("sourceKind", "workspace_file".to_string()),
            ("workspaceId", workspace_id.to_string()),
            ("phase", phase.as_str().to_string()),
            (
                "batchSize",
                (outcome.succeeded + outcome.failed).to_string(),
            ),
            ("succeeded", outcome.succeeded.to_string()),
            ("failed", outcome.failed.to_string()),
            ("durationMs", elapsed.as_millis().to_string()),
            ("model", model.to_string()),
        ],
    );
}

fn write_failure_log<const N: usize>(
    logging: &dyn DiagnosticLogPort,
    category: &str,
    message: &str,
    context: [(&str, String); N],
) {
    write_log(logging, LogSeverity::Warn, category, message, context);
}

fn write_log<const N: usize>(
    logging: &dyn DiagnosticLogPort,
    severity: LogSeverity,
    category: &str,
    message: &str,
    context: [(&str, String); N],
) {
    let mut fields = BTreeMap::from([("source".to_string(), "background-indexing".to_string())]);
    fields.extend(
        context
            .into_iter()
            .map(|(key, value)| (key.to_string(), value)),
    );
    let _ = logging.write_diagnostic(DiagnosticLog {
        severity,
        category: category.to_string(),
        message: message.to_string(),
        context: fields,
    });
}

/// 只把变体名写进日志。`RetrievalError` 的载荷可能带存储层原文（例如 rusqlite 的消息），而
/// 设计文档 §8.2 只允许落盘错误**类别**。映射本身住在领域类型上，command 层向 UI 返回的
/// 错误串走同一份，两处不会各自漂移。
fn error_category(error: &RetrievalError) -> &'static str {
    error.category()
}

/// `retrieval` 的索引源。它只知道"给我全部源记录"，不知道 `agent_memories` 表和
/// `AgentMemory` 类型的存在——那些细节到本文件为止。
/// `None` when the memory directory could not be opened at startup.
///
/// Both methods then report storage unavailability rather than returning an empty result. An empty
/// snapshot is not a safe stand-in: reconciliation treats anything missing from it as deleted, so
/// a failed store would silently wipe every indexed memory instead of leaving the index intact
/// until the directory is reachable again.
struct GovernedMemoryIndexSource {
    personalization: PersonalizationApi,
}

impl IndexSourcePort for GovernedMemoryIndexSource {
    /// The governed store is the authoritative snapshot, so a memory added or removed outside the
    /// application converges on the next reconcile with no user action.
    ///
    /// Empty while migration has not finished, which is the point: indexing a half-migrated store
    /// would leave documents for memories that are about to be rewritten, and the reconcile that
    /// runs once it *is* finished removes anything that no longer exists — including every row
    /// keyed on a pre-v2 path.
    fn snapshot(&self) -> Result<Vec<IndexSourceRecord>, RetrievalError> {
        let memories = self
            .personalization
            .compatibility_memories()
            .map_err(|error| RetrievalError::Storage(error.to_string()))?;
        Ok(memories.into_iter().map(index_source_record).collect())
    }

    /// Hits resolve by reading the named records. One whose record is gone is simply absent, which
    /// is what stops a deleted memory being surfaced from a surviving index row.
    fn fetch(&self, source_ids: &[String]) -> Result<Vec<IndexSourceRecord>, RetrievalError> {
        let memories = self
            .personalization
            .compatibility_memories_by_handle(source_ids)
            .map_err(|error| RetrievalError::Storage(error.to_string()))?;
        Ok(memories.into_iter().map(index_source_record).collect())
    }
}

fn index_source_record(
    memory: crate::contexts::personalization::api::CompatibilityMemory,
) -> IndexSourceRecord {
    IndexSourceRecord {
        // The v2 file name, which is what the compatibility view hands out as a handle everywhere
        // else. Keying on anything else would make a hit unresolvable.
        source_id: memory.file_name,
        agent_id: memory.source_agent_id.unwrap_or_default(),
        // 无工作区文件夹用空串哨兵，与检索侧 scope 的映射一致；两侧不一致就永远搜不到。
        folder: memory.source_workspace.unwrap_or_default(),
        content: memory.content,
        created_at: memory
            .created_at
            .to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
    }
}

/// `retrieval` 声明"给我一个可用的 embedding 端点"，`agent_runtime` 拥有 Profile 与凭据——
/// 这个适配器是两者之间唯一的那根线（设计文档 §4.3）。
struct AgentRuntimeEmbeddingEndpoint {
    agent_runtime: AgentRuntimeApi,
}

impl EmbeddingEndpointPort for AgentRuntimeEmbeddingEndpoint {
    fn resolve(&self, profile_id: &str) -> Result<ResolvedEmbeddingEndpoint, RetrievalError> {
        // 底层错误一律折叠成这一句**字面量**，绝不插值：调用方
        // （`openai_embedding_adapter.rs` 的 `embed`）会把本错误的 Display 拼进
        // `EmbeddingFailure::message`，插值等于给凭据与 provider 文本开了一条渗出通道。
        // 代价是丢掉"profile 不存在 / 没有 API key / 接口格式不对"的区分，用户仍能从设置页
        // 的失败计数与 `invalid_request` 类别看到问题。
        let view = self
            .agent_runtime
            .resolve_embedding_endpoint(profile_id)
            .map_err(|_| {
                RetrievalError::Embedding(
                    "the configured embedding profile could not be resolved".to_string(),
                )
            })?;
        Ok(ResolvedEmbeddingEndpoint {
            base_url: view.base_url,
            credential: view.credential,
        })
    }
}

/// 每次调用都按**当前**配置解析 Profile。`HttpEmbeddingAdapter` 的 profile id 是构造期固定的，
/// 装配时读一次就意味着用户在设置页换了 Profile 之后必须重启应用才生效。
struct ConfiguredProfileEmbeddingAdapter {
    configuration: Arc<dyn RetrievalConfigurationRepository>,
    endpoint: Arc<dyn EmbeddingEndpointPort>,
}

impl EmbeddingPort for ConfiguredProfileEmbeddingAdapter {
    fn embed(&self, model: &str, inputs: &[String]) -> Result<Vec<Vec<f32>>, EmbeddingFailure> {
        // 读配置失败与"没配置"是两回事：前者是瞬时的（连接池 checkout 超时），归为可重试，
        // 让退避在下一轮重来；后者是确定性的，失败即停。混为一谈会让一次池超时把整批
        // 32 条记忆永久打成 failed，而 reconcile 因内容哈希未变会保留 failed，只能靠人工重建。
        let configuration = self.configuration.load().map_err(|_| EmbeddingFailure {
            category: FailureCategory::Network,
            retry_after: None,
            message: "the retrieval configuration could not be read".to_string(),
        })?;
        let profile_id = configuration
            .resolved_model()
            .map(|(profile, _model)| profile.to_string())
            .ok_or_else(|| EmbeddingFailure {
                category: FailureCategory::InvalidRequest,
                retry_after: None,
                message: "no embedding profile is configured".to_string(),
            })?;
        HttpEmbeddingAdapter::new(self.endpoint.clone(), profile_id).embed(model, inputs)
    }
}

/// Bridges `RuntimeAgentApiAdapter` (assembled by `bootstrap::assemble_agent_runtime_api`, whose
/// output `assemble_retrieval` itself depends on — see this module's own `AgentRuntimeApi`
/// dependency above) to the real `RetrievalApi`, assembled afterward. `runtime.rs`'s `setup`
/// therefore cannot hand `RuntimeAgentApiAdapter` a working `RetrievalApi` at construction time —
/// this cell is constructed empty, passed in as `Arc<dyn AgentRetrievalPort>`, and `bind` is
/// called exactly once immediately after `assemble_retrieval` returns, well before any generation
/// can run. Every method degrades exactly like "not configured yet" until then, matching
/// `AgentRetrievalPort::is_configured`'s existing contract of never blocking or panicking.
#[derive(Default)]
pub(crate) struct DeferredAgentRetrieval {
    bound: OnceLock<RetrievalApi>,
    code: OnceLock<Arc<dyn AgentCodeRetrievalPort>>,
}

impl DeferredAgentRetrieval {
    /// Called exactly once, from `runtime.rs`'s `setup`, after both
    /// `assemble_agent_runtime_api` and `assemble_retrieval` have returned. A second call is a
    /// bootstrap ordering bug, not a runtime condition to recover from — `OnceLock::set`'s `Err`
    /// (the rejected value) is deliberately discarded rather than surfaced.
    pub(crate) fn bind(&self, retrieval: RetrievalApi) {
        let _ = self.bound.set(retrieval);
    }

    pub(crate) fn bind_code(&self, retrieval: Arc<dyn AgentCodeRetrievalPort>) {
        let _ = self.code.set(retrieval);
    }
}

impl AgentRetrievalPort for DeferredAgentRetrieval {
    fn is_configured(&self) -> bool {
        self.bound.get().is_some_and(RetrievalApi::is_configured)
    }

    fn search(&self, query: &str, limit: usize) -> Result<AgentRetrievalOutcome, String> {
        let Some(retrieval) = self.bound.get() else {
            return Err("retrieval is not yet available".to_string());
        };
        retrieval
            .search(query, limit)
            .map(project_search_outcome)
            .map_err(|error| error.to_string())
    }

    fn notify_source_changed(&self) {
        if let Some(retrieval) = self.bound.get() {
            retrieval.wake_worker();
        }
    }

    fn code_retrieval(&self) -> Option<&dyn AgentCodeRetrievalPort> {
        self.code.get().map(Arc::as_ref)
    }
}

/// Projects `retrieval`'s internal `SearchOutcome` into `agent_runtime`'s own, deliberately
/// narrower shape for the `recall` tool result — `source_id`/`score` are internal to `retrieval`
/// and stop here; they never cross the context boundary (Task 13 brief).
fn project_search_outcome(outcome: SearchOutcome) -> AgentRetrievalOutcome {
    AgentRetrievalOutcome {
        hits: outcome
            .hits
            .into_iter()
            .map(|hit| AgentRetrievalHit {
                content: hit.content,
                created_at: hit.created_at,
                matched_via: hit.matched_via.as_str().to_string(),
            })
            .collect(),
        degraded: outcome
            .degraded
            .map(|degradation| degradation.as_str().to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contexts::operations::api::OperationsError;
    use crate::contexts::retrieval::application::{RetrievalConfiguration, RetrievalIndexStatus};
    use crate::contexts::retrieval::domain::{
        CodeWorkspace, Degradation, IndexState, MatchedVia, RetrievalDocument, ScoredHit,
        SourceKind,
    };
    use crate::test_support::TempDirectory;
    use std::sync::mpsc::sync_channel;
    use std::sync::Mutex;

    const MODEL: &str = "test-embedding-model";

    #[test]
    fn the_retry_backoff_follows_the_documented_table_and_then_holds() {
        // 设计文档 §5.2 的退避表：1s / 4s / 15s / 60s / 300s。超出表长后停在最后一档，
        // 而不是越界 panic——`attempt_count` 的上限与本循环的失败计数不是同一个计数器。
        let steps = (0..RETRY_BACKOFF_SECONDS.len())
            .map(|failures| retry_backoff(failures).as_secs())
            .collect::<Vec<_>>();
        assert_eq!(steps, vec![1, 4, 15, 60, 300]);
        assert_eq!(
            retry_backoff(RETRY_BACKOFF_SECONDS.len() + 10).as_secs(),
            300
        );
    }

    #[test]
    fn provider_retry_after_is_bounded_and_never_shortens_the_worker_backoff() {
        let outcome = BatchOutcome {
            failed: 1,
            retry_after: Some(Duration::from_secs(10)),
            ..BatchOutcome::default()
        };
        assert_eq!(failure_retry_delay(&outcome, 0), Duration::from_secs(10));
        assert_eq!(failure_retry_delay(&outcome, 3), Duration::from_secs(60));

        let excessive = BatchOutcome {
            failed: 1,
            retry_after: Some(Duration::from_secs(600)),
            ..BatchOutcome::default()
        };
        assert_eq!(failure_retry_delay(&excessive, 0), MAX_PROVIDER_RETRY_AFTER);
    }

    #[test]
    fn error_category_maps_each_variant_without_the_payload_text() {
        // 这条只钉死纯函数 `error_category` 的映射表，不驱动任何 `write_*_log` 调用点——
        // 它不能证明日志管线本身不泄漏，那部分由下面两条驱动真实调用路径的测试负责
        // （`a_reconcile_failure_...` / `a_batch_outcome_failure_...`）。
        assert_eq!(
            error_category(&RetrievalError::Storage("SENSITIVE-SENTINEL".to_string())),
            "storage"
        );
        assert_eq!(
            error_category(&RetrievalError::Embedding("SENSITIVE-SENTINEL".to_string())),
            "embedding"
        );
        assert_eq!(
            error_category(&RetrievalError::NotConfigured),
            "not_configured"
        );
        assert_eq!(error_category(&RetrievalError::Unavailable), "unavailable");
    }

    /// 三种可编排行为，与 `retrieval::api` 测试模块里的同名 fake 同构：已配置 / 未配置 /
    /// 读配置本身失败（模拟 r2d2 连接池 checkout 超时这类瞬时故障）。
    enum FakeConfigurationRepository {
        Configured,
        Unconfigured,
        Failing,
    }

    impl RetrievalConfigurationRepository for FakeConfigurationRepository {
        fn load(&self) -> Result<RetrievalConfiguration, RetrievalError> {
            match self {
                Self::Configured => Ok(RetrievalConfiguration {
                    source_profile_id: Some("profile-a".to_string()),
                    embedding_model: Some(MODEL.to_string()),
                    automatic_code_index_mode: Default::default(),
                }),
                Self::Unconfigured => Ok(RetrievalConfiguration::default()),
                Self::Failing => Err(RetrievalError::Storage(
                    "connection pool checkout timed out".to_string(),
                )),
            }
        }

        fn save(&self, _profile_id: &str, _embedding_model: &str) -> Result<(), RetrievalError> {
            unimplemented!("not exercised by these tests")
        }

        fn save_automatic_code_index_mode(
            &self,
            _mode: crate::contexts::retrieval::domain::CodeIndexAutomaticMode,
        ) -> Result<(), RetrievalError> {
            unimplemented!("not exercised by these tests")
        }
    }

    struct UnreachableEmbeddingEndpoint;

    impl EmbeddingEndpointPort for UnreachableEmbeddingEndpoint {
        fn resolve(&self, _profile_id: &str) -> Result<ResolvedEmbeddingEndpoint, RetrievalError> {
            panic!("must not resolve an endpoint once profile resolution has already failed")
        }
    }

    #[test]
    fn a_transient_configuration_read_failure_is_retryable_not_invalid_request() {
        // r2d2 连接池 checkout 超时这类瞬时故障必须映射成 `Network`（可重试），而不是
        // `InvalidRequest`（确定性、立刻放弃）——否则一次池超时就会把整批 32 条记忆永久
        // 打成 failed，且 reconcile 因内容哈希未变不会重新入队，只能靠人工重建（Fix 1）。
        let adapter = ConfiguredProfileEmbeddingAdapter {
            configuration: Arc::new(FakeConfigurationRepository::Failing),
            endpoint: Arc::new(UnreachableEmbeddingEndpoint),
        };

        let failure = adapter
            .embed(MODEL, &["hello".to_string()])
            .expect_err("a storage failure while reading configuration must not succeed");

        assert_eq!(failure.category, FailureCategory::Network);
    }

    #[test]
    fn a_genuinely_unconfigured_profile_is_still_invalid_request() {
        // 与上一条成对存在：真的没配置时必须保持确定性失败，不能被 Fix 1 误改成可重试——
        // 那会让从未配置过 embedding 的用户被无限重试烧配额。
        let adapter = ConfiguredProfileEmbeddingAdapter {
            configuration: Arc::new(FakeConfigurationRepository::Unconfigured),
            endpoint: Arc::new(UnreachableEmbeddingEndpoint),
        };

        let failure = adapter
            .embed(MODEL, &["hello".to_string()])
            .expect_err("an unconfigured profile must fail");

        assert_eq!(failure.category, FailureCategory::InvalidRequest);
    }

    #[derive(Default)]
    struct CapturingLogPort {
        logs: Mutex<Vec<DiagnosticLog>>,
    }

    impl DiagnosticLogPort for CapturingLogPort {
        fn write_diagnostic(&self, log: DiagnosticLog) -> Result<(), OperationsError> {
            self.logs.lock().expect("lock").push(log);
            Ok(())
        }
    }

    struct FailingSource;

    impl IndexSourcePort for FailingSource {
        fn snapshot(&self) -> Result<Vec<IndexSourceRecord>, RetrievalError> {
            Err(RetrievalError::Storage("SENSITIVE-SENTINEL".to_string()))
        }
        fn fetch(&self, _source_ids: &[String]) -> Result<Vec<IndexSourceRecord>, RetrievalError> {
            unimplemented!("these tests drive the indexing worker, not the search path")
        }
    }

    struct EmptySource;

    impl IndexSourcePort for EmptySource {
        fn snapshot(&self) -> Result<Vec<IndexSourceRecord>, RetrievalError> {
            Ok(Vec::new())
        }
        fn fetch(&self, _source_ids: &[String]) -> Result<Vec<IndexSourceRecord>, RetrievalError> {
            unimplemented!("these tests drive the indexing worker, not the search path")
        }
    }

    struct FailingEmbedder;

    impl EmbeddingPort for FailingEmbedder {
        fn embed(
            &self,
            _model: &str,
            _inputs: &[String],
        ) -> Result<Vec<Vec<f32>>, EmbeddingFailure> {
            // Auth 是确定性失败：不论 attempt_count 多少都会立刻 give_up（见
            // indexing_service.rs 的 an_auth_failure_gives_up_immediately_...），这样下面驱动
            // drain_pending_batches/run_indexing_cycle 的测试不用关心重试计数是否达到上限。
            Err(EmbeddingFailure {
                category: FailureCategory::Auth,
                retry_after: None,
                message: "SENSITIVE-SENTINEL".to_string(),
            })
        }
    }

    /// `claim_pending_batch` 只在第一次调用时吐出一条待处理文档，之后返回空——模拟真实仓储里
    /// "已被认领过的行不会再被认领"，让被测的循环能自然收敛到 `BatchOutcome::default()`，不需要
    /// 在测试里实现完整的状态机。
    #[derive(Default)]
    struct OnceThenEmptyRepository {
        claimed: Mutex<bool>,
    }

    impl RetrievalDocumentRepository for OnceThenEmptyRepository {
        fn upsert_pending(&self, _document: &RetrievalDocument) -> Result<(), RetrievalError> {
            unimplemented!("not exercised by these tests")
        }
        fn list_indexed_source_ids(
            &self,
            _source_kind: SourceKind,
        ) -> Result<Vec<(String, String)>, RetrievalError> {
            Ok(Vec::new())
        }
        fn delete_by_source(
            &self,
            _source_kind: SourceKind,
            _source_id: &str,
        ) -> Result<(), RetrievalError> {
            unimplemented!("not exercised by these tests")
        }
        fn claim_pending_batch(
            &self,
            _source_kind: SourceKind,
            _limit: usize,
        ) -> Result<Vec<RetrievalDocument>, RetrievalError> {
            let mut claimed = self.claimed.lock().expect("lock");
            if *claimed {
                return Ok(Vec::new());
            }
            *claimed = true;
            Ok(vec![RetrievalDocument {
                id: "agent_memory:m1".to_string(),
                source_kind: SourceKind::AgentMemory,
                source_id: "m1".to_string(),
                scope_agent_id: "agent-a".to_string(),
                scope_folder: String::new(),
                content: "uses npm".to_string(),
                content_hash: "irrelevant".to_string(),
                index_state: IndexState::Pending,
                attempt_count: 0,
                embedding_model: None,
            }])
        }
        fn store_embedding(
            &self,
            _id: &str,
            _model: &str,
            _embedding: &[f32],
        ) -> Result<(), RetrievalError> {
            unimplemented!("not exercised by these tests")
        }
        fn record_failure(
            &self,
            _id: &str,
            _category: FailureCategory,
            _give_up: bool,
        ) -> Result<(), RetrievalError> {
            Ok(())
        }
        fn vector_candidates(
            &self,
            _source_kind: SourceKind,
            _model: &str,
        ) -> Result<Vec<(String, Vec<f32>)>, RetrievalError> {
            unimplemented!("not exercised by these tests")
        }
        fn keyword_candidates(
            &self,
            _source_kind: SourceKind,
            _query: &str,
            _limit: usize,
        ) -> Result<Vec<String>, RetrievalError> {
            unimplemented!("not exercised by these tests")
        }
        fn index_status(&self) -> Result<RetrievalIndexStatus, RetrievalError> {
            unimplemented!("not exercised by these tests")
        }
        fn requeue_all(&self) -> Result<(), RetrievalError> {
            unimplemented!("not exercised by these tests")
        }
        fn requeue_stale_model(&self, _new_model: &str) -> Result<(), RetrievalError> {
            unimplemented!("not exercised by these tests")
        }
    }

    /// 持续可重试地失败：`RateLimit` 是 `is_retryable()` 的，配合 `attempt_count: 0` 的文档，
    /// `process_pending_batch` 每次都把行留在 `pending`——这正是 429/5xx 持续发生时的现实。
    struct RetryableFailingEmbedder;

    impl EmbeddingPort for RetryableFailingEmbedder {
        fn embed(
            &self,
            _model: &str,
            _inputs: &[String],
        ) -> Result<Vec<Vec<f32>>, EmbeddingFailure> {
            Err(EmbeddingFailure {
                category: FailureCategory::RateLimit,
                retry_after: None,
                message: "SENSITIVE-SENTINEL".to_string(),
            })
        }
    }

    /// `claim_pending_batch` 永远吐得出下一条待处理文档——模拟持续失败下排不空的 pending
    /// 队列。计数用来断言 drain 的批次上限。
    #[derive(Default)]
    struct AlwaysPendingRepository {
        claims: Mutex<usize>,
    }

    impl RetrievalDocumentRepository for AlwaysPendingRepository {
        fn upsert_pending(&self, _document: &RetrievalDocument) -> Result<(), RetrievalError> {
            unimplemented!("not exercised by these tests")
        }
        fn list_indexed_source_ids(
            &self,
            _source_kind: SourceKind,
        ) -> Result<Vec<(String, String)>, RetrievalError> {
            Ok(Vec::new())
        }
        fn delete_by_source(
            &self,
            _source_kind: SourceKind,
            _source_id: &str,
        ) -> Result<(), RetrievalError> {
            unimplemented!("not exercised by these tests")
        }
        fn claim_pending_batch(
            &self,
            _source_kind: SourceKind,
            _limit: usize,
        ) -> Result<Vec<RetrievalDocument>, RetrievalError> {
            *self.claims.lock().expect("lock") += 1;
            Ok(vec![RetrievalDocument {
                id: "agent_memory:m1".to_string(),
                source_kind: SourceKind::AgentMemory,
                source_id: "m1".to_string(),
                scope_agent_id: "agent-a".to_string(),
                scope_folder: String::new(),
                content: "uses npm".to_string(),
                content_hash: "irrelevant".to_string(),
                index_state: IndexState::Pending,
                attempt_count: 0,
                embedding_model: None,
            }])
        }
        fn store_embedding(
            &self,
            _id: &str,
            _model: &str,
            _embedding: &[f32],
        ) -> Result<(), RetrievalError> {
            unimplemented!("not exercised by these tests")
        }
        fn record_failure(
            &self,
            _id: &str,
            _category: FailureCategory,
            _give_up: bool,
        ) -> Result<(), RetrievalError> {
            Ok(())
        }
        fn vector_candidates(
            &self,
            _source_kind: SourceKind,
            _model: &str,
        ) -> Result<Vec<(String, Vec<f32>)>, RetrievalError> {
            unimplemented!("not exercised by these tests")
        }
        fn keyword_candidates(
            &self,
            _source_kind: SourceKind,
            _query: &str,
            _limit: usize,
        ) -> Result<Vec<String>, RetrievalError> {
            unimplemented!("not exercised by these tests")
        }
        fn index_status(&self) -> Result<RetrievalIndexStatus, RetrievalError> {
            unimplemented!("not exercised by these tests")
        }
        fn requeue_all(&self) -> Result<(), RetrievalError> {
            unimplemented!("not exercised by these tests")
        }
        fn requeue_stale_model(&self, _new_model: &str) -> Result<(), RetrievalError> {
            unimplemented!("not exercised by these tests")
        }
    }

    #[test]
    fn the_drain_returns_after_the_backoff_table_instead_of_starving_reconcile() {
        // 回归对象：无上限的 drain。`reconcile` 与 drain 共用同一个 worker 线程，pending 队列
        // 在持续可重试失败下排不空时，无上限的 drain 会把线程占住数小时；期间新存的记忆连
        // `retrieval_documents` 的行都建不起来，FTS 也就搜不到它们。
        //
        // 用一个持续发唤醒的线程把每次退避立刻打断，好让"批次数有上限"这件事能在秒级内被断言
        // ——被测的是循环的终止条件，退避时长本身由另外两条测试盯着。
        let repository = Arc::new(AlwaysPendingRepository::default());
        let (wake_tx, wakeups) = sync_channel::<()>(1);
        let worker = RetrievalIndexingWorker {
            indexing: IndexingService::new(
                repository.clone(),
                Arc::new(EmptySource),
                Arc::new(RetryableFailingEmbedder),
            ),
            configuration: Arc::new(FakeConfigurationRepository::Configured),
            wakeups,
            inter_batch_interval: Duration::ZERO,
            code: None,
        };
        // 接收端随 worker 一起在下面的线程里被丢弃，`send` 届时返回 Err，本线程自然退出。
        thread::spawn(move || while wake_tx.send(()).is_ok() {});

        let (done_tx, done_rx) = sync_channel::<()>(1);
        thread::spawn(move || {
            drain_pending_batches(&worker, MODEL, &CapturingLogPort::default());
            let _ = done_tx.send(());
        });

        done_rx
            .recv_timeout(Duration::from_secs(30))
            .expect("drain_pending_batches must return while batches still fail, not loop forever");
        assert_eq!(
            *repository.claims.lock().expect("lock"),
            RETRY_BACKOFF_SECONDS.len(),
            "the drain must hand control back after the backoff table is exhausted"
        );
    }

    #[test]
    fn workspace_round_robin_is_bounded_and_advances_past_large_queues() {
        let workspaces = (0..10).collect::<Vec<_>>();
        let mut cursor = 0;
        assert_eq!(
            round_robin_workspaces(&workspaces, &mut cursor, 3),
            vec![0, 1, 2]
        );
        assert_eq!(
            round_robin_workspaces(&workspaces, &mut cursor, 3),
            vec![3, 4, 5]
        );
        assert_eq!(
            round_robin_workspaces(&workspaces, &mut cursor, 3),
            vec![6, 7, 8]
        );
        assert_eq!(
            round_robin_workspaces(&workspaces, &mut cursor, 3),
            vec![9, 0, 1]
        );
        assert_eq!(cursor, 2);
    }

    #[test]
    fn workspace_code_round_only_selects_the_active_workspace() {
        let active = TempDirectory::new("active-code-workspace");
        let inactive = TempDirectory::new("inactive-code-workspace");
        let mut active_workspace = CodeWorkspace::new(
            active.path().to_string_lossy().into_owned(),
            "active".to_string(),
        );
        active_workspace.enabled = true;
        let active_id = active_workspace.workspace_id.clone();
        let mut inactive_workspace = CodeWorkspace::new(
            inactive.path().to_string_lossy().into_owned(),
            "inactive".to_string(),
        );
        inactive_workspace.enabled = true;

        let selected = eligible_code_workspaces(
            vec![inactive_workspace.clone(), active_workspace],
            Some(active.path()),
        );

        assert_eq!(
            selected
                .iter()
                .map(|workspace| workspace.workspace_id.as_str())
                .collect::<Vec<_>>(),
            vec![active_id]
        );
        assert!(eligible_code_workspaces(vec![inactive_workspace], None).is_empty());
    }

    #[test]
    fn targeted_mutation_maps_canonical_root_only_to_an_enabled_workspace() {
        let root = TempDirectory::new("targeted-mutation-root");
        let other = TempDirectory::new("targeted-mutation-other");
        let mut enabled = CodeWorkspace::new(
            root.path().to_string_lossy().into_owned(),
            "enabled".to_string(),
        );
        enabled.enabled = true;
        let enabled_id = enabled.workspace_id.clone();
        let disabled = CodeWorkspace::new(
            root.path().to_string_lossy().into_owned(),
            "disabled".to_string(),
        );
        let mutation = CodeIndexMutationBatch {
            canonical_workspace: root.path().canonicalize().expect("canonical root"),
            relative_paths: vec!["src/lib.rs".to_string()],
        };

        assert_eq!(
            workspace_for_mutation(&[disabled, enabled.clone()], &mutation)
                .expect("enabled workspace")
                .workspace_id,
            enabled_id
        );
        let outside = CodeIndexMutationBatch {
            canonical_workspace: other.path().canonicalize().expect("canonical other root"),
            relative_paths: vec!["src/lib.rs".to_string()],
        };
        assert!(workspace_for_mutation(&[enabled], &outside).is_none());
    }

    /// 断言一组 `DiagnosticLog` 里没有任何一条的 message 或 context 值携带了哨兵文本。
    /// Fix 3 要防的正是这种泄漏：`write_failure_log` 的某个调用点若被改成塞入
    /// `error.to_string()`/`EmbeddingFailure::message` 之类的原始载荷，这个断言就会失败。
    fn assert_no_log_carries_the_sentinel(logs: &[DiagnosticLog]) {
        for log in logs {
            assert!(
                !log.message.contains("SENSITIVE-SENTINEL"),
                "a log message leaked the underlying payload: {log:?}"
            );
            for value in log.context.values() {
                assert!(
                    !value.contains("SENSITIVE-SENTINEL"),
                    "a log context value leaked the underlying payload: {log:?}"
                );
            }
        }
    }

    #[test]
    fn code_index_diagnostics_expose_only_the_safe_field_whitelist() {
        use crate::contexts::retrieval::infrastructure::code_reconciler::CodeReconcileOutcome;

        let logging = CapturingLogPort::default();
        write_code_reconcile_log(
            &logging,
            "workspace-safe",
            3,
            &CodeReconcileOutcome {
                discovered: 4,
                replaced: 2,
                deleted: 1,
                ..CodeReconcileOutcome::default()
            },
            Duration::from_millis(12),
            CodeIndexPhase::Ready,
        );
        write_code_batch_log(
            &logging,
            "workspace-safe",
            &BatchOutcome {
                succeeded: 2,
                ..BatchOutcome::default()
            },
            Duration::from_millis(8),
            "model-safe",
            CodeIndexPhase::Embedding,
        );

        let allowed = [
            "source",
            "sourceKind",
            "workspaceId",
            "phase",
            "generation",
            "discovered",
            "replaced",
            "deleted",
            "failed",
            "durationMs",
            "batchSize",
            "succeeded",
            "model",
        ];
        let logs = logging.logs.lock().expect("lock");
        assert_eq!(logs.len(), 2);
        for log in logs.iter() {
            assert!(log
                .context
                .keys()
                .all(|key| allowed.contains(&key.as_str())));
            let rendered = format!("{log:?}").to_ascii_lowercase();
            for forbidden in [
                "rawcode-sentinel",
                "query-sentinel",
                "credential-sentinel",
                "detected-secret-sentinel",
                "c:\\private\\sentinel",
                "/private/sentinel",
                "provider-body-sentinel",
            ] {
                assert!(!rendered.contains(forbidden));
            }
        }
    }

    #[test]
    fn a_reconcile_failure_is_logged_by_category_without_the_underlying_storage_error_text() {
        // 驱动真实调用点——`run_indexing_cycle` 里的 reconcile 失败分支——而不是直接调纯函数
        // `error_category`，证明设计文档 §8.2 的"只落盘类别"约束在实际日志管线里成立。
        let logging = CapturingLogPort::default();
        let (_wake_tx, wakeups) = sync_channel::<()>(1);
        let worker = RetrievalIndexingWorker {
            indexing: IndexingService::new(
                Arc::new(OnceThenEmptyRepository::default()),
                Arc::new(FailingSource),
                Arc::new(FailingEmbedder),
            ),
            configuration: Arc::new(FakeConfigurationRepository::Unconfigured),
            wakeups,
            inter_batch_interval: Duration::ZERO,
            code: None,
        };

        run_indexing_cycle(&worker, &logging);

        let logs = logging.logs.lock().expect("lock");
        let reconcile_log = logs
            .iter()
            .find(|log| log.category == RECONCILE_CATEGORY)
            .expect("a reconcile failure must be logged");
        assert_eq!(reconcile_log.severity, LogSeverity::Warn);
        assert_eq!(
            reconcile_log.context.get("category").map(String::as_str),
            Some("storage")
        );
        assert_no_log_carries_the_sentinel(&logs);
    }

    #[test]
    fn a_batch_outcome_failure_is_logged_with_category_and_never_the_underlying_message() {
        // 驱动 `drain_pending_batches` 里 Fix 2 新加的 category 字段所在的那条 warn：
        // provider 的失败类别必须出现，但 `EmbeddingFailure::message`（可能夹带响应体片段）
        // 绝不能。
        let logging = CapturingLogPort::default();
        let (_wake_tx, wakeups) = sync_channel::<()>(1);
        let worker = RetrievalIndexingWorker {
            indexing: IndexingService::new(
                Arc::new(OnceThenEmptyRepository::default()),
                Arc::new(EmptySource),
                Arc::new(FailingEmbedder),
            ),
            configuration: Arc::new(FakeConfigurationRepository::Configured),
            wakeups,
            inter_batch_interval: Duration::ZERO,
            code: None,
        };

        run_indexing_cycle(&worker, &logging);

        let logs = logging.logs.lock().expect("lock");
        let failure_log = logs
            .iter()
            .find(|log| log.message.contains("backing off"))
            .expect("a batch failure must be logged with a backoff message");
        assert_eq!(
            failure_log.context.get("category").map(String::as_str),
            Some("auth")
        );
        assert_no_log_carries_the_sentinel(&logs);
    }

    #[test]
    fn a_wake_received_during_backoff_ends_it_immediately_instead_of_waiting_out_the_delay() {
        // 退避表第一档是 1s（`RETRY_BACKOFF_SECONDS[0]`）。预先把唤醒塞进容量 1 的 channel，
        // 模拟"用户在退避开始前就已经按下重建"：`drain_pending_batches` 必须立刻消费掉它、
        // 回到循环顶部重试，而不是把它晾在缓冲区里直到退避走完（Fix 4 的核心断言）。
        let logging = CapturingLogPort::default();
        let (wake_tx, wakeups) = sync_channel::<()>(1);
        wake_tx.try_send(()).expect("capacity-1 buffer has room");
        let worker = RetrievalIndexingWorker {
            indexing: IndexingService::new(
                Arc::new(OnceThenEmptyRepository::default()),
                Arc::new(EmptySource),
                Arc::new(FailingEmbedder),
            ),
            configuration: Arc::new(FakeConfigurationRepository::Configured),
            wakeups,
            inter_batch_interval: Duration::ZERO,
            code: None,
        };

        let started = Instant::now();
        drain_pending_batches(&worker, MODEL, &logging);
        let elapsed = started.elapsed();

        assert!(
            elapsed < Duration::from_millis(500),
            "a buffered wake should end the backoff immediately, took {elapsed:?}"
        );
    }

    #[test]
    fn a_disconnected_wake_channel_still_waits_out_the_full_backoff() {
        // 发送端没了时 `recv_timeout` 会立刻返回 `Disconnected` 而不是等到超时——如果不特殊
        // 处理，退避形同虚设，会在同一批失败文档上原地打转，把 provider 的速率限制撞穿
        // （见 `drain_pending_batches` 顶部注释）。门面被丢弃后 worker 只剩这条退避路径，必须
        // 仍然真的睡够一档，行为与 `start_retrieval_indexing_worker` 对同一种情况的处理一致。
        let logging = CapturingLogPort::default();
        let (wake_tx, wakeups) = sync_channel::<()>(1);
        drop(wake_tx);
        let worker = RetrievalIndexingWorker {
            indexing: IndexingService::new(
                Arc::new(OnceThenEmptyRepository::default()),
                Arc::new(EmptySource),
                Arc::new(FailingEmbedder),
            ),
            configuration: Arc::new(FakeConfigurationRepository::Configured),
            wakeups,
            inter_batch_interval: Duration::ZERO,
            code: None,
        };

        let started = Instant::now();
        drain_pending_batches(&worker, MODEL, &logging);
        let elapsed = started.elapsed();

        assert!(
            elapsed >= Duration::from_secs(1),
            "a disconnected wake channel must still wait out the backoff, took {elapsed:?}"
        );
    }

    #[test]
    fn deferred_agent_retrieval_reports_unconfigured_and_fails_search_before_binding() {
        // `assemble_agent_runtime_api` runs before `assemble_retrieval` (`runtime.rs`'s `setup`),
        // so `RuntimeAgentApiAdapter` can observe this cell before `bind` is ever called — it must
        // behave exactly like "not configured", never panic or block.
        let deferred = DeferredAgentRetrieval::default();

        assert!(!deferred.is_configured());
        assert!(deferred.search("npm", 5).is_err());
    }

    #[test]
    fn project_search_outcome_keeps_only_the_fields_the_model_should_see() {
        // `source_id`/`score` are internal to `retrieval` and must not cross into
        // `AgentRetrievalHit` — proven here by construction, since that type has no such fields.
        let outcome = SearchOutcome {
            hits: vec![ScoredHit {
                source_id: "internal-id".to_string(),
                content: "uses npm not pnpm".to_string(),
                created_at: "2026-01-01T00:00:00Z".to_string(),
                score: 0.87,
                matched_via: MatchedVia::Both,
            }],
            degraded: Some(Degradation::KeywordOnly),
        };

        let projected = project_search_outcome(outcome);

        assert_eq!(projected.hits.len(), 1);
        assert_eq!(projected.hits[0].content, "uses npm not pnpm");
        assert_eq!(projected.hits[0].created_at, "2026-01-01T00:00:00Z");
        assert_eq!(projected.hits[0].matched_via, "both");
        assert_eq!(projected.degraded.as_deref(), Some("keyword_only"));
    }
}
