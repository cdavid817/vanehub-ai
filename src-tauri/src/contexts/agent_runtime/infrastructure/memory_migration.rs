use super::memory_directory::FileAgentMemoryStore;
use super::memory_naming::{derive_description, derive_name};
use crate::contexts::agent_runtime::application::{AgentMemory, AgentRuntimeApplicationError};
use crate::contexts::agent_runtime::domain::{MemoryDocument, MemoryMetadata};
use std::collections::HashSet;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct MemoryMigrationOutcome {
    pub(crate) migrated: usize,
    /// Rows whose id already appears as a `migrated_from` value in the directory.
    pub(crate) skipped: usize,
    /// Rows that could not be converted at all. Logged and stepped over, never fatal.
    pub(crate) failed: usize,
}

/// Converts stored memory rows into memory files (`migrate-agent-memory-to-file-store`).
///
/// No model call: a generated name and a truncated leading sentence are derived deterministically,
/// and `type` is deliberately left absent. Guessing a type would need a credential and a network
/// round-trip at startup, and a wrong type is worse than an absent one — absent degrades
/// gracefully by spec, wrong does not, and the model can now set it in place as it touches each
/// memory.
///
/// Rows are never deleted here. They are the rollback: reverting the code restores the previous
/// behavior with the data intact.
pub(crate) fn migrate_memory_rows(
    store: &FileAgentMemoryStore,
    rows: &[AgentMemory],
) -> Result<MemoryMigrationOutcome, AgentRuntimeApplicationError> {
    let existing = store.scan()?;
    let already_migrated = existing
        .iter()
        .filter_map(|header| header.metadata.migrated_from.clone())
        .collect::<HashSet<_>>();
    let mut taken_names = existing
        .iter()
        .map(|header| header.metadata.name.clone())
        .collect::<HashSet<_>>();

    let mut outcome = MemoryMigrationOutcome::default();
    for row in rows {
        if already_migrated.contains(&row.id) {
            outcome.skipped += 1;
            continue;
        }
        match convert_row(row, &taken_names) {
            Some(document) => {
                taken_names.insert(document.metadata.name.clone());
                match store.write(&document) {
                    Ok(_) => outcome.migrated += 1,
                    Err(_) => outcome.failed += 1,
                }
            }
            None => outcome.failed += 1,
        }
    }

    store.reconcile_index()?;
    Ok(outcome)
}

fn convert_row(row: &AgentMemory, taken_names: &HashSet<String>) -> Option<MemoryDocument> {
    let body = row.content.trim();
    if body.is_empty() {
        return None;
    }
    let description = derive_description(body)?;
    let name = derive_name(&row.content, &row.id, taken_names);
    let metadata = MemoryMetadata::new(name, description, None)
        .ok()?
        .with_provenance(
            Some(row.agent_id.clone()),
            row.folder.clone(),
            Some(row.source.as_str().to_string()),
            Some(row.created_at.clone()),
        )
        .with_migrated_from(row.id.clone());
    MemoryDocument::new(metadata, body).ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contexts::agent_runtime::application::MemorySource;
    use crate::test_support::TempDirectory;
    use std::fs;

    fn row(id: &str, content: &str) -> AgentMemory {
        AgentMemory {
            id: id.to_string(),
            agent_id: "onepiece".to_string(),
            folder: Some("D:/code/vanehub-ai".to_string()),
            content: content.to_string(),
            source: MemorySource::Automatic,
            created_at: "2026-08-15T09:12:44Z".to_string(),
        }
    }

    struct Fixture {
        _directory: TempDirectory,
        store: FileAgentMemoryStore,
    }

    impl Fixture {
        fn new(label: &str) -> Self {
            let directory = TempDirectory::new(label);
            let store = FileAgentMemoryStore::new(directory.path()).expect("memory store");
            Self {
                _directory: directory,
                store,
            }
        }
    }

    #[test]
    fn a_row_becomes_a_file_preserving_content_and_provenance() {
        let fixture = Fixture::new("memory migration preserves");
        let rows = vec![row(
            "row-1",
            "The user prefers npm. Never pnpm in this repo.",
        )];

        let outcome = migrate_memory_rows(&fixture.store, &rows).expect("migrate");

        assert_eq!(outcome.migrated, 1);
        let headers = fixture.store.scan().expect("scan");
        assert_eq!(headers.len(), 1);
        let document = fixture
            .store
            .read(&headers[0].relative_path)
            .expect("read migrated memory");
        assert_eq!(
            document.body,
            "The user prefers npm. Never pnpm in this repo."
        );
        assert_eq!(document.metadata.agent_id.as_deref(), Some("onepiece"));
        assert_eq!(
            document.metadata.folder.as_deref(),
            Some("D:/code/vanehub-ai")
        );
        assert_eq!(document.metadata.source.as_deref(), Some("automatic"));
        assert_eq!(
            document.metadata.created_at.as_deref(),
            Some("2026-08-15T09:12:44Z")
        );
        assert_eq!(document.metadata.migrated_from.as_deref(), Some("row-1"));
        // Guessing a type would need a credential at startup, and a wrong type does not degrade.
        assert_eq!(document.metadata.memory_type, None);
    }

    #[test]
    fn the_description_is_the_leading_sentence_truncated() {
        let fixture = Fixture::new("memory migration description");
        let long = "a".repeat(500);
        let rows = vec![
            row("row-1", "First sentence. Second sentence."),
            row("row-2", &format!("{long}. Trailing.")),
            row("row-3", "Line one\nLine two"),
        ];

        migrate_memory_rows(&fixture.store, &rows).expect("migrate");

        let descriptions = fixture
            .store
            .scan()
            .expect("scan")
            .iter()
            .map(|header| header.metadata.description.clone())
            .collect::<Vec<_>>();
        // Derivation itself is covered in `memory_naming`; this asserts migration routes through
        // it rather than re-deriving descriptions its own way.
        assert!(descriptions.contains(&"First sentence".to_string()));
        assert!(descriptions.contains(&"Line one".to_string()));
        assert!(descriptions
            .iter()
            .any(|description| description.chars().count() < long.chars().count()));
    }

    #[test]
    fn migration_is_idempotent_through_migrated_from() {
        let fixture = Fixture::new("memory migration idempotent");
        let rows = vec![row("row-1", "A fact."), row("row-2", "Another fact.")];

        let first = migrate_memory_rows(&fixture.store, &rows).expect("first run");
        let second = migrate_memory_rows(&fixture.store, &rows).expect("second run");

        assert_eq!(first.migrated, 2);
        assert_eq!(second.migrated, 0);
        assert_eq!(second.skipped, 2);
        assert_eq!(fixture.store.scan().expect("scan").len(), 2);
    }

    #[test]
    fn a_second_run_does_not_overwrite_a_file_the_model_has_since_edited() {
        // This is the whole point of keying idempotence on the row id rather than on file absence.
        let fixture = Fixture::new("memory migration preserves edits");
        let rows = vec![row("row-1", "Original content.")];
        migrate_memory_rows(&fixture.store, &rows).expect("first run");
        let relative_path = fixture.store.scan().expect("scan")[0].relative_path.clone();
        let edited = MemoryDocument::new(
            fixture
                .store
                .read(&relative_path)
                .expect("read")
                .metadata
                .clone(),
            "Corrected content.",
        )
        .expect("edited document");
        fixture.store.write(&edited).expect("model correction");

        migrate_memory_rows(&fixture.store, &rows).expect("second run");

        assert_eq!(
            fixture.store.read(&relative_path).expect("read").body,
            "Corrected content."
        );
    }

    #[test]
    fn one_unconvertible_row_does_not_abort_the_batch() {
        let fixture = Fixture::new("memory migration failure isolation");
        let rows = vec![
            row("row-1", "A convertible fact."),
            row("row-2", "   "),
            row("row-3", "Another convertible fact."),
        ];

        let outcome = migrate_memory_rows(&fixture.store, &rows).expect("migrate");

        assert_eq!(outcome.migrated, 2);
        assert_eq!(outcome.failed, 1);
        assert_eq!(fixture.store.scan().expect("scan").len(), 2);
    }

    #[test]
    fn colliding_names_get_a_numeric_suffix() {
        let fixture = Fixture::new("memory migration collisions");
        // The first NAME_WORD_BUDGET words must be identical for these to collide at all —
        // divergence at word seven produces three distinct slugs and tests nothing.
        let rows = vec![
            row("row-1", "Same leading words here in all rows, first."),
            row("row-2", "Same leading words here in all rows, second."),
            row("row-3", "Same leading words here in all rows, third."),
        ];

        let outcome = migrate_memory_rows(&fixture.store, &rows).expect("migrate");

        assert_eq!(outcome.migrated, 3);
        let mut names = fixture
            .store
            .scan()
            .expect("scan")
            .iter()
            .map(|header| header.metadata.name.clone())
            .collect::<Vec<_>>();
        names.sort();
        assert_eq!(
            names,
            vec![
                "same-leading-words-here-in-all",
                "same-leading-words-here-in-all-2",
                "same-leading-words-here-in-all-3",
            ]
        );
    }

    #[test]
    fn identical_content_still_yields_distinct_names() {
        let fixture = Fixture::new("memory migration identical content");
        let rows = vec![
            row("row-1", "Exactly the same."),
            row("row-2", "Exactly the same."),
        ];

        let outcome = migrate_memory_rows(&fixture.store, &rows).expect("migrate");

        assert_eq!(outcome.migrated, 2);
        assert_eq!(fixture.store.scan().expect("scan").len(), 2);
    }

    #[test]
    fn chinese_content_names_itself_after_the_symbols_it_mentions() {
        // This pool holds plenty of Chinese memories. Slugging keeps ASCII alphanumerics, so an
        // identifier quoted inside Chinese prose becomes the name — which is the useful outcome,
        // far better than the row-id fallback.
        let fixture = Fixture::new("memory migration chinese with symbol");
        let rows = vec![row("abc123de-f456", "别把 compose_prompt 改成给路径。")];

        migrate_memory_rows(&fixture.store, &rows).expect("migrate");

        let headers = fixture.store.scan().expect("scan");
        assert_eq!(headers[0].metadata.name, "compose-prompt");
        assert_eq!(
            fixture
                .store
                .read(&headers[0].relative_path)
                .expect("read")
                .body,
            "别把 compose_prompt 改成给路径。"
        );
    }

    #[test]
    fn content_with_no_ascii_at_all_falls_back_to_a_row_id_name() {
        let fixture = Fixture::new("memory migration pure chinese");
        let rows = vec![row("abc123de-f456", "别把提示词改成给路径，会静默失效。")];

        migrate_memory_rows(&fixture.store, &rows).expect("migrate");

        let headers = fixture.store.scan().expect("scan");
        assert_eq!(headers[0].metadata.name, "memory-abc123de");
        // The description still carries the real text, so the memory stays recognizable in the
        // index despite the opaque name.
        assert_eq!(
            headers[0].metadata.description,
            "别把提示词改成给路径，会静默失效"
        );
    }

    #[test]
    fn content_slugging_to_a_windows_device_name_falls_back() {
        // `con.md` is not a creatable file on Windows regardless of extension.
        let fixture = Fixture::new("memory migration device name");
        let rows = vec![row("row-9", "con")];

        migrate_memory_rows(&fixture.store, &rows).expect("migrate");

        let headers = fixture.store.scan().expect("scan");
        assert_eq!(headers[0].metadata.name, "memory-row9");
    }

    #[test]
    fn migration_writes_the_index() {
        let fixture = Fixture::new("memory migration index");
        let rows = vec![row("row-1", "A migrated fact.")];

        migrate_memory_rows(&fixture.store, &rows).expect("migrate");

        let index = fs::read_to_string(fixture.store.root().join("MEMORY.md")).expect("index");
        assert!(index.contains("a-migrated-fact.md"));
    }

    #[test]
    fn an_empty_row_set_still_writes_an_empty_index() {
        // The index's existence is what tells startup the directory has been initialized, so a
        // fresh installation with no rows must still get one.
        let fixture = Fixture::new("memory migration empty");

        let outcome = migrate_memory_rows(&fixture.store, &[]).expect("migrate");

        assert_eq!(outcome, MemoryMigrationOutcome::default());
        assert!(fixture.store.root().join("MEMORY.md").is_file());
    }
}
