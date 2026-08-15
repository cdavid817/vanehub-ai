use rusqlite::{params, OptionalExtension};

use super::super::application::{LinkProgress, LinkProgressProbe};
use super::super::domain::GoalLinkTarget;
use crate::platform::database::NativeDatabase;

/// Every probe funnels through `Option`: a missing row and a failed query both
/// collapse to `None`, and `None` becomes `Unresolvable`. One broken lookup
/// therefore degrades a single child instead of failing the whole goal read.
macro_rules! probe_impl {
    ($name:ident, $kind:expr, $resolve:ident) => {
        impl LinkProgressProbe for $name {
            fn kind(&self) -> GoalLinkTarget {
                $kind
            }

            fn probe(&self, target_id: &str) -> LinkProgress {
                self.$resolve(target_id)
                    .unwrap_or(LinkProgress::Unresolvable)
            }
        }
    };
}

pub(crate) struct PlanProgressProbe {
    database: NativeDatabase,
}

impl PlanProgressProbe {
    pub(crate) fn new(database: NativeDatabase) -> Self {
        Self { database }
    }

    fn resolve(&self, plan_id: &str) -> Option<LinkProgress> {
        let connection = self.database.connection().ok()?;
        let plan_status: String = connection
            .query_row(
                "SELECT status FROM plans WHERE id = ?1",
                params![plan_id],
                |row| row.get(0),
            )
            .optional()
            .ok()??;

        if plan_status == "archived" {
            return Some(LinkProgress::Terminal);
        }

        let run_status: Option<String> = connection
            .query_row(
                "SELECT status FROM plan_runs WHERE plan_id = ?1
                 ORDER BY created_at DESC, id DESC LIMIT 1",
                params![plan_id],
                |row| row.get(0),
            )
            .optional()
            .ok()?;

        Some(match run_status.as_deref() {
            // A failed plan run is NOT terminal: the plan run state machine
            // allows failed -> running, so the work can still be retried.
            // `awaiting_acceptance` is likewise still active — that run is
            // waiting on a human of its own.
            Some("completed" | "cancelled") => LinkProgress::Terminal,
            _ => LinkProgress::Active,
        })
    }
}

probe_impl!(PlanProgressProbe, GoalLinkTarget::Plan, resolve);

pub(crate) struct LoopProgressProbe {
    database: NativeDatabase,
}

impl LoopProgressProbe {
    pub(crate) fn new(database: NativeDatabase) -> Self {
        Self { database }
    }

    fn resolve(&self, definition_id: &str) -> Option<LinkProgress> {
        let connection = self.database.connection().ok()?;
        let _: i64 = connection
            .query_row(
                "SELECT 1 FROM loop_definitions WHERE id = ?1",
                params![definition_id],
                |row| row.get(0),
            )
            .optional()
            .ok()??;

        let run_status: Option<String> = connection
            .query_row(
                "SELECT status FROM loop_runs WHERE definition_id = ?1
                 ORDER BY created_at DESC, id DESC LIMIT 1",
                params![definition_id],
                |row| row.get(0),
            )
            .optional()
            .ok()?;

        Some(match run_status.as_deref() {
            // Unlike plans, a failed loop run is terminal — the loop state
            // machine has no edge out of failed. Keep the two lists apart.
            Some("succeeded" | "failed" | "cancelled") => LinkProgress::Terminal,
            _ => LinkProgress::Active,
        })
    }
}

probe_impl!(LoopProgressProbe, GoalLinkTarget::Loop, resolve);

pub(crate) struct WorkItemProgressProbe {
    database: NativeDatabase,
}

impl WorkItemProgressProbe {
    pub(crate) fn new(database: NativeDatabase) -> Self {
        Self { database }
    }

    fn resolve(&self, work_item_id: &str) -> Option<LinkProgress> {
        let connection = self.database.connection().ok()?;
        let (stage, archived): (String, i64) = connection
            .query_row(
                "SELECT stage, archived FROM work_items WHERE id = ?1",
                params![work_item_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()
            .ok()??;

        Some(if archived != 0 || stage == "done" {
            LinkProgress::Terminal
        } else {
            LinkProgress::Active
        })
    }
}

probe_impl!(WorkItemProgressProbe, GoalLinkTarget::WorkItem, resolve);

/// Sessions are shown, never counted. This probe exists only so a deleted
/// session is marked unresolvable in the UI instead of looking healthy.
pub(crate) struct SessionProgressProbe {
    database: NativeDatabase,
}

impl SessionProgressProbe {
    pub(crate) fn new(database: NativeDatabase) -> Self {
        Self { database }
    }

    fn resolve(&self, session_id: &str) -> Option<LinkProgress> {
        let connection = self.database.connection().ok()?;
        let _: i64 = connection
            .query_row(
                "SELECT 1 FROM sessions WHERE id = ?1",
                params![session_id],
                |row| row.get(0),
            )
            .optional()
            .ok()??;
        Some(LinkProgress::Active)
    }
}

probe_impl!(SessionProgressProbe, GoalLinkTarget::Session, resolve);
