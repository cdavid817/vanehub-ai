use crate::contexts::execution_observability::application::evidence::models::{
    EvidenceQueryScope, ExecutionRecordFilters, ExecutionRecordKind, ExecutionRecordQuery,
};
use crate::contexts::execution_observability::application::EvidenceApplicationError;
use crate::contexts::execution_observability::domain::{
    fidelity_token, status_token, EvidenceSeatId, EvidenceSessionId,
};
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use sha2::{Digest, Sha256};

const CURSOR_VERSION: u8 = 1;

/// A keyset position, bound to the query that produced it.
///
/// Opaque and versioned. The frontend passes it back unread — decoding it there would recreate the
/// offset arithmetic this design exists to remove — and the backend refuses one that was issued
/// for different filters instead of interpreting it as an offset into a differently-shaped result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct RecordCursor {
    pub(super) occurred_at: String,
    pub(super) record_id: String,
    pub(super) filter_fingerprint: String,
}

impl RecordCursor {
    pub(super) fn encode(&self) -> String {
        let raw = format!(
            "{CURSOR_VERSION}\u{1f}{}\u{1f}{}\u{1f}{}",
            self.occurred_at, self.record_id, self.filter_fingerprint
        );
        URL_SAFE_NO_PAD.encode(raw.as_bytes())
    }

    /// Decodes and checks the fingerprint in one step, so there is no window in which a caller
    /// holds a decoded position it has not yet verified belongs to its query.
    pub(super) fn decode(
        encoded: &str,
        expected_fingerprint: &str,
    ) -> Result<Self, EvidenceApplicationError> {
        let bytes = URL_SAFE_NO_PAD
            .decode(encoded.as_bytes())
            .map_err(|_| EvidenceApplicationError::InvalidCursor)?;
        let raw = String::from_utf8(bytes).map_err(|_| EvidenceApplicationError::InvalidCursor)?;
        let parts = raw.split('\u{1f}').collect::<Vec<_>>();
        if parts.len() != 4 {
            return Err(EvidenceApplicationError::InvalidCursor);
        }
        if parts[0] != CURSOR_VERSION.to_string() {
            return Err(EvidenceApplicationError::InvalidCursor);
        }
        let cursor = Self {
            occurred_at: parts[1].to_string(),
            record_id: parts[2].to_string(),
            filter_fingerprint: parts[3].to_string(),
        };
        if cursor.filter_fingerprint != expected_fingerprint {
            return Err(EvidenceApplicationError::CursorFilterMismatch);
        }
        Ok(cursor)
    }
}

/// A canonical digest of everything that changes which rows a query can return.
///
/// Every filter the query supports is included, in a fixed order, with a present/absent marker so
/// an absent field and an empty string cannot collide. Adding a filter to the query without adding
/// it here would let a cursor issued under the old filter set survive into a differently-shaped
/// result — the exact failure `cursor_filter_mismatch` exists to make impossible.
pub(super) fn filter_fingerprint(query: &ExecutionRecordQuery) -> String {
    let mut parts = Vec::new();
    let EvidenceQueryScope {
        session_id,
        seat_id,
        run_id,
        trace_id,
        span_id,
        operation_id,
        command_id,
    } = &query.scope;
    parts.push(optional(
        "session",
        session_id.as_ref().map(EvidenceSessionId::as_str),
    ));
    parts.push(optional(
        "seat",
        seat_id.as_ref().map(EvidenceSeatId::as_str),
    ));
    parts.push(optional("run", run_id.as_deref()));
    parts.push(optional("trace", trace_id.as_deref()));
    parts.push(optional("span", span_id.as_deref()));
    parts.push(optional("operation", operation_id.as_deref()));
    parts.push(optional("command", command_id.as_deref()));

    let ExecutionRecordFilters {
        kinds,
        statuses,
        fidelities,
        search,
    } = &query.filters;
    let mut kind_tokens = kinds
        .iter()
        .map(|kind| ExecutionRecordKind::as_str(*kind).to_string())
        .collect::<Vec<_>>();
    kind_tokens.sort();
    parts.push(format!("kinds={}", kind_tokens.join(",")));

    let mut status_tokens = statuses
        .iter()
        .map(|status| status_token(*status).to_string())
        .collect::<Vec<_>>();
    status_tokens.sort();
    parts.push(format!("statuses={}", status_tokens.join(",")));

    let mut fidelity_tokens = fidelities
        .iter()
        .map(|fidelity| fidelity_token(*fidelity).to_string())
        .collect::<Vec<_>>();
    fidelity_tokens.sort();
    parts.push(format!("fidelities={}", fidelity_tokens.join(",")));

    parts.push(optional(
        "search",
        search
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty()),
    ));

    let mut hasher = Sha256::new();
    hasher.update(parts.join("\u{1e}").as_bytes());
    hasher
        .finalize()
        .iter()
        .take(16)
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

/// `absent` is a distinct token from an empty value, so "no seat filter" and "a seat filter whose
/// value happens to be empty" cannot produce the same fingerprint.
fn optional(key: &str, value: Option<&str>) -> String {
    match value {
        Some(value) => format!("{key}=v:{value}"),
        None => format!("{key}=absent"),
    }
}
