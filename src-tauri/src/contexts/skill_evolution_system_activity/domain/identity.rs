use super::{ActivityKind, ActivityScopeKind};
use sha2::{Digest, Sha256};
use thiserror::Error;

const GLOBAL_SCOPE_ID: &str = "global";
const ID_NAMESPACE_V1: &str = "vanehub:system-activity-session:v1";
const GENERATION_NAMESPACE_V1: &str = "vanehub:system-activity-generation:v1";
const ITEM_NAMESPACE_V1: &str = "vanehub:system-activity-item:v1";

#[derive(Debug, Error, PartialEq, Eq)]
pub(crate) enum SystemActivityIdentityError {
    #[error("workspace system activity requires a canonical scope id")]
    MissingCanonicalWorkspace,
    #[error("global system activity must use the reserved global scope")]
    InvalidGlobalScope,
}

pub(crate) fn stable_system_activity_session_id(
    activity_kind: ActivityKind,
    scope_kind: ActivityScopeKind,
    canonical_scope_id: &str,
) -> Result<String, SystemActivityIdentityError> {
    let scope = match scope_kind {
        ActivityScopeKind::Global if canonical_scope_id == GLOBAL_SCOPE_ID => GLOBAL_SCOPE_ID,
        ActivityScopeKind::Global => return Err(SystemActivityIdentityError::InvalidGlobalScope),
        ActivityScopeKind::Workspace if canonical_scope_id.trim().is_empty() => {
            return Err(SystemActivityIdentityError::MissingCanonicalWorkspace);
        }
        ActivityScopeKind::Workspace => canonical_scope_id,
    };
    let input = format!("{ID_NAMESPACE_V1}|{activity_kind:?}|{scope_kind:?}|{scope}");
    Ok(format!("system-activity-v1-{}", sha256_hex(&input)))
}

pub(crate) fn stable_activity_generation_id(session_id: &str, projection_version: u16) -> String {
    let input = format!("{GENERATION_NAMESPACE_V1}|{session_id}|{projection_version}");
    format!("activity-generation-v1-{}", sha256_hex(&input))
}

pub(crate) fn stable_activity_item_id(
    session_id: &str,
    generation_id: &str,
    event_id: &str,
) -> String {
    let input = format!("{ITEM_NAMESPACE_V1}|{session_id}|{generation_id}|{event_id}");
    format!("activity-item-v1-{}", sha256_hex(&input))
}

fn sha256_hex(input: &str) -> String {
    Sha256::digest(input.as_bytes())
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}
