//! 检索上下文的装配根与后台索引 worker。
//!
//! `retrieval` 与 `agent_runtime` 的**唯一**交汇点在本文件：前者需要一个 embedding 端点和一份
//! 记忆快照，后者拥有 Profile、凭据与记忆表。两个上下文因此都不 import 对方，跨界只发生在
//! 组合根里（设计文档 §4.3）。
//!
//! 应用服务刻意不打日志、只返回结构化结果，日志格式因此只在本文件定义一处（设计文档 §8.2）。
//! **绝不落盘**：记忆内容、query 原文、凭据、provider 响应体——下面每条日志的字段都只有计数、
//! 耗时、模型 id 与错误类别。

use crate::contexts::agent_runtime::api::AgentRuntimeApi;
use crate::contexts::agent_runtime::infrastructure::SqliteAgentMemoryRepository;
use crate::contexts::operations::api::{DiagnosticLog, DiagnosticLogPort, LogSeverity};
use crate::contexts::operations::infrastructure::UnifiedLoggingAdapter;
use crate::contexts::retrieval::api::{RetrievalApi, RetrievalWorkerSignal};
use crate::contexts::retrieval::application::{
    BatchOutcome, EmbeddingEndpointPort, EmbeddingFailure, EmbeddingPort, IndexSourcePort,
    IndexSourceRecord, IndexingService, ResolvedEmbeddingEndpoint,
    RetrievalConfigurationRepository, RetrievalDocumentRepository, SearchService,
    RECONCILE_POLL_INTERVAL_SECONDS, RETRY_BACKOFF_SECONDS,
};
use crate::contexts::retrieval::domain::{FailureCategory, RetrievalError};
use crate::contexts::retrieval::infrastructure::{
    HttpEmbeddingAdapter, SqliteRetrievalConfigurationRepository, SqliteRetrievalDocumentRepository,
};
use crate::platform::database::NativeDatabase;
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::mpsc::{Receiver, RecvTimeoutError};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

const POLL_INTERVAL: Duration = Duration::from_secs(RECONCILE_POLL_INTERVAL_SECONDS);
const RECONCILE_CATEGORY: &str = "retrieval.indexing.reconcile";
const BATCH_CATEGORY: &str = "retrieval.indexing.batch";

pub(crate) struct RetrievalAssembly {
    pub(crate) api: RetrievalApi,
    pub(crate) worker: RetrievalIndexingWorker,
}

/// worker 需要的全部状态。与 `RetrievalApi` 分开返回，好让组合根按既有惯例先 `manage` 门面、
/// 再和其他 background job 一起启动线程。
pub(crate) struct RetrievalIndexingWorker {
    indexing: IndexingService,
    configuration: Arc<dyn RetrievalConfigurationRepository>,
    wakeups: Receiver<()>,
}

pub(crate) fn assemble_retrieval(
    database: NativeDatabase,
    agent_runtime: AgentRuntimeApi,
) -> RetrievalAssembly {
    let documents: Arc<dyn RetrievalDocumentRepository> =
        Arc::new(SqliteRetrievalDocumentRepository::new(database.clone()));
    let configuration: Arc<dyn RetrievalConfigurationRepository> = Arc::new(
        SqliteRetrievalConfigurationRepository::new(database.clone()),
    );
    let source: Arc<dyn IndexSourcePort> = Arc::new(AgentMemoryIndexSource {
        memories: SqliteAgentMemoryRepository::new(database),
    });
    let endpoint: Arc<dyn EmbeddingEndpointPort> =
        Arc::new(AgentRuntimeEmbeddingEndpoint { agent_runtime });
    let embeddings: Arc<dyn EmbeddingPort> = Arc::new(ConfiguredProfileEmbeddingAdapter {
        configuration: configuration.clone(),
        endpoint,
    });
    let (signal, wakeups) = RetrievalWorkerSignal::channel();
    RetrievalAssembly {
        api: RetrievalApi::new(
            Arc::new(SearchService::new(
                configuration.clone(),
                documents.clone(),
                source.clone(),
                embeddings.clone(),
            )),
            documents.clone(),
            configuration.clone(),
            signal,
        ),
        worker: RetrievalIndexingWorker {
            indexing: IndexingService::new(documents, source, embeddings),
            configuration,
            wakeups,
        },
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
        loop {
            run_indexing_cycle(&worker, &logging);
            match worker.wakeups.recv_timeout(POLL_INTERVAL) {
                Ok(()) | Err(RecvTimeoutError::Timeout) => {}
                // 发送端全没了（门面被丢弃）。`recv_timeout` 此后会立刻返回，不自己睡一觉
                // 就会把兜底轮询变成忙等。
                Err(RecvTimeoutError::Disconnected) => thread::sleep(POLL_INTERVAL),
            }
        }
    });
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
    let Some(model) = configured_model(worker.configuration.as_ref()) else {
        return;
    };
    drain_pending_batches(worker, &model, logging);
}

/// 串行处理，不并发冲击速率限制（设计文档 §5.2）。
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
            ],
        );
        thread::sleep(retry_backoff(consecutive_failures));
        consecutive_failures += 1;
    }
}

fn retry_backoff(consecutive_failures: usize) -> Duration {
    let step = consecutive_failures.min(RETRY_BACKOFF_SECONDS.len() - 1);
    Duration::from_secs(RETRY_BACKOFF_SECONDS[step])
}

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
/// 设计文档 §8.2 只允许落盘错误**类别**。
fn error_category(error: &RetrievalError) -> &'static str {
    match error {
        RetrievalError::Storage(_) => "storage",
        RetrievalError::Embedding(_) => "embedding",
        RetrievalError::NotConfigured => "not_configured",
        RetrievalError::Unavailable => "unavailable",
    }
}

/// `retrieval` 的索引源。它只知道"给我全部源记录"，不知道 `agent_memories` 表和
/// `AgentMemory` 类型的存在——那些细节到本文件为止。
struct AgentMemoryIndexSource {
    memories: SqliteAgentMemoryRepository,
}

impl IndexSourcePort for AgentMemoryIndexSource {
    fn snapshot(&self) -> Result<Vec<IndexSourceRecord>, RetrievalError> {
        let memories = self
            .memories
            .list_all()
            .map_err(|error| RetrievalError::Storage(error.to_string()))?;
        Ok(memories
            .into_iter()
            .map(|memory| IndexSourceRecord {
                source_id: memory.id,
                agent_id: memory.agent_id,
                // 无工作区文件夹用空串哨兵，与 `agent_memories.folder` 的列约定一致；检索侧的
                // scope 也这样映射，两侧不一致就永远搜不到。
                folder: memory.folder.unwrap_or_default(),
                content: memory.content,
                created_at: memory.created_at,
            })
            .collect())
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
        let profile_id = self
            .configuration
            .load()
            .ok()
            .and_then(|configuration| {
                configuration
                    .resolved_model()
                    .map(|(profile, _model)| profile.to_string())
            })
            .ok_or_else(|| EmbeddingFailure {
                // 读不到配置是确定性失败，重试只会烧配额（`document.rs` 的重试哲学）。
                category: FailureCategory::InvalidRequest,
                message: "no embedding profile is configured".to_string(),
            })?;
        HttpEmbeddingAdapter::new(self.endpoint.clone(), profile_id).embed(model, inputs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn logged_error_categories_never_carry_the_error_payload() {
        // 哨兵：把敏感文本塞进错误载荷，证明落盘的只有类别。
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
}
