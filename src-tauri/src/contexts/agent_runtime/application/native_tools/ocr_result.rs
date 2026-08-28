//! The OnePiece OCR tool's result contract.
//!
//! Moved here from the extensions context when PaddleOCR moved to `local_media`. The struct stayed
//! byte-identical on the wire -- `contractVersion` is still 1 -- because the tool's output is what
//! an Agent reads, and a shared runtime is an implementation change, not a contract change.

use serde::Serialize;
use std::collections::BTreeSet;

pub(crate) const OCR_RESULT_CONTRACT_VERSION: u16 = 1;

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct OcrResultPoint {
    pub(crate) x: f32,
    pub(crate) y: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct OcrResultBlock {
    pub(crate) page_number: u32,
    pub(crate) order: u32,
    pub(crate) text: String,
    pub(crate) polygon: Option<Vec<OcrResultPoint>>,
    pub(crate) confidence: Option<f32>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct NormalizedOcrResult {
    pub(crate) contract_version: u16,
    pub(crate) operation_id: String,
    pub(crate) source_artifact_id: String,
    pub(crate) source_content_hash: String,
    pub(crate) engine_name: String,
    pub(crate) engine_version: String,
    pub(crate) languages: Vec<String>,
    pub(crate) pages: Vec<u32>,
    pub(crate) blocks: Vec<OcrResultBlock>,
    pub(crate) text: String,
    pub(crate) truncated: bool,
    pub(crate) duration_ms: u64,
    pub(crate) warnings: Vec<String>,
}

/// Identity fields the caller supplies; everything else is derived from the recognized blocks.
pub(crate) struct OcrResultIdentity<'a> {
    pub(crate) operation_id: &'a str,
    pub(crate) artifact_id: &'a str,
    pub(crate) content_hash: &'a str,
    pub(crate) engine_name: &'a str,
    pub(crate) engine_version: &'a str,
    pub(crate) languages: Vec<String>,
    pub(crate) duration_ms: u64,
    pub(crate) truncated: bool,
    pub(crate) warnings: Vec<String>,
}

/// Order the blocks, derive the page list and plain text, and refuse a malformed identity.
///
/// Sorting rather than trusting arrival order matters because the two entry points reach here by
/// different routes; duplicate `(page, order)` pairs are rejected outright, since a result whose
/// blocks cannot be totally ordered has no deterministic plain-text projection.
pub(crate) fn normalize_ocr_result(
    identity: OcrResultIdentity<'_>,
    mut blocks: Vec<OcrResultBlock>,
) -> Option<NormalizedOcrResult> {
    if identity.operation_id.is_empty()
        || identity.artifact_id.is_empty()
        || identity.content_hash.len() != 64
        || identity.engine_name.is_empty()
        || identity.engine_version.is_empty()
    {
        return None;
    }
    blocks.sort_by_key(|block| (block.page_number, block.order));
    if blocks
        .windows(2)
        .any(|pair| (pair[0].page_number, pair[0].order) == (pair[1].page_number, pair[1].order))
    {
        return None;
    }
    let pages = blocks
        .iter()
        .map(|block| block.page_number)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    let text = blocks
        .iter()
        .map(|block| block.text.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    Some(NormalizedOcrResult {
        contract_version: OCR_RESULT_CONTRACT_VERSION,
        operation_id: identity.operation_id.to_owned(),
        source_artifact_id: identity.artifact_id.to_owned(),
        source_content_hash: identity.content_hash.to_owned(),
        engine_name: identity.engine_name.to_owned(),
        engine_version: identity.engine_version.to_owned(),
        languages: identity.languages,
        pages,
        blocks,
        text,
        truncated: identity.truncated,
        duration_ms: identity.duration_ms,
        warnings: identity.warnings,
    })
}

#[cfg(test)]
#[path = "ocr_result_tests.rs"]
mod tests;
