use crate::contexts::retrieval::domain::{
    CodeChunk, CodeEmbeddingConfirmation, CodeFileManifest, CodeIndexAuditEntry,
    CodeIndexAuditEvent, CodeIndexAuditReason, CodeIndexAutomaticMode, CodeIndexPhase,
    CodeIndexStatus, CodeSearchCandidate, CodeSearchOutcome, CodeSearchQuery, CodeSymbol,
    CodeWorkspace, FailureCategory, RetrievalDocument, RetrievalError, RetrievalScope, SourceKind,
};
use std::path::Path;
use std::time::Duration;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct RetrievalIndexStatus {
    pub(crate) indexed: u32,
    pub(crate) pending: u32,
    pub(crate) failed: u32,
    /// 只给类别，不带原始错误文本——错误体可能含凭据或 provider 响应内容（设计文档 §8.2）。
    pub(crate) last_failure_category: Option<String>,
}

pub(crate) trait RetrievalDocumentRepository: Send + Sync {
    fn upsert_pending(&self, document: &RetrievalDocument) -> Result<(), RetrievalError>;

    fn upsert_pending_scoped(
        &self,
        document: &RetrievalDocument,
        scope: &RetrievalScope,
    ) -> Result<(), RetrievalError> {
        scope.validate_for(document.source_kind)?;
        if scope
            .workspace_id()
            .is_some_and(|workspace| workspace != document.scope_folder)
        {
            return Err(RetrievalError::InvalidScope);
        }
        self.upsert_pending(document)
    }
    fn list_indexed_source_ids(
        &self,
        source_kind: SourceKind,
    ) -> Result<Vec<(String, String)>, RetrievalError>;
    fn delete_by_source(
        &self,
        source_kind: SourceKind,
        source_id: &str,
    ) -> Result<(), RetrievalError>;
    fn claim_pending_batch(
        &self,
        source_kind: SourceKind,
        limit: usize,
    ) -> Result<Vec<RetrievalDocument>, RetrievalError>;
    fn store_embedding(
        &self,
        id: &str,
        model: &str,
        embedding: &[f32],
    ) -> Result<(), RetrievalError>;
    fn record_failure(
        &self,
        id: &str,
        category: FailureCategory,
        give_up: bool,
    ) -> Result<(), RetrievalError>;
    /// 覆盖整个共享池，不按 agent/folder 过滤：`agent-memory-shared-pool` 之后每条记忆对每个
    /// Agent 都可见，过滤只会让 `recall` 搜不到已经注入进系统提示词的记忆。
    fn vector_candidates(
        &self,
        source_kind: SourceKind,
        model: &str,
    ) -> Result<Vec<(String, Vec<f32>)>, RetrievalError>;
    /// 与 `vector_candidates` 同样覆盖整个共享池。
    fn keyword_candidates(
        &self,
        source_kind: SourceKind,
        query: &str,
        limit: usize,
    ) -> Result<Vec<String>, RetrievalError>;
    /// 全量聚合，不按 agent 分组：检索配置本身是全局单例，`is_configured()` 是全局的，索引源
    /// 快照也覆盖全部 agent，所以"这套索引现在什么状态"只有一个答案。按 agent 过滤会让
    /// 非 OnePiece agent 的行既不出现在状态里、也无法被重建。
    fn index_status(&self) -> Result<RetrievalIndexStatus, RetrievalError>;
    fn requeue_all(&self) -> Result<(), RetrievalError>;
    /// 把 embedding 模型与 `new_model` 不一致的已索引行打回 `pending`。
    ///
    /// 换模型后 `vector_candidates` 会按 `embedding_model = ?` 把旧模型的行全部滤掉，而
    /// reconcile 只在内容哈希变化时重新入队——换模型不改内容，所以没有这个方法，那些行会
    /// 永远停在 `indexed` 却永远进不了向量召回，每次检索静默降级成关键词单路，状态页还显示
    /// 一切正常。
    fn requeue_stale_model(&self, new_model: &str) -> Result<(), RetrievalError>;

    fn list_indexed_source_ids_scoped(
        &self,
        source_kind: SourceKind,
        scope: &RetrievalScope,
    ) -> Result<Vec<(String, String)>, RetrievalError> {
        require_legacy_global_scope(source_kind, scope)?;
        self.list_indexed_source_ids(source_kind)
    }

    fn delete_by_source_scoped(
        &self,
        source_kind: SourceKind,
        scope: &RetrievalScope,
        source_id: &str,
    ) -> Result<(), RetrievalError> {
        require_legacy_global_scope(source_kind, scope)?;
        self.delete_by_source(source_kind, source_id)
    }

    fn claim_pending_batch_scoped(
        &self,
        source_kind: SourceKind,
        scope: &RetrievalScope,
        limit: usize,
    ) -> Result<Vec<RetrievalDocument>, RetrievalError> {
        require_legacy_global_scope(source_kind, scope)?;
        self.claim_pending_batch(source_kind, limit)
    }

    fn vector_candidates_scoped(
        &self,
        source_kind: SourceKind,
        scope: &RetrievalScope,
        model: &str,
    ) -> Result<Vec<(String, Vec<f32>)>, RetrievalError> {
        require_legacy_global_scope(source_kind, scope)?;
        self.vector_candidates(source_kind, model)
    }

    fn keyword_candidates_scoped(
        &self,
        source_kind: SourceKind,
        scope: &RetrievalScope,
        query: &str,
        limit: usize,
    ) -> Result<Vec<String>, RetrievalError> {
        require_legacy_global_scope(source_kind, scope)?;
        self.keyword_candidates(source_kind, query, limit)
    }

    fn index_status_scoped(
        &self,
        source_kind: SourceKind,
        scope: &RetrievalScope,
    ) -> Result<RetrievalIndexStatus, RetrievalError> {
        require_legacy_global_scope(source_kind, scope)?;
        self.index_status()
    }

    fn requeue_all_scoped(
        &self,
        source_kind: SourceKind,
        scope: &RetrievalScope,
    ) -> Result<(), RetrievalError> {
        require_legacy_global_scope(source_kind, scope)?;
        self.requeue_all()
    }

    fn requeue_stale_model_scoped(
        &self,
        source_kind: SourceKind,
        scope: &RetrievalScope,
        new_model: &str,
    ) -> Result<(), RetrievalError> {
        require_legacy_global_scope(source_kind, scope)?;
        self.requeue_stale_model(new_model)
    }
}

pub(crate) trait CodeIndexRepository: Send + Sync {
    fn register_workspace(
        &self,
        root: &Path,
        display_name: &str,
    ) -> Result<CodeWorkspace, RetrievalError>;
    fn ensure_automatic_workspace(
        &self,
        root: &Path,
        display_name: &str,
        mode: crate::contexts::retrieval::domain::CodeIndexMode,
    ) -> Result<(CodeWorkspace, bool), RetrievalError>;
    fn list_workspaces(&self) -> Result<Vec<CodeWorkspace>, RetrievalError>;
    fn load_workspace(&self, workspace_id: &str) -> Result<Option<CodeWorkspace>, RetrievalError>;
    fn save_workspace_configuration(
        &self,
        workspace_id: &str,
        update: crate::contexts::retrieval::domain::CodeIndexConfigurationUpdate,
    ) -> Result<CodeWorkspace, RetrievalError>;
    fn rebuild_workspace(&self, workspace_id: &str) -> Result<CodeWorkspace, RetrievalError>;
    fn delete_workspace(&self, workspace_id: &str) -> Result<(), RetrievalError>;
    fn workspace_generation(&self, workspace_id: &str) -> Result<Option<u64>, RetrievalError>;
    fn set_workspace_phase(
        &self,
        workspace_id: &str,
        phase: CodeIndexPhase,
    ) -> Result<(), RetrievalError>;
    fn workspace_status(&self, workspace_id: &str) -> Result<CodeIndexStatus, RetrievalError>;
    fn embedding_confirmation(
        &self,
        workspace_id: &str,
    ) -> Result<Option<CodeEmbeddingConfirmation>, RetrievalError>;
    fn confirm_embedding(
        &self,
        workspace_id: &str,
        profile_id: &str,
        model: &str,
        generation: u64,
    ) -> Result<CodeEmbeddingConfirmation, RetrievalError>;
    fn record_audit(
        &self,
        workspace_id: &str,
        relative_path: Option<&str>,
        event: CodeIndexAuditEvent,
        reason: Option<CodeIndexAuditReason>,
        item_count: u64,
    ) -> Result<(), RetrievalError>;
    fn list_audit(
        &self,
        workspace_id: &str,
        limit: usize,
    ) -> Result<Vec<CodeIndexAuditEntry>, RetrievalError>;
    fn load_code_candidates(
        &self,
        workspace_id: &str,
        source_ids: &[String],
    ) -> Result<Vec<CodeSearchCandidate>, RetrievalError>;
    fn list_file_manifests(
        &self,
        workspace_id: &str,
    ) -> Result<Vec<CodeFileManifest>, RetrievalError>;
    fn replace_code_file(
        &self,
        manifest: &CodeFileManifest,
        chunks: &[CodeChunk],
        symbols: &[CodeSymbol],
    ) -> Result<(), RetrievalError>;
    fn update_file_fingerprint(&self, manifest: &CodeFileManifest) -> Result<(), RetrievalError>;
    fn delete_code_file(
        &self,
        workspace_id: &str,
        relative_path: &str,
    ) -> Result<(), RetrievalError>;
}

pub(crate) trait CodeRetrievalPort: Send + Sync {
    fn search_code(&self, query: &CodeSearchQuery) -> Result<CodeSearchOutcome, RetrievalError>;
}

fn require_legacy_global_scope(
    source_kind: SourceKind,
    scope: &RetrievalScope,
) -> Result<(), RetrievalError> {
    scope.validate_for(source_kind)?;
    matches!(scope, RetrievalScope::GlobalMemory)
        .then_some(())
        .ok_or(RetrievalError::InvalidScope)
}

/// 算向量的消费侧契约。唯一实现是 infrastructure 的 `HttpEmbeddingAdapter`。
pub(crate) trait EmbeddingPort: Send + Sync {
    fn embed(&self, model: &str, inputs: &[String]) -> Result<Vec<Vec<f32>>, EmbeddingFailure>;
}

/// `HttpEmbeddingAdapter` 把 provider 的 HTTP 状态码映射成它；`process_pending_batch` 读
/// `category` 判定是否放弃重试。
#[derive(Debug)]
pub(crate) struct EmbeddingFailure {
    pub(crate) category: FailureCategory,
    pub(crate) retry_after: Option<Duration>,
    /// 非测试代码里没有读者，也**不该**有：后台 worker 的失败日志按设计文档 §8.2 只落盘错误
    /// 类别，而这段文本来自与 provider 的交互，是最可能夹带响应体片段的一处。字段保留是因为
    /// 它属于既定的接口形状，且 openai_embedding_adapter.rs 的哨兵测试正是拿它做断言对象。
    /// 没有计划中的任务会给它添加非测试读者，故不写"届时移除"。
    #[allow(dead_code)]
    pub(crate) message: String,
}

#[derive(Debug, Clone)]
pub(crate) struct ResolvedEmbeddingEndpoint {
    pub(crate) base_url: String,
    pub(crate) credential: String,
}

/// 消费侧契约：retrieval 只声明"给我一个可用的 embedding 端点"，不知道 Profile、凭据存储、
/// provider 目录的存在（设计文档 §4.3）。唯一实现写在 bootstrap 的装配根里，封装
/// `agent_runtime::api::resolve_embedding_endpoint`。
pub(crate) trait EmbeddingEndpointPort: Send + Sync {
    fn resolve(&self, profile_id: &str) -> Result<ResolvedEmbeddingEndpoint, RetrievalError>;
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct RetrievalConfiguration {
    pub(crate) source_profile_id: Option<String>,
    pub(crate) embedding_model: Option<String>,
    pub(crate) automatic_code_index_mode: CodeIndexAutomaticMode,
}

impl RetrievalConfiguration {
    /// 两者齐备才算"已配置"——缺任一个都无法发起一次 embedding 调用。
    pub(crate) fn resolved_model(&self) -> Option<(&str, &str)> {
        let profile = self.source_profile_id.as_deref()?;
        let model = self.embedding_model.as_deref()?;
        (!profile.is_empty() && !model.is_empty()).then_some((profile, model))
    }
}

pub(crate) trait RetrievalConfigurationRepository: Send + Sync {
    fn load(&self) -> Result<RetrievalConfiguration, RetrievalError>;
    fn save(&self, profile_id: &str, embedding_model: &str) -> Result<(), RetrievalError>;
    fn save_automatic_code_index_mode(
        &self,
        mode: CodeIndexAutomaticMode,
    ) -> Result<(), RetrievalError>;
}
