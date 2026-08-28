//! The authoritative file format, and what happens to a file written by a build that had fewer
//! fields than this one.

use chrono::{DateTime, TimeZone, Utc};

use super::memory_document::{compose, parse};
use crate::contexts::personalization::domain::{
    AgentId, LegacyMemorySaveSource, MemoryAudience, MemoryId, MemoryProvenance, MemoryRecord,
    MemoryScope, MemorySensitivity, MemorySource, MemoryStatus, MemoryType, WorkspaceKey,
};

fn at() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 8, 24, 9, 0, 0).unwrap()
}

fn migrated_record() -> MemoryRecord {
    MemoryRecord {
        id: MemoryId::parse("01K2MEM0000000000000000001").expect("id"),
        name: "user-role".to_string(),
        description: "The user is a data scientist".to_string(),
        memory_type: MemoryType::Untyped,
        content: "Prefers concise answers.".to_string(),
        scope: MemoryScope::Global,
        audience: MemoryAudience::AllAgents,
        status: MemoryStatus::Active,
        source: MemorySource::LegacyMigration,
        provenance: MemoryProvenance {
            source_agent_id: Some(AgentId::parse("onepiece").expect("agent")),
            source_session_id: None,
            source_message_id: None,
            source_workspace_key: Some(WorkspaceKey::parse("ws_0123456789abcdef").expect("key")),
            legacy_original_save_source: Some(LegacyMemorySaveSource::Explicit),
            legacy_folder: Some("D:/code/vanehub-ai".to_string()),
            legacy_source_relative_path: Some("user-role.md".to_string()),
        },
        sensitivity: MemorySensitivity::Normal,
        revision: 1,
        created_at: at(),
        updated_at: at(),
        verified_at: None,
        last_used_at: None,
        use_count: 0,
    }
}

#[test]
fn legacy_provenance_round_trips_through_the_file() {
    let record = migrated_record();

    let parsed = parse(&compose(&record)).expect("round trip");

    assert_eq!(parsed, record);
}

#[test]
fn a_file_written_before_these_fields_existed_still_parses() {
    // Built by removing the new lines from a composed file rather than by pasting bytes: the body
    // hash covers only the body, so this is exactly what the previous build wrote, and it stays
    // exact as the rest of the format evolves.
    let record = migrated_record();
    let older: String = compose(&record)
        .lines()
        .filter(|line| {
            !line.starts_with("legacy_save_source:")
                && !line.starts_with("legacy_folder:")
                && !line.starts_with("legacy_source_path:")
        })
        .collect::<Vec<_>>()
        .join("\n");

    let parsed = parse(&older).expect("a file without the new fields is not malformed");

    assert_eq!(parsed.provenance.legacy_original_save_source, None);
    assert_eq!(parsed.provenance.legacy_folder, None);
    assert_eq!(parsed.provenance.legacy_source_relative_path, None);
    // Everything that was already there is untouched.
    assert_eq!(parsed.id, record.id);
    assert_eq!(parsed.content, record.content);
    assert_eq!(parsed.revision, record.revision);
    assert_eq!(
        parsed.provenance.source_workspace_key,
        record.provenance.source_workspace_key
    );
}

#[test]
fn a_record_without_legacy_provenance_writes_no_lines_for_it() {
    // Additive means additive: a memory that was never migrated must produce the same bytes it
    // produced before these fields existed, or every file would rewrite on its next save.
    let mut record = migrated_record();
    record.source = MemorySource::ExplicitUser;
    record.memory_type = MemoryType::Project;
    record.provenance.legacy_original_save_source = None;
    record.provenance.legacy_folder = None;
    record.provenance.legacy_source_relative_path = None;

    let composed = compose(&record);

    assert!(!composed.contains("legacy_save_source"));
    assert!(!composed.contains("legacy_folder"));
    assert!(!composed.contains("legacy_source_path"));
    assert_eq!(parse(&composed).expect("round trip"), record);
}

#[test]
fn a_folder_containing_frontmatter_punctuation_survives() {
    // The raw value comes from a file this build did not write. Encoding it is what stops a path
    // with a colon, a quote, or a newline in it from forging another frontmatter field.
    for folder in [
        "D:/code: notes/project",
        "D:/code/\"quoted\"/project",
        "D:/code/line\nbreak",
        "D:/代码/项目",
        "/srv/project #1",
    ] {
        let mut record = migrated_record();
        record.provenance.legacy_folder = Some(folder.to_string());

        let parsed = parse(&compose(&record)).expect("round trip");

        assert_eq!(
            parsed.provenance.legacy_folder.as_deref(),
            Some(folder),
            "for {folder:?}"
        );
        assert_eq!(parsed, record);
    }
}

#[test]
fn an_unrecognized_save_source_makes_the_file_malformed_rather_than_silently_absent() {
    // This build wrote the value, so an unreadable one means the file was edited or written by a
    // build with a different contract. Dropping it would activate a record whose provenance this
    // build cannot vouch for; refusing sends it to quarantine, where it is visible.
    let composed = compose(&migrated_record()).replace(
        "legacy_save_source: explicit",
        "legacy_save_source: invented",
    );

    assert!(parse(&composed).is_err());
}
