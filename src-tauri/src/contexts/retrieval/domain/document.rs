use sha2::{Digest, Sha256};

/// 索引来源类别。第 1 期只有 `AgentMemory`；第 2/3 期扩展 `session_message`、`workspace_file`。
/// 字符串形式是持久化格式，改动即破坏既有索引行。
// Task 5 的文档仓储会构造并匹配 SourceKind；届时移除本属性。
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SourceKind {
    AgentMemory,
}

// 同上，随 SourceKind 一起在 Task 5 移除。
#[allow(dead_code)]
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

// Task 5 的文档仓储会读写 index_state；届时移除本属性。
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum IndexState {
    Pending,
    Indexed,
    Failed,
}

// 同上，随 IndexState 一起在 Task 5 移除。
#[allow(dead_code)]
impl IndexState {
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
// Task 5 的仓储 trait 签名会用上这个类型（记录失败类别）；届时移除本属性。
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FailureCategory {
    Auth,
    InvalidRequest,
    RateLimit,
    Network,
}

// 同上；is_retryable 要到 Task 8 的重试逻辑才被调用，届时一并移除。
#[allow(dead_code)]
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

// Task 5 的仓储 trait 签名会用上这个类型；届时移除本属性。
#[allow(dead_code)]
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
// Task 5 的仓储会调用它生成主键；届时移除本属性。
#[allow(dead_code)]
pub(crate) fn document_id(source_kind: SourceKind, source_id: &str) -> String {
    format!("{}:{}", source_kind.as_str(), source_id)
}

// Task 5 的仓储会在写入前调用它计算内容哈希；届时移除本属性。
#[allow(dead_code)]
pub(crate) fn content_hash(content: &str) -> String {
    bytes_to_hex(&Sha256::digest(content.as_bytes()))
}

// 仅被 content_hash 调用；content_hash 在 Task 5 变为可达后本函数一并可达，届时移除本属性。
#[allow(dead_code)]
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
        assert!(first.chars().all(|c| c.is_ascii_hexdigit() && !c.is_uppercase()));
    }

    #[test]
    fn source_kind_round_trips_through_its_persisted_form() {
        assert_eq!(SourceKind::AgentMemory.as_str(), "agent_memory");
        assert_eq!(SourceKind::parse("agent_memory"), Some(SourceKind::AgentMemory));
        assert_eq!(SourceKind::parse("nonsense"), None);
    }

    #[test]
    fn index_state_round_trips_through_its_persisted_form() {
        for state in [IndexState::Pending, IndexState::Indexed, IndexState::Failed] {
            assert_eq!(IndexState::parse(state.as_str()), Some(state));
        }
        assert_eq!(IndexState::parse("nonsense"), None);
    }
}
