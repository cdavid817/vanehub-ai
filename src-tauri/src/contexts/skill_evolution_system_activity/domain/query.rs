use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use serde::{Deserialize, Serialize};

use super::*;

pub(crate) const MAX_ACTIVITY_PAGE_SIZE: u16 = 100;
const MAX_QUERY_VALUES: usize = 16;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ActivityCuratorState {
    Queued,
    Approved,
    Rejected,
    Deferred,
}

impl ActivityCuratorState {
    pub(crate) const fn event_code(self) -> ActivityEventCode {
        match self {
            Self::Queued => ActivityEventCode::CuratorQueued,
            Self::Approved => ActivityEventCode::CuratorApproved,
            Self::Rejected => ActivityEventCode::CuratorRejected,
            Self::Deferred => ActivityEventCode::CuratorDeferred,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct ActivitySafeSearch {
    pub(crate) event_alias_codes: Vec<ActivityEventCode>,
    pub(crate) identity_tokens: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ActivityTimelineQuery {
    pub(crate) session_id: String,
    pub(crate) committed_from_ms: Option<i64>,
    pub(crate) committed_to_ms: Option<i64>,
    pub(crate) severities: Vec<ActivitySeverity>,
    pub(crate) source_domains: Vec<EvolutionSourceDomain>,
    pub(crate) statuses: Vec<ActivityStatus>,
    pub(crate) skill_id: Option<String>,
    pub(crate) run_id: Option<String>,
    pub(crate) curator_states: Vec<ActivityCuratorState>,
    pub(crate) attention_kinds: Vec<ActivityAttentionKind>,
    pub(crate) search: Option<ActivitySafeSearch>,
    pub(crate) cursor: Option<String>,
    pub(crate) page_size: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ActivityTimelineEntry {
    pub(crate) sequence: u64,
    pub(crate) envelope: EvolutionActivityEnvelopeV1,
    pub(crate) detail_unavailable_reason: Option<ActivityReasonCode>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ActivityTimelinePage {
    pub(crate) active_generation_id: String,
    pub(crate) entries: Vec<ActivityTimelineEntry>,
    pub(crate) next_cursor: Option<String>,
    pub(crate) complete: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ActivityTimelineQueryResult {
    Page(ActivityTimelinePage),
    StaleGeneration {
        requested_generation_id: String,
        active_generation_id: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ActivityPageCursorV1 {
    version: u8,
    generation_id: String,
    before_sequence: u64,
}

impl ActivityTimelineQuery {
    pub(crate) fn validate(&self) -> Result<(), ActivityEnvelopeError> {
        sanitize_text(&self.session_id, "query.session_id", 160)?;
        if self.page_size == 0 || self.page_size > MAX_ACTIVITY_PAGE_SIZE {
            return Err(ActivityEnvelopeError::InvalidField("query.page_size"));
        }
        if self
            .committed_from_ms
            .zip(self.committed_to_ms)
            .is_some_and(|(from, to)| from > to)
            || [
                self.severities.len(),
                self.source_domains.len(),
                self.statuses.len(),
                self.curator_states.len(),
                self.attention_kinds.len(),
            ]
            .into_iter()
            .any(|size| size > MAX_QUERY_VALUES)
        {
            return Err(ActivityEnvelopeError::InvalidField("query.filters"));
        }
        for value in [self.skill_id.as_deref(), self.run_id.as_deref()]
            .into_iter()
            .flatten()
        {
            normalize_safe_identity_token(value)?;
        }
        if let Some(search) = &self.search {
            if search.event_alias_codes.len() > MAX_QUERY_VALUES
                || search.identity_tokens.len() > MAX_QUERY_VALUES
            {
                return Err(ActivityEnvelopeError::InvalidField("query.search"));
            }
            for token in &search.identity_tokens {
                normalize_safe_identity_token(token)?;
            }
        }
        Ok(())
    }
}

pub(crate) fn normalize_safe_identity_token(value: &str) -> Result<String, ActivityEnvelopeError> {
    let normalized = sanitize_text(value, "query.identity_token", 160)?;
    Ok(normalized.to_lowercase())
}

pub(crate) fn encode_activity_page_cursor(
    generation_id: &str,
    before_sequence: u64,
) -> Result<String, ActivityEnvelopeError> {
    let bytes = serde_json::to_vec(&ActivityPageCursorV1 {
        version: 1,
        generation_id: generation_id.to_owned(),
        before_sequence,
    })
    .map_err(|_| ActivityEnvelopeError::Serialization)?;
    Ok(URL_SAFE_NO_PAD.encode(bytes))
}

pub(crate) fn decode_activity_page_cursor(
    value: &str,
) -> Result<(String, u64), ActivityEnvelopeError> {
    if value.is_empty() || value.len() > 512 {
        return Err(ActivityEnvelopeError::InvalidField("query.cursor"));
    }
    let bytes = URL_SAFE_NO_PAD
        .decode(value)
        .map_err(|_| ActivityEnvelopeError::InvalidField("query.cursor"))?;
    let cursor: ActivityPageCursorV1 = serde_json::from_slice(&bytes)
        .map_err(|_| ActivityEnvelopeError::InvalidField("query.cursor"))?;
    if cursor.version != 1 || cursor.before_sequence == 0 {
        return Err(ActivityEnvelopeError::InvalidField("query.cursor"));
    }
    sanitize_text(&cursor.generation_id, "query.cursor_generation", 160)?;
    Ok((cursor.generation_id, cursor.before_sequence))
}
