use chrono::{DateTime, Utc};

use super::memory::{
    MemoryAudience, MemoryId, MemoryRecord, MemoryScope, MemorySource, MemoryStatus, MemoryType,
};
use super::scope::{AgentId, WorkspaceKey};

/// What a list page returns when the caller does not ask for a size.
pub(crate) const MEMORY_PAGE_DEFAULT_SIZE: usize = 50;
/// The ceiling a caller cannot exceed. A UI query is always bounded; complete enumeration is a
/// separate, explicitly named maintenance operation rather than "ask for a very large page".
pub(crate) const MEMORY_PAGE_MAX_SIZE: usize = 200;

/// A row in a list page.
///
/// Deliberately has no `content` field. Rendering a list must never require reading every body,
/// and a type that cannot carry a body cannot regress into doing so.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MemorySummary {
    pub(crate) id: MemoryId,
    pub(crate) name: String,
    pub(crate) description: String,
    pub(crate) memory_type: MemoryType,
    pub(crate) scope_kind: &'static str,
    pub(crate) workspace_key: Option<WorkspaceKey>,
    pub(crate) audience_is_restricted: bool,
    pub(crate) status: MemoryStatus,
    pub(crate) source: MemorySource,
    pub(crate) source_agent_id: Option<AgentId>,
    pub(crate) revision: u64,
    pub(crate) updated_at: DateTime<Utc>,
}

impl MemorySummary {
    pub(crate) fn from_record(record: &MemoryRecord) -> Self {
        Self {
            id: record.id.clone(),
            name: record.name.clone(),
            description: record.description.clone(),
            memory_type: record.memory_type,
            scope_kind: record.scope.kind_str(),
            workspace_key: record.scope.workspace_key().cloned(),
            audience_is_restricted: matches!(
                record.audience,
                MemoryAudience::SelectedAgents { .. }
            ),
            status: record.status,
            source: record.source,
            source_agent_id: record.provenance.source_agent_id.clone(),
            revision: record.revision,
            updated_at: record.updated_at,
        }
    }
}

/// Which scopes a query covers.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) enum MemoryScopeFilter {
    #[default]
    Any,
    GlobalOnly,
    Workspace {
        workspace_key: WorkspaceKey,
    },
}

impl MemoryScopeFilter {
    pub(crate) fn matches(&self, scope: &MemoryScope) -> bool {
        match (self, scope) {
            (Self::Any, _) => true,
            (Self::GlobalOnly, MemoryScope::Global) => true,
            (Self::GlobalOnly, MemoryScope::Workspace { .. }) => false,
            (
                Self::Workspace { workspace_key },
                MemoryScope::Workspace {
                    workspace_key: other,
                },
            ) => workspace_key == other,
            (Self::Workspace { .. }, MemoryScope::Global) => false,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) enum MemoryOrder {
    /// Newest first. The default because staleness is the property users scan for.
    #[default]
    UpdatedDescending,
    UpdatedAscending,
    NameAscending,
}

/// Keyset position rather than an offset: an offset page shifts under concurrent writes and
/// silently skips or repeats rows while the user is paging through them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MemoryCursor {
    pub(crate) updated_at: DateTime<Utc>,
    pub(crate) id: MemoryId,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct MemoryQuery {
    pub(crate) search: Option<String>,
    pub(crate) scope: MemoryScopeFilter,
    /// Empty means "any status". A candidate is only ever returned when a caller asks for it.
    pub(crate) statuses: Vec<MemoryStatus>,
    pub(crate) memory_types: Vec<MemoryType>,
    pub(crate) source_agent_id: Option<AgentId>,
    pub(crate) audience_agent_id: Option<AgentId>,
    pub(crate) order: MemoryOrder,
    pub(crate) cursor: Option<MemoryCursor>,
    page_size: usize,
}

impl MemoryQuery {
    /// Clamps rather than rejects: an over-large page size is a caller bug that must not become a
    /// way to pull every memory body through the list endpoint, but it is not worth failing a
    /// user's query over.
    pub(crate) fn with_page_size(mut self, requested: usize) -> Self {
        self.page_size = requested.clamp(1, MEMORY_PAGE_MAX_SIZE);
        self
    }

    pub(crate) fn page_size(&self) -> usize {
        if self.page_size == 0 {
            MEMORY_PAGE_DEFAULT_SIZE
        } else {
            self.page_size
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MemoryPage {
    pub(crate) items: Vec<MemorySummary>,
    pub(crate) next_cursor: Option<MemoryCursor>,
    /// Present only when the store can produce it cheaply; the UI must render without it.
    pub(crate) total_matched: Option<usize>,
}

impl MemoryPage {
    pub(crate) fn empty() -> Self {
        Self {
            items: Vec::new(),
            next_cursor: None,
            total_matched: Some(0),
        }
    }
}
