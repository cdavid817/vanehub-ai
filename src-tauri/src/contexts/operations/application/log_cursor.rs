//! The opaque cursor a log page hands back.
//!
//! Keyset, never offset. An offset cursor names a position in a result set, and a result set that
//! grew since the first page has moved every position in it — so page two either repeats rows or
//! skips them, and both look exactly like ordinary pagination. A keyset cursor names the last row
//! instead, which does not move when rows are added above it.
//!
//! The filter fingerprint travels with it for the same reason: continuing a cursor under different
//! filters would splice two result sets together, and the boundary between them would be invisible.

use super::log_index::{OperationsLogError, SessionLogFilters, SessionLogQueryScope};

/// Bumped when the encoding changes. A cursor from an older build is refused rather than
/// misread — the bytes would still decode into *something*, and that something would be a
/// position in a set nobody asked for.
const CURSOR_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LogPageCursor {
    pub(crate) occurred_at_ms: i64,
    pub(crate) sequence: i64,
    pub(crate) record_id: String,
    pub(crate) filter_fingerprint: String,
}

impl LogPageCursor {
    /// Encodes to an opaque string.
    ///
    /// Opaque to the caller, not encrypted: it carries no secret, and hiding its shape is what
    /// stops a client from constructing one by hand and inventing a position.
    pub(crate) fn encode(&self) -> String {
        let payload = format!(
            "{CURSOR_VERSION}|{}|{}|{}|{}",
            self.occurred_at_ms, self.sequence, self.filter_fingerprint, self.record_id
        );
        BASE64.encode(payload.as_bytes())
    }

    pub(crate) fn decode(raw: &str, expected: &str) -> Result<Self, OperationsLogError> {
        let decoded = BASE64
            .decode(raw)
            .and_then(|bytes| String::from_utf8(bytes).ok())
            .ok_or(OperationsLogError::InvalidCursor)?;
        let mut parts = decoded.splitn(5, '|');
        let version: u32 = parts
            .next()
            .and_then(|value| value.parse().ok())
            .ok_or(OperationsLogError::InvalidCursor)?;
        if version != CURSOR_VERSION {
            return Err(OperationsLogError::InvalidCursor);
        }
        let occurred_at_ms: i64 = parts
            .next()
            .and_then(|value| value.parse().ok())
            .ok_or(OperationsLogError::InvalidCursor)?;
        let sequence: i64 = parts
            .next()
            .and_then(|value| value.parse().ok())
            .ok_or(OperationsLogError::InvalidCursor)?;
        let fingerprint = parts
            .next()
            .ok_or(OperationsLogError::InvalidCursor)?
            .to_string();
        let record_id = parts
            .next()
            .ok_or(OperationsLogError::InvalidCursor)?
            .to_string();
        // A malformed cursor and a cursor for other filters are different failures, and a caller
        // acts differently on each: one is a bug or a stale client, the other is "your filters
        // changed, start again".
        if fingerprint != expected {
            return Err(OperationsLogError::CursorFilterMismatch);
        }
        Ok(Self {
            occurred_at_ms,
            sequence,
            record_id,
            filter_fingerprint: fingerprint,
        })
    }
}

/// A stable fingerprint of everything that decides which rows a query admits.
///
/// Every field that narrows the result participates. One that did not would let a caller change it
/// mid-pagination and keep using the cursor, which is exactly the silent splice this prevents.
pub(crate) fn filter_fingerprint(
    scope: &SessionLogQueryScope,
    filters: &SessionLogFilters,
) -> String {
    let mut levels: Vec<&'static str> = filters.levels.iter().map(|level| level.token()).collect();
    levels.sort_unstable();
    levels.dedup();
    let parts = [
        scope.session_id.as_deref().unwrap_or(""),
        scope.seat_id.as_deref().unwrap_or(""),
        scope.run_id.as_deref().unwrap_or(""),
        scope.trace_id.as_deref().unwrap_or(""),
        scope.span_id.as_deref().unwrap_or(""),
        scope.operation_id.as_deref().unwrap_or(""),
        scope.agent_id.as_deref().unwrap_or(""),
        filters.search.as_deref().unwrap_or(""),
        filters.from.as_deref().unwrap_or(""),
        filters.to.as_deref().unwrap_or(""),
    ];
    // Length-prefixed, so two different field splits cannot produce the same string: a session id
    // of "a" with a run id of "bc" must not fingerprint the same as "ab" with "c".
    let mut payload = String::new();
    for part in parts {
        payload.push_str(&format!("{}:{part};", part.len()));
    }
    payload.push_str(&levels.join(","));
    format!("{:016x}", stable_hash(&payload))
}

fn stable_hash(value: &str) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    value.hash(&mut hasher);
    hasher.finish()
}

/// A tiny URL-safe base64, so the cursor survives JSON and a query string without escaping.
struct Base64;

const BASE64: Base64 = Base64;
const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";

impl Base64 {
    fn encode(&self, bytes: &[u8]) -> String {
        let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
        for chunk in bytes.chunks(3) {
            let b = [
                chunk[0],
                chunk.get(1).copied().unwrap_or(0),
                chunk.get(2).copied().unwrap_or(0),
            ];
            let n = (u32::from(b[0]) << 16) | (u32::from(b[1]) << 8) | u32::from(b[2]);
            for index in 0..4 {
                if index <= chunk.len() {
                    out.push(ALPHABET[((n >> (18 - 6 * index)) & 0x3f) as usize] as char);
                } else {
                    out.push('=');
                }
            }
        }
        out
    }

    fn decode(&self, value: &str) -> Option<Vec<u8>> {
        let mut bits = 0u32;
        let mut count = 0u32;
        let mut out = Vec::new();
        for character in value.bytes() {
            if character == b'=' {
                break;
            }
            let index = ALPHABET.iter().position(|entry| *entry == character)? as u32;
            bits = (bits << 6) | index;
            count += 6;
            if count >= 8 {
                count -= 8;
                out.push(((bits >> count) & 0xff) as u8);
            }
        }
        Some(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contexts::operations::application::IndexedLogLevel;

    fn scope() -> SessionLogQueryScope {
        SessionLogQueryScope {
            session_id: Some("session-1".to_string()),
            ..SessionLogQueryScope::default()
        }
    }

    #[test]
    fn a_cursor_round_trips_through_its_opaque_form() {
        let fingerprint = filter_fingerprint(&scope(), &SessionLogFilters::default());
        let cursor = LogPageCursor {
            occurred_at_ms: 1_700_000_000_123,
            sequence: 42,
            record_id: "record-7".to_string(),
            filter_fingerprint: fingerprint.clone(),
        };

        let decoded = LogPageCursor::decode(&cursor.encode(), &fingerprint).expect("decode");

        assert_eq!(decoded, cursor);
    }

    /// A malformed cursor must never be read as a position. Falling back to "start from zero" or
    /// to an offset would silently answer a different question than the one asked.
    #[test]
    fn a_malformed_cursor_is_refused_rather_than_reinterpreted() {
        let fingerprint = filter_fingerprint(&scope(), &SessionLogFilters::default());
        for raw in ["", "not-base64!!", "MTIz"] {
            assert_eq!(
                LogPageCursor::decode(raw, &fingerprint),
                Err(OperationsLogError::InvalidCursor)
            );
        }
    }

    #[test]
    fn a_cursor_issued_for_other_filters_is_a_distinct_failure() {
        let original = filter_fingerprint(&scope(), &SessionLogFilters::default());
        let cursor = LogPageCursor {
            occurred_at_ms: 1,
            sequence: 1,
            record_id: "record-1".to_string(),
            filter_fingerprint: original,
        }
        .encode();
        let narrowed = filter_fingerprint(
            &scope(),
            &SessionLogFilters {
                levels: vec![IndexedLogLevel::Error],
                ..SessionLogFilters::default()
            },
        );

        assert_eq!(
            LogPageCursor::decode(&cursor, &narrowed),
            Err(OperationsLogError::CursorFilterMismatch)
        );
    }

    /// Two different splits of the same characters must not fingerprint alike, or a cursor from
    /// one scope would be accepted by another.
    #[test]
    fn fingerprints_separate_fields_that_concatenate_the_same_way() {
        let left = filter_fingerprint(
            &SessionLogQueryScope {
                session_id: Some("a".to_string()),
                run_id: Some("bc".to_string()),
                ..SessionLogQueryScope::default()
            },
            &SessionLogFilters::default(),
        );
        let right = filter_fingerprint(
            &SessionLogQueryScope {
                session_id: Some("ab".to_string()),
                run_id: Some("c".to_string()),
                ..SessionLogQueryScope::default()
            },
            &SessionLogFilters::default(),
        );

        assert_ne!(left, right);
    }

    #[test]
    fn level_order_does_not_change_a_fingerprint() {
        let ascending = filter_fingerprint(
            &scope(),
            &SessionLogFilters {
                levels: vec![IndexedLogLevel::Error, IndexedLogLevel::Warn],
                ..SessionLogFilters::default()
            },
        );
        let descending = filter_fingerprint(
            &scope(),
            &SessionLogFilters {
                levels: vec![IndexedLogLevel::Warn, IndexedLogLevel::Error],
                ..SessionLogFilters::default()
            },
        );

        assert_eq!(ascending, descending);
    }
}
