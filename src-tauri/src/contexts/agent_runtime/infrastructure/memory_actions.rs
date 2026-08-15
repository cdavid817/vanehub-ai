use crate::contexts::agent_runtime::application::{AgentMemoryPort, MemorySource, SaveMemoryInput};
use crate::contexts::agent_runtime::domain::{MemoryActionKind, ParsedMemoryActions};

/// Outcome of applying one extraction's actions. Counts only — never the content that was written,
/// because this is what reaches the unified log.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct AppliedMemoryActions {
    pub(crate) written: usize,
    pub(crate) deleted: usize,
    /// Actions the parser dropped, plus actions whose store operation failed.
    pub(crate) rejected: usize,
}

/// Applies a validated action list to the memory store.
///
/// Create and update are the same store operation: saving under a name that already exists
/// replaces that memory. Keeping them distinct in the schema is still worth it, because it tells
/// the model that correcting an existing memory is a thing it may do — which is the behavior the
/// row store could not express at all.
///
/// A failing action is counted and stepped over rather than aborting the batch. Extraction is
/// best-effort background work; one unusable action must not discard the rest, and none of it may
/// fail the generation that triggered it.
pub(crate) fn apply_memory_actions(
    memories: &dyn AgentMemoryPort,
    agent_id: &str,
    folder: Option<&str>,
    source: MemorySource,
    parsed: &ParsedMemoryActions,
) -> AppliedMemoryActions {
    let mut outcome = AppliedMemoryActions {
        rejected: parsed.rejections.len(),
        ..AppliedMemoryActions::default()
    };
    for action in &parsed.actions {
        let applied = match action.kind {
            MemoryActionKind::Delete => memories
                .delete(&format!("{}.md", action.name))
                .map(|()| &mut outcome.deleted),
            MemoryActionKind::Create | MemoryActionKind::Update => memories
                .save(SaveMemoryInput {
                    agent_id,
                    folder,
                    name: Some(&action.name),
                    description: action.description.as_deref(),
                    memory_type: action.memory_type,
                    content: action.body.as_deref().unwrap_or_default(),
                    source,
                })
                .map(|()| &mut outcome.written),
        };
        match applied {
            Ok(counter) => *counter += 1,
            Err(_) => outcome.rejected += 1,
        }
    }
    outcome
}

/// Renders the existing pool as the manifest the extraction prompt carries.
///
/// This is the load-bearing part of deduplication. Without it the model cannot name an existing
/// memory, so every extraction can only create and the pool grows the same way the row store's
/// did. Descriptions only, never bodies: the manifest's cost must scale with how many memories
/// exist, not with how large they are.
pub(crate) fn render_existing_manifest(memories: &dyn AgentMemoryPort) -> String {
    // `list_all` reads bodies the manifest then discards. Acceptable while the pool is bounded by
    // the scan cap; a header-only port method is the fix if this ever shows up in a profile.
    let Ok(existing) = memories.list_all() else {
        return String::new();
    };
    existing
        .iter()
        .map(|memory| {
            let tag = memory
                .memory_type
                .map(|memory_type| format!("[{}] ", memory_type.as_str()))
                .unwrap_or_default();
            format!("- {tag}{} — {}", memory.name, memory.description)
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contexts::agent_runtime::domain::{MemoryAction, MemoryActionRejection, MemoryType};
    use crate::contexts::agent_runtime::infrastructure::FileAgentMemoryStore;
    use crate::test_support::TempDirectory;

    fn create(name: &str, body: &str) -> MemoryAction {
        MemoryAction {
            kind: MemoryActionKind::Create,
            name: name.to_string(),
            description: Some(format!("About {name}")),
            memory_type: Some(MemoryType::Project),
            body: Some(body.to_string()),
        }
    }

    fn parsed(actions: Vec<MemoryAction>) -> ParsedMemoryActions {
        ParsedMemoryActions {
            actions,
            rejections: Vec::new(),
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
    fn creates_are_written_with_their_model_chosen_metadata() {
        let fixture = Fixture::new("memory actions create");

        let outcome = apply_memory_actions(
            &fixture.store,
            "onepiece",
            Some("D:/code"),
            MemorySource::Automatic,
            &parsed(vec![create("npm-only", "Never pnpm in this repo.")]),
        );

        assert_eq!(outcome.written, 1);
        let stored = fixture.store.read("npm-only.md").expect("read");
        assert_eq!(stored.metadata.description, "About npm-only");
        assert_eq!(stored.metadata.memory_type, Some(MemoryType::Project));
        assert_eq!(stored.body, "Never pnpm in this repo.");
    }

    #[test]
    fn an_update_replaces_the_named_memory_rather_than_adding_a_second_one() {
        // The whole reason the schema carries a name: the row store could only ever append.
        let fixture = Fixture::new("memory actions update");
        apply_memory_actions(
            &fixture.store,
            "onepiece",
            None,
            MemorySource::Automatic,
            &parsed(vec![create("npm-only", "Original.")]),
        );

        let mut update = create("npm-only", "Corrected.");
        update.kind = MemoryActionKind::Update;
        let outcome = apply_memory_actions(
            &fixture.store,
            "onepiece",
            None,
            MemorySource::Automatic,
            &parsed(vec![update]),
        );

        assert_eq!(outcome.written, 1);
        assert_eq!(fixture.store.scan().expect("scan").len(), 1);
        assert_eq!(
            fixture.store.read("npm-only.md").expect("read").body,
            "Corrected."
        );
    }

    #[test]
    fn a_delete_retracts_the_named_memory() {
        let fixture = Fixture::new("memory actions delete");
        apply_memory_actions(
            &fixture.store,
            "onepiece",
            None,
            MemorySource::Automatic,
            &parsed(vec![create("stale", "Outdated.")]),
        );

        let outcome = apply_memory_actions(
            &fixture.store,
            "onepiece",
            None,
            MemorySource::Automatic,
            &parsed(vec![MemoryAction {
                kind: MemoryActionKind::Delete,
                name: "stale".to_string(),
                description: None,
                memory_type: None,
                body: None,
            }]),
        );

        assert_eq!(outcome.deleted, 1);
        assert!(fixture.store.scan().expect("scan").is_empty());
    }

    #[test]
    fn parser_rejections_are_carried_into_the_outcome() {
        let fixture = Fixture::new("memory actions rejections");

        let outcome = apply_memory_actions(
            &fixture.store,
            "onepiece",
            None,
            MemorySource::Automatic,
            &ParsedMemoryActions {
                actions: vec![create("kept", "Body.")],
                rejections: vec![
                    MemoryActionRejection {
                        index: 1,
                        reason: "invalid-name",
                    },
                    MemoryActionRejection {
                        index: 2,
                        reason: "missing-body",
                    },
                ],
            },
        );

        assert_eq!(outcome.written, 1);
        assert_eq!(outcome.rejected, 2);
        assert_eq!(fixture.store.scan().expect("scan").len(), 1);
    }

    #[test]
    fn an_empty_action_list_touches_nothing() {
        let fixture = Fixture::new("memory actions empty");

        let outcome = apply_memory_actions(
            &fixture.store,
            "onepiece",
            None,
            MemorySource::Automatic,
            &ParsedMemoryActions::default(),
        );

        assert_eq!(outcome, AppliedMemoryActions::default());
        assert!(fixture.store.scan().expect("scan").is_empty());
    }

    #[test]
    fn the_manifest_carries_descriptions_but_never_bodies() {
        let fixture = Fixture::new("memory actions manifest");
        apply_memory_actions(
            &fixture.store,
            "onepiece",
            None,
            MemorySource::Automatic,
            &parsed(vec![create("npm-only", "A body that must not appear.")]),
        );

        let manifest = render_existing_manifest(&fixture.store);

        assert!(manifest.contains("npm-only"));
        assert!(manifest.contains("About npm-only"));
        assert!(!manifest.contains("A body that must not appear."));
    }
}
