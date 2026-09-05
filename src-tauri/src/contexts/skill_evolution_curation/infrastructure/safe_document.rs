use crate::contexts::skill_evolution_curation::domain::{
    CuratorCandidateSnapshot, CuratorDraftKind,
};
use serde_json::Value;
use sha2::{Digest, Sha256};
use thiserror::Error;

const MAX_SNAPSHOT_BYTES: usize = 16 * 1024;
const MAX_LEARN_BLOCK_BYTES: usize = 8 * 1024;
const MAX_EXACT_PATCH_BYTES: usize = 16 * 1024;
// Matched as substrings of the normalized key, not exact names: an exact-match list lets
// `userPrompt`, `apiKeys`, or `toolArgs` sail past and land raw prompts or secrets on disk.
// Fail closed — a rejected draft surfaces `UnsafeShape` to its author instead of persisting.
const PROHIBITED_KEY_STEMS: &[&str] = &[
    "prompt",
    "providerpayload",
    "terminaloutput",
    "credential",
    "secret",
    "password",
    "apikey",
    "accesskey",
    "authtoken",
    "modelresponse",
    "toolargument",
    "toolarg",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ValidatedDraftDocument {
    body_json: String,
    body_hash: String,
    scanner_version: String,
}

impl ValidatedDraftDocument {
    pub(crate) fn from_validated_value(
        kind: CuratorDraftKind,
        value: &Value,
        scanner_version: &str,
    ) -> Result<Self, SafeDocumentError> {
        if scanner_version.trim().is_empty() || contains_prohibited_key(value) {
            return Err(SafeDocumentError::UnsafeShape);
        }
        let body_json = serde_json::to_string(value).map_err(|_| SafeDocumentError::Encoding)?;
        let limit = match kind {
            CuratorDraftKind::LearnBlock => MAX_LEARN_BLOCK_BYTES,
            CuratorDraftKind::ExactPatch => MAX_EXACT_PATCH_BYTES,
        };
        if body_json.is_empty() || body_json.len() > limit {
            return Err(SafeDocumentError::SizeLimit);
        }
        Ok(Self {
            body_hash: sha256(body_json.as_bytes()),
            body_json,
            scanner_version: scanner_version.to_owned(),
        })
    }

    pub(super) fn body_json(&self) -> &str {
        &self.body_json
    }

    pub(super) fn body_hash(&self) -> &str {
        &self.body_hash
    }

    pub(super) fn scanner_version(&self) -> &str {
        &self.scanner_version
    }
}

pub(super) fn safe_snapshot_json(
    snapshot: &CuratorCandidateSnapshot,
) -> Result<String, SafeDocumentError> {
    let json = serde_json::to_string(snapshot).map_err(|_| SafeDocumentError::Encoding)?;
    if json.len() > MAX_SNAPSHOT_BYTES {
        return Err(SafeDocumentError::SizeLimit);
    }
    Ok(json)
}

fn contains_prohibited_key(value: &Value) -> bool {
    match value {
        Value::Object(object) => object.iter().any(|(key, value)| {
            let normalized = key.to_ascii_lowercase().replace(['_', '-'], "");
            PROHIBITED_KEY_STEMS
                .iter()
                .any(|stem| normalized.contains(stem))
                || contains_prohibited_key(value)
        }),
        Value::Array(values) => values.iter().any(contains_prohibited_key),
        _ => false,
    }
}

fn sha256(bytes: &[u8]) -> String {
    crate::platform::hashing::sha256_tagged(bytes)
}

#[derive(Debug, Error, PartialEq, Eq)]
pub(crate) enum SafeDocumentError {
    #[error("curator document encoding failed")]
    Encoding,
    #[error("curator document exceeds its persistence limit")]
    SizeLimit,
    #[error("curator document contains a prohibited persistence shape")]
    UnsafeShape,
}
