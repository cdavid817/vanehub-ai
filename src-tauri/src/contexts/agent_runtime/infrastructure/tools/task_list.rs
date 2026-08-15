//! The session-scoped Agent task list backing the `todo_write` tool (`add-agent-task-list`).
//!
//! The list is deliberately *not* in the message history. Context compaction summarizes earlier
//! turns, and an enumerated checklist is the first thing a summary flattens -- so the list lives
//! in runtime state and is projected into the system prompt instead, where it is always current
//! and survives compaction untouched.
//!
//! It is equally deliberately not the unified Todo Board. That board holds user-owned records
//! with a user-controlled stage; this is a scratchpad the model rewrites as often as it
//! reconsiders. Letting a tool loop write the board would churn records the user curates.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

/// Well past any real task decomposition, small enough that the projected prompt section stays a
/// bounded cost on every generation.
pub(crate) const MAX_TASK_ITEMS: usize = 40;

/// A task title, not a paragraph. Rationale belongs in the reply, not in a checklist row.
pub(crate) const MAX_TASK_CONTENT_CHARS: usize = 200;

pub(crate) const STATUS_PENDING: &str = "pending";
pub(crate) const STATUS_IN_PROGRESS: &str = "in_progress";
pub(crate) const STATUS_COMPLETED: &str = "completed";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TaskStatus {
    Pending,
    InProgress,
    Completed,
}

impl TaskStatus {
    pub(crate) fn parse(value: &str) -> Option<Self> {
        match value {
            STATUS_PENDING => Some(Self::Pending),
            STATUS_IN_PROGRESS => Some(Self::InProgress),
            STATUS_COMPLETED => Some(Self::Completed),
            _ => None,
        }
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::Pending => STATUS_PENDING,
            Self::InProgress => STATUS_IN_PROGRESS,
            Self::Completed => STATUS_COMPLETED,
        }
    }

    /// The marker used in the projected system-prompt section. Chosen so the three states are
    /// distinguishable at a glance without a legend.
    const fn marker(self) -> &'static str {
        match self {
            Self::Pending => "[ ]",
            Self::InProgress => "[~]",
            Self::Completed => "[x]",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TaskItem {
    pub(crate) content: String,
    pub(crate) status: TaskStatus,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum TaskListError {
    TooManyItems { submitted: usize },
    EmptyContent { index: usize },
    ContentTooLong { index: usize, characters: usize },
    UnknownStatus { index: usize, status: String },
    MultipleInProgress { count: usize },
}

impl TaskListError {
    /// Every message names what to change. A rejection the model cannot act on just becomes a
    /// retry of the same invalid list.
    pub(crate) fn message(&self) -> String {
        match self {
            Self::TooManyItems { submitted } => format!(
                "A task list may contain at most {MAX_TASK_ITEMS} items, but {submitted} were submitted. Merge or drop items instead of splitting the list across calls."
            ),
            Self::EmptyContent { index } => {
                format!("Task {} has empty content.", index + 1)
            }
            Self::ContentTooLong { index, characters } => format!(
                "Task {} is {characters} characters; the maximum is {MAX_TASK_CONTENT_CHARS}. Keep each task a short title.",
                index + 1
            ),
            Self::UnknownStatus { index, status } => format!(
                "Task {} has unrecognized status \"{status}\". Use \"{STATUS_PENDING}\", \"{STATUS_IN_PROGRESS}\", or \"{STATUS_COMPLETED}\".",
                index + 1
            ),
            Self::MultipleInProgress { count } => format!(
                "{count} tasks are marked {STATUS_IN_PROGRESS}, but only one task may be in progress at a time. Mark the others {STATUS_PENDING}."
            ),
        }
    }
}

/// Validates a submitted list without touching stored state, so a rejected write provably leaves
/// the previous list alone.
pub(crate) fn validate(items: &[(String, String)]) -> Result<Vec<TaskItem>, TaskListError> {
    if items.len() > MAX_TASK_ITEMS {
        return Err(TaskListError::TooManyItems {
            submitted: items.len(),
        });
    }
    let mut validated = Vec::with_capacity(items.len());
    for (index, (content, status)) in items.iter().enumerate() {
        let trimmed = content.trim();
        if trimmed.is_empty() {
            return Err(TaskListError::EmptyContent { index });
        }
        let characters = trimmed.chars().count();
        if characters > MAX_TASK_CONTENT_CHARS {
            return Err(TaskListError::ContentTooLong { index, characters });
        }
        let Some(status) = TaskStatus::parse(status) else {
            return Err(TaskListError::UnknownStatus {
                index,
                status: status.clone(),
            });
        };
        validated.push(TaskItem {
            content: trimmed.to_owned(),
            status,
        });
    }
    let in_progress = validated
        .iter()
        .filter(|item| item.status == TaskStatus::InProgress)
        .count();
    if in_progress > 1 {
        // Rejected rather than normalized: silently demoting the extras would teach the model
        // that the field does not mean anything.
        return Err(TaskListError::MultipleInProgress { count: in_progress });
    }
    Ok(validated)
}

/// Renders a list for a tool result or the projected prompt section. Shared so the model reads
/// the same shape in both places and cannot mistake one for a different list.
pub(crate) fn render(items: &[TaskItem]) -> String {
    items
        .iter()
        .map(|item| format!("{} {}", item.status.marker(), item.content))
        .collect::<Vec<_>>()
        .join("\n")
}

#[derive(Debug, Default)]
pub(crate) struct TaskListStore {
    lists: Mutex<HashMap<String, Vec<TaskItem>>>,
}

pub(crate) fn store() -> &'static TaskListStore {
    static STORE: OnceLock<TaskListStore> = OnceLock::new();
    STORE.get_or_init(TaskListStore::default)
}

impl TaskListStore {
    /// Replaces the session's list wholesale and returns what is now stored.
    pub(crate) fn replace(&self, session_id: &str, items: Vec<TaskItem>) -> Vec<TaskItem> {
        let Ok(mut lists) = self.lists.lock() else {
            return items;
        };
        if items.is_empty() {
            lists.remove(session_id);
            return Vec::new();
        }
        lists.insert(session_id.to_owned(), items.clone());
        items
    }

    pub(crate) fn get(&self, session_id: &str) -> Vec<TaskItem> {
        self.lists
            .lock()
            .map(|lists| lists.get(session_id).cloned().unwrap_or_default())
            .unwrap_or_default()
    }

    pub(crate) fn clear_session(&self, session_id: &str) {
        if let Ok(mut lists) = self.lists.lock() {
            lists.remove(session_id);
        }
    }
}

/// The bounded system-prompt section, or `None` when the session has no tasks. `None` rather than
/// an empty heading: a section that says "no tasks" would spend prompt budget asserting nothing.
pub(crate) fn prompt_section(session_id: &str) -> Option<String> {
    let items = store().get(session_id);
    (!items.is_empty()).then(|| {
        format!(
            "## Task list\nYour current task list for this session. Keep it current with todo_write; it is not visible to you anywhere else.\n\n{}",
            render(&items)
        )
    })
}

#[cfg(test)]
#[path = "task_list_tests.rs"]
mod tests;
