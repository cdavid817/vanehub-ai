use sha2::{Digest, Sha256};

/// 索引来源类别。第 1 期只有 `AgentMemory`；第 2/3 期扩展 `session_message`、`workspace_file`。
/// 字符串形式是持久化格式，改动即破坏既有索引行。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SourceKind {
    AgentMemory,
}

impl SourceKind {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::AgentMemory => "agent_memory",
        }
    }

    pub(crate) fn parse(value: &str) -> Option<Self> {
        match value {
            "agent_memory" => Some(Self::AgentMemory),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum IndexState {
    Pending,
    Indexed,
    Failed,
}

impl IndexState {
    // 检索仓储写 index_state 时一律绑定 SQL 字符串字面量（见 sqlite_repository.rs 的
    // upsert_pending/store_embedding/record_failure/requeue_all），从不把 Rust 端的 IndexState
    // 序列化回字符串——所以生产代码里没有 as_str 的调用方，它只用于本文件的往返测试
    // （断言 parse(as_str(x)) == x）。没有计划中的任务会改变这一点，故不写"届时移除"。
    #[allow(dead_code)]
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Indexed => "indexed",
            Self::Failed => "failed",
        }
    }

    pub(crate) fn parse(value: &str) -> Option<Self> {
        match value {
            "pending" => Some(Self::Pending),
            "indexed" => Some(Self::Indexed),
            "failed" => Some(Self::Failed),
            _ => None,
        }
    }
}

/// 决定重试策略：`Auth`/`InvalidRequest` 是确定性失败，重试只会烧配额（设计文档 §5.2）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FailureCategory {
    Auth,
    InvalidRequest,
    RateLimit,
    Network,
}

impl FailureCategory {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Auth => "auth",
            Self::InvalidRequest => "invalid_request",
            Self::RateLimit => "rate_limit",
            Self::Network => "network",
        }
    }

    pub(crate) fn is_retryable(self) -> bool {
        matches!(self, Self::RateLimit | Self::Network)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct RetrievalDocument {
    pub(crate) id: String,
    pub(crate) source_kind: SourceKind,
    pub(crate) source_id: String,
    pub(crate) scope_agent_id: String,
    /// 空串哨兵表示"无工作区文件夹"，与 `agent_memories.folder` 一致（`memory_schema.rs:4-6`）。
    pub(crate) scope_folder: String,
    pub(crate) content: String,
    pub(crate) content_hash: String,
    pub(crate) index_state: IndexState,
    pub(crate) attempt_count: u32,
    pub(crate) embedding_model: Option<String>,
}

/// 确定性主键，与 `UNIQUE (source_kind, source_id)` 同源——reconcile 因此可以直接 upsert，
/// 不必先查后插。
pub(crate) fn document_id(source_kind: SourceKind, source_id: &str) -> String {
    format!("{}:{}", source_kind.as_str(), source_id)
}

pub(crate) fn content_hash(content: &str) -> String {
    bytes_to_hex(&Sha256::digest(content.as_bytes()))
}

fn bytes_to_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(HEX[(byte >> 4) as usize] as char);
        output.push(HEX[(byte & 0x0f) as usize] as char);
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn document_id_is_deterministic_and_namespaced_by_source_kind() {
        assert_eq!(
            document_id(SourceKind::AgentMemory, "mem-1"),
            "agent_memory:mem-1"
        );
        assert_eq!(
            document_id(SourceKind::AgentMemory, "mem-1"),
            document_id(SourceKind::AgentMemory, "mem-1")
        );
    }

    #[test]
    fn content_hash_is_stable_and_distinguishes_content() {
        let first = content_hash("Uses npm, not pnpm.");
        assert_eq!(first, content_hash("Uses npm, not pnpm."));
        assert_ne!(first, content_hash("Uses pnpm."));
        assert_eq!(first.len(), 64);
        assert!(first
            .chars()
            .all(|c| c.is_ascii_hexdigit() && !c.is_uppercase()));
    }

    #[test]
    fn source_kind_round_trips_through_its_persisted_form() {
        assert_eq!(SourceKind::AgentMemory.as_str(), "agent_memory");
        assert_eq!(
            SourceKind::parse("agent_memory"),
            Some(SourceKind::AgentMemory)
        );
        assert_eq!(SourceKind::parse("nonsense"), None);
    }

    #[test]
    fn index_state_round_trips_through_its_persisted_form() {
        // 先钉死字面量再验往返：只验往返的话，`as_str` 与 `parse` 被对称改成别的字符串
        // 仍然会通过，而那等于悄悄改掉了已落盘数据的格式。
        assert_eq!(IndexState::Pending.as_str(), "pending");
        assert_eq!(IndexState::Indexed.as_str(), "indexed");
        assert_eq!(IndexState::Failed.as_str(), "failed");
        for state in [IndexState::Pending, IndexState::Indexed, IndexState::Failed] {
            assert_eq!(IndexState::parse(state.as_str()), Some(state));
        }
        assert_eq!(IndexState::parse("nonsense"), None);
    }
}
