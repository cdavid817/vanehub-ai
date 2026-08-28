//! Where a permission diagnostic goes when the audit trail cannot take it.
//!
//! One line, through unified logging, with counts and tokens only. It exists for the case the audit
//! trail cannot cover: the thing that usually fails during evaluation is storage, and the audit
//! trail is storage. Without this, a database outage would produce a burst of approval prompts and
//! no record anywhere saying why.

use crate::contexts::permissions::application::PermissionsDiagnosticsPort;
use crate::contexts::permissions::domain::Action;
use crate::platform::logging::{fallback_log_dir, write_message_raw, LogLevel};
use std::collections::BTreeMap;

pub(crate) struct UnifiedLogDiagnosticsAdapter;

impl PermissionsDiagnosticsPort for UnifiedLogDiagnosticsAdapter {
    fn evaluation_failed_closed(
        &self,
        action: &Action,
        reason: &'static str,
        session_id: &str,
        generation_id: &str,
    ) {
        // A write failure is swallowed on purpose. This is already the fallback path; propagating
        // would mean an evaluation that failed closed *and* could not say so now also fails to
        // return, which is strictly worse for the caller waiting on a decision.
        let _ = write_message_raw(
            &fallback_log_dir(),
            LogLevel::Error,
            "permissions",
            "permission evaluation failed closed",
            BTreeMap::from([
                // `action` is a fixed vocabulary term, not user content. The resource is a path and
                // is deliberately absent, as is the underlying error, which can quote a statement.
                ("action".to_string(), action.as_str().to_string()),
                ("reason".to_string(), reason.to_string()),
                ("session_id".to_string(), session_id.to_string()),
                ("generation_id".to_string(), generation_id.to_string()),
            ]),
        );
    }
}
