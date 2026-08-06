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
        // 退避期间也监听唤醒信号：`rebuild()`/`save_configuration()` 全靠 `notify()` 才能不等一整个
        // 轮询周期就生效，但如果这里只会 `thread::sleep`，用户在退避中途修好配置、按下"重建"，
        // 最多要等 300s 才会看到反应。收到唤醒就立刻结束退避、回到循环顶部重试；发送端没了则
        // 退化成真正的睡眠——和 `start_retrieval_indexing_worker` 对同一种情况的处理保持一致
        // （同一个理由：`recv_timeout` 在发送端消失后会立刻返回，不特殊处理就会把退避变成忙等）。
        let delay = retry_backoff(consecutive_failures);
        match worker.wakeups.recv_timeout(delay) {
            Ok(()) | Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => thread::sleep(delay),
        }
        consecutive_failures += 1;
    }
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
        // 读配置失败与"没配置"是两回事：前者是瞬时的（连接池 checkout 超时），归为可重试，
        // 让退避在下一轮重来；后者是确定性的，失败即停。混为一谈会让一次池超时把整批
        // 32 条记忆永久打成 failed，而 reconcile 因内容哈希未变会保留 failed，只能靠人工重建。
        let configuration = self.configuration.load().map_err(|_| EmbeddingFailure {
            category: FailureCategory::Network,
            message: "the retrieval configuration could not be read".to_string(),
        })?;
        let profile_id = configuration
            .resolved_model()
            .map(|(profile, _model)| profile.to_string())
            .ok_or_else(|| EmbeddingFailure {
                category: FailureCategory::InvalidRequest,
                message: "no embedding profile is configured".to_string(),
            })?;
        HttpEmbeddingAdapter::new(self.endpoint.clone(), profile_id).embed(model, inputs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contexts::operations::api::OperationsError;
    use crate::contexts::retrieval::application::{RetrievalConfiguration, RetrievalIndexStatus};
    use crate::contexts::retrieval::domain::{
        IndexState, RetrievalDocument, RetrievalScope, SourceKind,
    };
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
    }

    struct EmptySource;

    impl IndexSourcePort for EmptySource {
        fn snapshot(&self) -> Result<Vec<IndexSourceRecord>, RetrievalError> {
            Ok(Vec::new())
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
            _scope: &RetrievalScope,
            _source_kind: SourceKind,
            _model: &str,
        ) -> Result<Vec<(String, Vec<f32>)>, RetrievalError> {
            unimplemented!("not exercised by these tests")
        }
        fn keyword_candidates(
            &self,
            _scope: &RetrievalScope,
            _source_kind: SourceKind,
            _query: &str,
            _limit: usize,
        ) -> Result<Vec<String>, RetrievalError> {
            unimplemented!("not exercised by these tests")
        }
        fn index_status(&self, _agent_id: &str) -> Result<RetrievalIndexStatus, RetrievalError> {
            unimplemented!("not exercised by these tests")
        }
        fn requeue_all(&self, _agent_id: &str) -> Result<(), RetrievalError> {
            unimplemented!("not exercised by these tests")
        }
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
        };

        let started = Instant::now();
        drain_pending_batches(&worker, MODEL, &logging);
        let elapsed = started.elapsed();

        assert!(
            elapsed >= Duration::from_secs(1),
            "a disconnected wake channel must still wait out the backoff, took {elapsed:?}"
        );
    }
}
