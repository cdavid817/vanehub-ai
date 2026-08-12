use super::overlay_history::{FilesystemOverlayHistoryRepository, HISTORY_SEGMENT_BYTES};
use super::overlay_layout::OverlayStorageLayout;
use crate::contexts::tooling::skills::application::{
    OverlayActor, OverlayHistoryAction, OverlayHistoryEntry, OverlayHistoryQuery,
    OverlayHistoryRepository, OverlayIntegrityCode, OverlayKey, OverlayPageIntegrity,
};
use crate::contexts::tooling::skills::domain::{OverlayScope, SkillId, DEFAULT_OVERLAY_LIMITS};
use crate::test_support::TempDirectory;
use std::fs;
use std::path::PathBuf;

fn key(scope: OverlayScope) -> OverlayKey {
    OverlayKey {
        canonical_skill_id: SkillId::parse("code-review").expect("valid Skill id"),
        scope,
        workspace_identity: None,
    }
}

fn event(index: u64, scope: OverlayScope, outcome_size: usize) -> OverlayHistoryEntry {
    OverlayHistoryEntry {
        event_id: format!("event-{index}"),
        canonical_skill_id: SkillId::parse("code-review").expect("valid Skill id"),
        scope,
        prior_revision: index.checked_sub(1),
        next_revision: index,
        actor: OverlayActor::User,
        action: OverlayHistoryAction::Patch,
        timestamp: format!("2026-08-11T00:00:{index:02}Z"),
        prior_document_hash: index
            .checked_sub(1)
            .map(|value| format!("document-{value}")),
        next_document_hash: format!("document-{index}"),
        scanner_version: "overlay-text-v1".to_string(),
        safe_outcome: format!("outcome-{index}-{}", "x".repeat(outcome_size)),
        prior_event_hash: None,
        event_hash: String::new(),
    }
}

fn segment_paths(home: &TempDirectory, key: &OverlayKey) -> Vec<PathBuf> {
    let root = OverlayStorageLayout::resolve(home.path(), key)
        .expect("layout")
        .history_root;
    let mut paths = fs::read_dir(root)
        .expect("history root")
        .map(|entry| entry.expect("history entry").path())
        .collect::<Vec<_>>();
    paths.sort();
    paths
}

fn append_until_segments(
    repository: &FilesystemOverlayHistoryRepository,
    key: &OverlayKey,
    count: usize,
) {
    for index in 1..=count as u64 {
        repository
            .append_verified(key, event(index, key.scope, 220))
            .expect("append history event");
    }
}

fn assert_integrity(
    repository: &FilesystemOverlayHistoryRepository,
    key: &OverlayKey,
    code: OverlayIntegrityCode,
) {
    let page = repository
        .read_verified_page(key, &OverlayHistoryQuery::bounded(None, 10, 50))
        .expect("integrity result");
    assert_eq!(page.integrity, OverlayPageIntegrity::Failed(code));
    assert!(page.entries.is_empty());
}

#[test]
fn history_events_persist_safe_fields_and_link_event_hashes() {
    let home = TempDirectory::new("overlay-history-events");
    let repository = FilesystemOverlayHistoryRepository::with_home_root(home.path().to_path_buf());
    let key = key(OverlayScope::System);
    let first = repository
        .append_verified(&key, event(1, OverlayScope::System, 0))
        .expect("first event");
    let second = repository
        .append_verified(&key, event(2, OverlayScope::System, 0))
        .expect("second event");

    assert_eq!(first.prior_event_hash, None);
    assert_eq!(
        second.prior_event_hash.as_deref(),
        Some(first.event_hash.as_str())
    );
    assert!(!first.event_hash.is_empty());
    let page = repository
        .read_verified_page(&key, &OverlayHistoryQuery::bounded(None, 10, 50))
        .expect("verified page");
    assert_eq!(page.integrity, OverlayPageIntegrity::Verified);
    assert_eq!(
        page.entries
            .iter()
            .map(|entry| entry.event_id.as_str())
            .collect::<Vec<_>>(),
        vec!["event-2", "event-1"]
    );
    assert_eq!(page.entries[0], second);
    assert_eq!(
        repository.verified_tail_hash(&key).expect("tail hash"),
        Some(page.entries[0].event_hash.clone())
    );
}

#[test]
fn history_rolls_over_before_four_mib_and_links_segments() {
    assert_eq!(
        HISTORY_SEGMENT_BYTES,
        DEFAULT_OVERLAY_LIMITS.maximum_history_segment_bytes
    );
    let home = TempDirectory::new("overlay-history-rollover");
    let repository = FilesystemOverlayHistoryRepository::with_home_root(home.path().to_path_buf());
    let key = key(OverlayScope::System);
    for index in 1..=4 {
        repository
            .append_verified(&key, event(index, OverlayScope::System, 1_400_000))
            .expect("append large history event");
    }
    let paths = segment_paths(&home, &key);

    assert!(paths.len() >= 2);
    assert!(paths.iter().all(|path| {
        fs::metadata(path).expect("segment metadata").len() <= HISTORY_SEGMENT_BYTES
    }));
    let second = fs::read_to_string(&paths[1]).expect("second segment");
    let marker = "\"previous_segment_hash\":\"";
    let hash_start = second.find(marker).expect("segment link") + marker.len();
    assert_eq!(second[hash_start..].chars().take(64).count(), 64);
    assert_eq!(
        repository
            .read_verified_page(&key, &OverlayHistoryQuery::bounded(None, 20, 50))
            .expect("verified rollover")
            .integrity,
        OverlayPageIntegrity::Verified
    );

    let mut broken = second;
    broken.replace_range(hash_start..hash_start + 64, &"0".repeat(64));
    fs::write(&paths[1], broken).expect("break segment link");
    assert_integrity(
        &repository,
        &key,
        OverlayIntegrityCode::HistoryEventChainBroken,
    );
}

#[test]
fn history_pagination_is_bounded_and_has_no_duplicates() {
    let home = TempDirectory::new("overlay-history-pagination");
    let repository =
        FilesystemOverlayHistoryRepository::with_limits(home.path().to_path_buf(), 4_096, 2);
    let key = key(OverlayScope::System);
    append_until_segments(&repository, &key, 5);
    let first = repository
        .read_verified_page(&key, &OverlayHistoryQuery::bounded(None, 20, 20))
        .expect("first page");
    let second = repository
        .read_verified_page(
            &key,
            &OverlayHistoryQuery::bounded(first.next_cursor.clone(), 20, 20),
        )
        .expect("second page");
    let third = repository
        .read_verified_page(
            &key,
            &OverlayHistoryQuery::bounded(second.next_cursor.clone(), 20, 20),
        )
        .expect("third page");

    assert_eq!(first.entries.len(), 2);
    assert_eq!(second.entries.len(), 2);
    assert_eq!(third.entries.len(), 1);
    assert_eq!(third.next_cursor, None);
    let ids = first
        .entries
        .iter()
        .chain(&second.entries)
        .chain(&third.entries)
        .map(|entry| entry.event_id.clone())
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(ids.len(), 5);
}

#[test]
fn tampering_is_reported_and_blocks_further_appends() {
    let home = TempDirectory::new("overlay-history-tamper");
    let repository =
        FilesystemOverlayHistoryRepository::with_limits(home.path().to_path_buf(), 4_096, 50);
    let key = key(OverlayScope::System);
    repository
        .append_verified(&key, event(1, OverlayScope::System, 0))
        .expect("append event");
    let path = segment_paths(&home, &key).remove(0);
    let content = fs::read_to_string(&path).expect("segment");
    fs::write(&path, content.replacen("outcome-1-", "outcome-X-", 1)).expect("tamper segment");

    assert_integrity(
        &repository,
        &key,
        OverlayIntegrityCode::HistoryEventChainBroken,
    );
    assert!(repository
        .append_verified(&key, event(2, OverlayScope::System, 0))
        .is_err());
}

#[test]
fn truncation_is_reported_without_repairing_the_segment() {
    let home = TempDirectory::new("overlay-history-truncated");
    let repository =
        FilesystemOverlayHistoryRepository::with_limits(home.path().to_path_buf(), 4_096, 50);
    let key = key(OverlayScope::System);
    repository
        .append_verified(&key, event(1, OverlayScope::System, 0))
        .expect("append event");
    let path = segment_paths(&home, &key).remove(0);
    let mut bytes = fs::read(&path).expect("segment");
    bytes.pop();
    fs::write(&path, bytes).expect("truncate segment");

    assert_integrity(
        &repository,
        &key,
        OverlayIntegrityCode::HistorySegmentTruncated,
    );
}

#[test]
fn a_missing_middle_segment_is_reported() {
    let home = TempDirectory::new("overlay-history-missing");
    let repository =
        FilesystemOverlayHistoryRepository::with_limits(home.path().to_path_buf(), 1_100, 50);
    let key = key(OverlayScope::System);
    append_until_segments(&repository, &key, 12);
    let paths = segment_paths(&home, &key);
    assert!(paths.len() >= 3);
    fs::remove_file(&paths[1]).expect("remove middle segment");

    assert_integrity(
        &repository,
        &key,
        OverlayIntegrityCode::HistorySegmentMissing,
    );
}
