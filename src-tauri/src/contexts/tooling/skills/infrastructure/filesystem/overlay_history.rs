#![cfg_attr(not(test), allow(dead_code))]

use super::overlay_layout::OverlayStorageLayout;
use crate::contexts::tooling::skills::application::{
    OverlayActor, OverlayApplicationError, OverlayHistoryAction, OverlayHistoryEntry,
    OverlayHistoryPage, OverlayHistoryQuery, OverlayHistoryRepository, OverlayIntegrityCode,
    OverlayKey, OverlayPageIntegrity, SkillApplicationError,
};
use crate::contexts::tooling::skills::domain::{OverlayScope, SkillId, DEFAULT_OVERLAY_LIMITS};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

const HISTORY_SCHEMA_VERSION: u32 = 1;
const DEFAULT_PAGE_ENTRIES: usize = 50;
pub(crate) const HISTORY_SEGMENT_BYTES: u64 = DEFAULT_OVERLAY_LIMITS.maximum_history_segment_bytes;

#[derive(Clone)]
pub(crate) struct FilesystemOverlayHistoryRepository {
    home_root: PathBuf,
    segment_limit: u64,
    page_limit: usize,
}

impl FilesystemOverlayHistoryRepository {
    pub(crate) fn new() -> Self {
        Self::with_home_root(super::default_home_root())
    }

    pub(crate) fn with_home_root(home_root: PathBuf) -> Self {
        Self::with_limits(home_root, HISTORY_SEGMENT_BYTES, DEFAULT_PAGE_ENTRIES)
    }

    pub(crate) fn with_limits(home_root: PathBuf, segment_limit: u64, page_limit: usize) -> Self {
        Self {
            home_root,
            segment_limit,
            page_limit: page_limit.max(1),
        }
    }

    pub(crate) fn append_verified(
        &self,
        key: &OverlayKey,
        mut entry: OverlayHistoryEntry,
    ) -> Result<OverlayHistoryEntry, SkillApplicationError> {
        validate_append_event(key, &entry)?;
        let verified = self.verify(key)?;
        entry.prior_event_hash = verified.tail_hash.clone();
        entry.event_hash = event_hash(&entry)?;
        let event_line = record_line(&HistoryRecordWire::Event(Box::new(EventWire::from(&entry))))?;
        let layout = OverlayStorageLayout::resolve(&self.home_root, key).map_err(layout_error)?;
        create_history_root(&layout.history_root)?;

        let (sequence, mut active_bytes, active_has_events) = if verified.segment_count == 0 {
            let header = header_line(1, None)?;
            ensure_segment_capacity(self.segment_limit, 1, header.len(), event_line.len())?;
            let path = segment_path(&layout.history_root, 1);
            write_new_segment(&path, &header)?;
            (1, header, false)
        } else if verified.last_segment_closed {
            let sequence = verified
                .segment_count
                .checked_add(1)
                .ok_or_else(|| history_validation("history-segment-sequence-overflow"))?;
            let header = header_line(sequence, verified.last_segment_hash.as_deref())?;
            ensure_segment_capacity(self.segment_limit, sequence, header.len(), event_line.len())?;
            let path = segment_path(&layout.history_root, sequence);
            write_new_segment(&path, &header)?;
            (sequence, header, false)
        } else {
            let sequence = verified.segment_count;
            let path = segment_path(&layout.history_root, sequence);
            let bytes = fs::read(path).map_err(filesystem_error)?;
            (sequence, bytes, verified.last_segment_event_count > 0)
        };

        if !fits_with_footer(self.segment_limit, sequence, &active_bytes, &event_line)? {
            if !active_has_events {
                return Err(history_limit(
                    self.segment_limit,
                    active_bytes.len() + event_line.len(),
                ));
            }
            let segment_hash = close_segment(
                &segment_path(&layout.history_root, sequence),
                &active_bytes,
                self.segment_limit,
            )?;
            let next_sequence = sequence
                .checked_add(1)
                .ok_or_else(|| history_validation("history-segment-sequence-overflow"))?;
            active_bytes = header_line(next_sequence, Some(&segment_hash))?;
            ensure_segment_capacity(
                self.segment_limit,
                next_sequence,
                active_bytes.len(),
                event_line.len(),
            )?;
            let next_path = segment_path(&layout.history_root, next_sequence);
            write_new_segment(&next_path, &active_bytes)?;
            append_and_sync(&next_path, &event_line)?;
        } else {
            append_and_sync(&segment_path(&layout.history_root, sequence), &event_line)?;
        }
        Ok(entry)
    }

    fn verify(&self, key: &OverlayKey) -> Result<VerifiedHistory, SkillApplicationError> {
        let layout = OverlayStorageLayout::resolve(&self.home_root, key).map_err(layout_error)?;
        let segments = ordered_segments(&layout.history_root)?;
        let mut verified = VerifiedHistory::default();
        let mut expected_segment_hash: Option<String> = None;
        for (index, segment) in segments.iter().enumerate() {
            let expected_sequence = (index as u64) + 1;
            if segment.sequence != expected_sequence {
                return Err(integrity(OverlayIntegrityCode::HistorySegmentMissing));
            }
            let bytes = fs::read(&segment.path).map_err(filesystem_error)?;
            if bytes.is_empty()
                || !bytes.ends_with(b"\n")
                || bytes.len() as u64 > self.segment_limit
            {
                return Err(integrity(OverlayIntegrityCode::HistorySegmentTruncated));
            }
            let is_last = index + 1 == segments.len();
            let result = verify_segment(
                key,
                &bytes,
                expected_sequence,
                expected_segment_hash.as_deref(),
                verified.tail_hash.as_deref(),
            )?;
            if !is_last && !result.closed {
                return Err(integrity(OverlayIntegrityCode::HistorySegmentTruncated));
            }
            verified.entries.extend(result.entries);
            verified.tail_hash = result.tail_hash;
            verified.last_segment_closed = result.closed;
            verified.last_segment_event_count = result.event_count;
            verified.last_segment_hash = result.segment_hash.clone();
            expected_segment_hash = result.segment_hash;
        }
        verified.segment_count = segments.len() as u64;
        Ok(verified)
    }
}

impl OverlayHistoryRepository for FilesystemOverlayHistoryRepository {
    fn read_verified_page(
        &self,
        key: &OverlayKey,
        query: &OverlayHistoryQuery,
    ) -> Result<OverlayHistoryPage, SkillApplicationError> {
        let verified = match self.verify(key) {
            Ok(verified) => verified,
            Err(SkillApplicationError::Overlay(OverlayApplicationError::Integrity { code })) => {
                return Ok(OverlayHistoryPage {
                    entries: Vec::new(),
                    next_cursor: None,
                    integrity: OverlayPageIntegrity::Failed(code),
                })
            }
            Err(error) => return Err(error),
        };
        let offset = parse_cursor(query.cursor.as_deref())?;
        let limit = query.limit.max(1).min(self.page_limit);
        let matching = verified
            .entries
            .into_iter()
            .rev()
            .filter(|entry| entry.scope == key.scope)
            .collect::<Vec<_>>();
        let entries = matching
            .iter()
            .skip(offset)
            .take(limit)
            .cloned()
            .collect::<Vec<_>>();
        let next_offset = offset.saturating_add(entries.len());
        let next_cursor = (next_offset < matching.len()).then(|| format!("offset:{next_offset}"));
        Ok(OverlayHistoryPage {
            entries,
            next_cursor,
            integrity: OverlayPageIntegrity::Verified,
        })
    }

    fn verified_tail_hash(
        &self,
        key: &OverlayKey,
    ) -> Result<Option<String>, SkillApplicationError> {
        self.verify(key).map(|verified| verified.tail_hash)
    }
}

#[derive(Default)]
struct VerifiedHistory {
    entries: Vec<OverlayHistoryEntry>,
    tail_hash: Option<String>,
    segment_count: u64,
    last_segment_closed: bool,
    last_segment_event_count: usize,
    last_segment_hash: Option<String>,
}

struct SegmentVerification {
    entries: Vec<OverlayHistoryEntry>,
    tail_hash: Option<String>,
    event_count: usize,
    closed: bool,
    segment_hash: Option<String>,
}

struct SegmentPath {
    sequence: u64,
    path: PathBuf,
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "record_type", rename_all = "snake_case", deny_unknown_fields)]
enum HistoryRecordWire {
    SegmentHeader {
        schema_version: u32,
        sequence: u64,
        previous_segment_hash: Option<String>,
    },
    Event(Box<EventWire>),
    SegmentFooter {
        schema_version: u32,
        sequence: u64,
        segment_hash: String,
    },
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct EventWire {
    event_id: String,
    canonical_skill_id: String,
    scope: String,
    prior_revision: Option<u64>,
    next_revision: u64,
    actor: String,
    action: String,
    timestamp: String,
    prior_document_hash: Option<String>,
    next_document_hash: String,
    scanner_version: String,
    safe_outcome: String,
    prior_event_hash: Option<String>,
    event_hash: String,
}

impl From<&OverlayHistoryEntry> for EventWire {
    fn from(entry: &OverlayHistoryEntry) -> Self {
        Self {
            event_id: entry.event_id.clone(),
            canonical_skill_id: entry.canonical_skill_id.as_str().to_string(),
            scope: entry.scope.as_str().to_string(),
            prior_revision: entry.prior_revision,
            next_revision: entry.next_revision,
            actor: actor_name(entry.actor).to_string(),
            action: action_name(entry.action).to_string(),
            timestamp: entry.timestamp.clone(),
            prior_document_hash: entry.prior_document_hash.clone(),
            next_document_hash: entry.next_document_hash.clone(),
            scanner_version: entry.scanner_version.clone(),
            safe_outcome: entry.safe_outcome.clone(),
            prior_event_hash: entry.prior_event_hash.clone(),
            event_hash: entry.event_hash.clone(),
        }
    }
}

impl TryFrom<EventWire> for OverlayHistoryEntry {
    type Error = SkillApplicationError;

    fn try_from(wire: EventWire) -> Result<Self, Self::Error> {
        Ok(Self {
            event_id: wire.event_id,
            canonical_skill_id: SkillId::parse(wire.canonical_skill_id)
                .map_err(|_| integrity(OverlayIntegrityCode::HistoryEventChainBroken))?,
            scope: OverlayScope::parse(&wire.scope)
                .ok_or_else(|| integrity(OverlayIntegrityCode::HistoryEventChainBroken))?,
            prior_revision: wire.prior_revision,
            next_revision: wire.next_revision,
            actor: parse_actor(&wire.actor)?,
            action: parse_action(&wire.action)?,
            timestamp: wire.timestamp,
            prior_document_hash: wire.prior_document_hash,
            next_document_hash: wire.next_document_hash,
            scanner_version: wire.scanner_version,
            safe_outcome: wire.safe_outcome,
            prior_event_hash: wire.prior_event_hash,
            event_hash: wire.event_hash,
        })
    }
}

fn verify_segment(
    key: &OverlayKey,
    bytes: &[u8],
    sequence: u64,
    expected_segment_hash: Option<&str>,
    initial_tail_hash: Option<&str>,
) -> Result<SegmentVerification, SkillApplicationError> {
    let lines = bytes
        .split_inclusive(|byte| *byte == b'\n')
        .collect::<Vec<_>>();
    let Some(first) = lines.first() else {
        return Err(integrity(OverlayIntegrityCode::HistorySegmentTruncated));
    };
    let header = parse_record(first)?;
    match header {
        HistoryRecordWire::SegmentHeader {
            schema_version,
            sequence: actual_sequence,
            previous_segment_hash,
        } if schema_version == HISTORY_SCHEMA_VERSION
            && actual_sequence == sequence
            && previous_segment_hash.as_deref() == expected_segment_hash => {}
        _ => return Err(integrity(OverlayIntegrityCode::HistoryEventChainBroken)),
    }

    let mut entries = Vec::new();
    let mut tail_hash = initial_tail_hash.map(str::to_string);
    let mut closed = false;
    let mut segment_hash = None;
    let mut bytes_before_line = first.len();
    for (line_index, line) in lines.iter().enumerate().skip(1) {
        let record = parse_record(line)?;
        match record {
            HistoryRecordWire::Event(wire) if !closed => {
                let entry: OverlayHistoryEntry = (*wire).try_into()?;
                validate_stored_event(key, &entry)?;
                if entry.prior_event_hash.as_deref() != tail_hash.as_deref()
                    || event_hash(&entry)? != entry.event_hash
                {
                    return Err(integrity(OverlayIntegrityCode::HistoryEventChainBroken));
                }
                tail_hash = Some(entry.event_hash.clone());
                entries.push(entry);
            }
            HistoryRecordWire::SegmentFooter {
                schema_version,
                sequence: footer_sequence,
                segment_hash: footer_hash,
            } if !closed && line_index + 1 == lines.len() => {
                if schema_version != HISTORY_SCHEMA_VERSION
                    || footer_sequence != sequence
                    || sha256(&bytes[..bytes_before_line]) != footer_hash
                {
                    return Err(integrity(OverlayIntegrityCode::HistoryEventChainBroken));
                }
                closed = true;
                segment_hash = Some(footer_hash);
            }
            _ => return Err(integrity(OverlayIntegrityCode::HistoryEventChainBroken)),
        }
        bytes_before_line += line.len();
    }
    Ok(SegmentVerification {
        event_count: entries.len(),
        entries,
        tail_hash,
        closed,
        segment_hash,
    })
}

fn event_hash(entry: &OverlayHistoryEntry) -> Result<String, SkillApplicationError> {
    #[derive(Serialize)]
    struct Material<'a> {
        event_id: &'a str,
        canonical_skill_id: &'a str,
        scope: &'a str,
        prior_revision: Option<u64>,
        next_revision: u64,
        actor: &'a str,
        action: &'a str,
        timestamp: &'a str,
        prior_document_hash: Option<&'a str>,
        next_document_hash: &'a str,
        scanner_version: &'a str,
        safe_outcome: &'a str,
        prior_event_hash: Option<&'a str>,
    }
    let material = Material {
        event_id: &entry.event_id,
        canonical_skill_id: entry.canonical_skill_id.as_str(),
        scope: entry.scope.as_str(),
        prior_revision: entry.prior_revision,
        next_revision: entry.next_revision,
        actor: actor_name(entry.actor),
        action: action_name(entry.action),
        timestamp: &entry.timestamp,
        prior_document_hash: entry.prior_document_hash.as_deref(),
        next_document_hash: &entry.next_document_hash,
        scanner_version: &entry.scanner_version,
        safe_outcome: &entry.safe_outcome,
        prior_event_hash: entry.prior_event_hash.as_deref(),
    };
    let bytes = serde_json::to_vec(&material).map_err(json_error)?;
    Ok(sha256(&bytes))
}

fn validate_append_event(
    key: &OverlayKey,
    entry: &OverlayHistoryEntry,
) -> Result<(), SkillApplicationError> {
    if entry.scope != key.scope {
        return Err(history_validation("history-event-scope-mismatch"));
    }
    validate_event_fields(entry).map_err(|_| history_validation("invalid-history-event"))?;
    if entry.canonical_skill_id != key.canonical_skill_id {
        return Err(history_validation("history-event-skill-mismatch"));
    }
    Ok(())
}

fn validate_stored_event(
    key: &OverlayKey,
    entry: &OverlayHistoryEntry,
) -> Result<(), SkillApplicationError> {
    let scope_allowed = match key.scope {
        OverlayScope::Project => entry.scope == OverlayScope::Project,
        OverlayScope::System | OverlayScope::User => {
            matches!(entry.scope, OverlayScope::System | OverlayScope::User)
        }
    };
    if entry.canonical_skill_id != key.canonical_skill_id || !scope_allowed {
        return Err(integrity(OverlayIntegrityCode::HistoryEventChainBroken));
    }
    validate_event_fields(entry)
}

fn validate_event_fields(entry: &OverlayHistoryEntry) -> Result<(), SkillApplicationError> {
    if entry.event_id.trim().is_empty()
        || entry.next_revision == 0
        || entry.timestamp.trim().is_empty()
        || entry.next_document_hash.trim().is_empty()
        || entry.scanner_version.trim().is_empty()
        || entry.safe_outcome.trim().is_empty()
    {
        return Err(integrity(OverlayIntegrityCode::HistoryEventChainBroken));
    }
    Ok(())
}

fn ordered_segments(root: &Path) -> Result<Vec<SegmentPath>, SkillApplicationError> {
    if !root.exists() {
        return Ok(Vec::new());
    }
    let root_metadata = fs::symlink_metadata(root).map_err(filesystem_error)?;
    if root_metadata.file_type().is_symlink() || !root_metadata.is_dir() {
        return Err(integrity(OverlayIntegrityCode::HistorySegmentMissing));
    }
    let entries = fs::read_dir(root)
        .map_err(filesystem_error)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(filesystem_error)?;
    let mut segments = Vec::new();
    for entry in entries {
        let file_type = entry.file_type().map_err(filesystem_error)?;
        let name = entry.file_name();
        let Some(name) = name.to_str() else {
            continue;
        };
        let Some(sequence) = parse_segment_name(name) else {
            continue;
        };
        if file_type.is_symlink() || !file_type.is_file() {
            return Err(integrity(OverlayIntegrityCode::HistorySegmentMissing));
        }
        segments.push(SegmentPath {
            sequence,
            path: entry.path(),
        });
    }
    segments.sort_by_key(|segment| segment.sequence);
    Ok(segments)
}

fn parse_segment_name(name: &str) -> Option<u64> {
    let sequence = name.strip_prefix("events-")?.strip_suffix(".jsonl")?;
    (sequence.len() == 10)
        .then(|| sequence.parse::<u64>().ok())
        .flatten()
}

fn segment_path(root: &Path, sequence: u64) -> PathBuf {
    root.join(format!("events-{sequence:010}.jsonl"))
}

fn header_line(
    sequence: u64,
    previous_segment_hash: Option<&str>,
) -> Result<Vec<u8>, SkillApplicationError> {
    record_line(&HistoryRecordWire::SegmentHeader {
        schema_version: HISTORY_SCHEMA_VERSION,
        sequence,
        previous_segment_hash: previous_segment_hash.map(str::to_string),
    })
}

fn footer_line(sequence: u64, segment_hash: &str) -> Result<Vec<u8>, SkillApplicationError> {
    record_line(&HistoryRecordWire::SegmentFooter {
        schema_version: HISTORY_SCHEMA_VERSION,
        sequence,
        segment_hash: segment_hash.to_string(),
    })
}

fn record_line(record: &HistoryRecordWire) -> Result<Vec<u8>, SkillApplicationError> {
    let mut line = serde_json::to_vec(record).map_err(json_error)?;
    line.push(b'\n');
    Ok(line)
}

fn parse_record(line: &[u8]) -> Result<HistoryRecordWire, SkillApplicationError> {
    let json = line
        .strip_suffix(b"\n")
        .ok_or_else(|| integrity(OverlayIntegrityCode::HistorySegmentTruncated))?;
    serde_json::from_slice(json)
        .map_err(|_| integrity(OverlayIntegrityCode::HistorySegmentTruncated))
}

fn fits_with_footer(
    limit: u64,
    sequence: u64,
    active: &[u8],
    event: &[u8],
) -> Result<bool, SkillApplicationError> {
    let mut candidate = active.to_vec();
    candidate.extend_from_slice(event);
    let footer = footer_line(sequence, &sha256(&candidate))?;
    Ok((candidate.len() + footer.len()) as u64 <= limit)
}

fn ensure_segment_capacity(
    limit: u64,
    sequence: u64,
    header_size: usize,
    event_size: usize,
) -> Result<(), SkillApplicationError> {
    let placeholder = footer_line(sequence, &"0".repeat(64))?;
    let actual = header_size + event_size + placeholder.len();
    if actual as u64 > limit {
        Err(history_limit(limit, actual))
    } else {
        Ok(())
    }
}

fn close_segment(path: &Path, bytes: &[u8], limit: u64) -> Result<String, SkillApplicationError> {
    let segment_hash = sha256(bytes);
    let sequence = path
        .file_name()
        .and_then(|name| name.to_str())
        .and_then(parse_segment_name)
        .ok_or_else(|| history_validation("invalid-history-segment-name"))?;
    let footer = footer_line(sequence, &segment_hash)?;
    if (bytes.len() + footer.len()) as u64 > limit {
        return Err(history_limit(limit, bytes.len() + footer.len()));
    }
    append_and_sync(path, &footer)?;
    Ok(segment_hash)
}

fn create_history_root(root: &Path) -> Result<(), SkillApplicationError> {
    fs::create_dir_all(root).map_err(filesystem_error)?;
    let metadata = fs::symlink_metadata(root).map_err(filesystem_error)?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        Err(SkillApplicationError::Filesystem(
            "Overlay history root is not a safe directory".to_string(),
        ))
    } else {
        Ok(())
    }
}

fn write_new_segment(path: &Path, header: &[u8]) -> Result<(), SkillApplicationError> {
    let mut file = crate::platform::private_relay_fs::create_new_private_file(path)
        .map_err(filesystem_error)?;
    file.write_all(header).map_err(filesystem_error)?;
    file.sync_all().map_err(filesystem_error)
}

fn append_and_sync(path: &Path, bytes: &[u8]) -> Result<(), SkillApplicationError> {
    let mut file = crate::platform::private_relay_fs::open_private_file_for_append(path)
        .map_err(filesystem_error)?;
    file.write_all(bytes).map_err(filesystem_error)?;
    file.sync_all().map_err(filesystem_error)
}

fn parse_cursor(cursor: Option<&str>) -> Result<usize, SkillApplicationError> {
    match cursor {
        None => Ok(0),
        Some(cursor) => cursor
            .strip_prefix("offset:")
            .and_then(|value| value.parse().ok())
            .ok_or_else(|| history_validation("invalid-history-cursor")),
    }
}

fn actor_name(actor: OverlayActor) -> &'static str {
    match actor {
        OverlayActor::User => "user",
        OverlayActor::System => "system",
    }
}

fn parse_actor(actor: &str) -> Result<OverlayActor, SkillApplicationError> {
    match actor {
        "user" => Ok(OverlayActor::User),
        "system" => Ok(OverlayActor::System),
        _ => Err(integrity(OverlayIntegrityCode::HistoryEventChainBroken)),
    }
}

fn action_name(action: OverlayHistoryAction) -> &'static str {
    match action {
        OverlayHistoryAction::Create => "create",
        OverlayHistoryAction::Patch => "patch",
        OverlayHistoryAction::Learn => "learn",
        OverlayHistoryAction::File => "file",
        OverlayHistoryAction::Import => "import",
        OverlayHistoryAction::Promote => "promote",
        OverlayHistoryAction::Disable => "disable",
        OverlayHistoryAction::Revert => "revert",
        OverlayHistoryAction::Reconcile => "reconcile",
        OverlayHistoryAction::Conflict => "conflict",
    }
}

fn parse_action(action: &str) -> Result<OverlayHistoryAction, SkillApplicationError> {
    match action {
        "create" => Ok(OverlayHistoryAction::Create),
        "patch" => Ok(OverlayHistoryAction::Patch),
        "learn" => Ok(OverlayHistoryAction::Learn),
        "file" => Ok(OverlayHistoryAction::File),
        "import" => Ok(OverlayHistoryAction::Import),
        "promote" => Ok(OverlayHistoryAction::Promote),
        "disable" => Ok(OverlayHistoryAction::Disable),
        "revert" => Ok(OverlayHistoryAction::Revert),
        "reconcile" => Ok(OverlayHistoryAction::Reconcile),
        "conflict" => Ok(OverlayHistoryAction::Conflict),
        _ => Err(integrity(OverlayIntegrityCode::HistoryEventChainBroken)),
    }
}

fn sha256(content: &[u8]) -> String {
    Sha256::digest(content)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn integrity(code: OverlayIntegrityCode) -> SkillApplicationError {
    OverlayApplicationError::Integrity { code }.into()
}

fn history_limit(maximum: u64, actual: usize) -> SkillApplicationError {
    OverlayApplicationError::LimitExceeded {
        kind: crate::contexts::tooling::skills::application::OverlayLimitKind::HistorySegmentBytes,
        maximum,
        actual: actual as u64,
    }
    .into()
}

fn history_validation(code: &str) -> SkillApplicationError {
    OverlayApplicationError::InvalidRequest {
        code: code.to_string(),
    }
    .into()
}

fn layout_error(error: impl std::fmt::Display) -> SkillApplicationError {
    SkillApplicationError::Validation(error.to_string())
}

fn json_error(error: serde_json::Error) -> SkillApplicationError {
    SkillApplicationError::Filesystem(error.to_string())
}

fn filesystem_error(error: std::io::Error) -> SkillApplicationError {
    SkillApplicationError::Filesystem(error.to_string())
}
