use super::{AgentMemoryProposal, AgentMemoryRef};
use crate::contexts::agent_runtime::domain::{MemoryActionKind, ParsedMemoryActions};

/// Turns one extraction's validated actions into proposals against the eligible set.
///
/// Names are how extraction refers to memories, because a name is what the manifest showed it.
/// Resolving a name to an immutable id and the revision the snapshot pinned happens here, and
/// against the eligible set alone: an update or delete naming something outside it becomes nothing
/// rather than a proposal against a memory this generation was never shown.
///
/// An update naming nothing eligible becomes a create. The model is describing a memory that does
/// not exist under that name, which is what a create is — and the alternative, dropping it, would
/// silently lose the observation because the model guessed at a name.
///
/// A delete becomes an archive proposal. The model proposing removal is not evidence strong enough
/// to destroy a record the user may have written, and archive is reversible where delete is not.
pub(crate) fn proposals_from_actions(
    parsed: &ParsedMemoryActions,
    eligible: &[AgentMemoryRef],
) -> Vec<AgentMemoryProposal> {
    let mut proposals = Vec::new();
    for action in &parsed.actions {
        let target = eligible.iter().find(|entry| entry.name == action.name);
        let proposal = match (action.kind, target) {
            (MemoryActionKind::Delete, Some(entry)) => Some(AgentMemoryProposal::Archive {
                target_id: entry.id.clone(),
                expected_revision: entry.revision,
            }),
            // Nothing eligible carries that name, so there is nothing to retract. Proposing an
            // archive against a guessed id is the one thing this must not do.
            (MemoryActionKind::Delete, None) => None,
            (MemoryActionKind::Update, Some(entry)) => Some(AgentMemoryProposal::Update {
                target_id: entry.id.clone(),
                expected_revision: entry.revision,
                description: action.description.clone(),
                content: action.body.clone(),
            }),
            (MemoryActionKind::Create, _) | (MemoryActionKind::Update, None) => {
                Some(AgentMemoryProposal::Create {
                    name: action.name.clone(),
                    description: action.description.clone().unwrap_or_default(),
                    memory_type: action.memory_type,
                    content: action.body.clone().unwrap_or_default(),
                })
            }
        };
        proposals.extend(proposal);
    }
    proposals
}

/// Renders the eligible set as the manifest the extraction prompt carries.
///
/// This is the load-bearing part of deduplication. Without it the model cannot name an existing
/// memory, so every extraction can only create and the pool grows without bound.
///
/// Built from what the snapshot ruled eligible rather than from the store. The manifest is part of
/// a prompt, so a manifest listing everything would show a session names, types and descriptions
/// of memories its policy excluded — and it would invite proposals against them. Descriptions
/// only, never bodies: the manifest's cost must scale with how many memories exist rather than
/// with how large they are.
pub(crate) fn render_existing_manifest(eligible: &[AgentMemoryRef]) -> String {
    eligible
        .iter()
        .map(|entry| {
            let tag = entry
                .memory_type
                .map(|memory_type| format!("[{}] ", memory_type.as_str()))
                .unwrap_or_default();
            format!("- {tag}{} — {}", entry.name, entry.description)
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contexts::agent_runtime::domain::{MemoryAction, MemoryActionRejection, MemoryType};

    fn eligible(name: &str, revision: u64) -> AgentMemoryRef {
        AgentMemoryRef {
            id: format!("{name}.md"),
            revision,
            name: name.to_string(),
            description: format!("About {name}"),
            memory_type: Some(MemoryType::Project),
            updated_at: None,
        }
    }

    fn action(kind: MemoryActionKind, name: &str) -> MemoryAction {
        MemoryAction {
            kind,
            name: name.to_string(),
            description: Some(format!("About {name}")),
            memory_type: Some(MemoryType::Project),
            body: Some("Never pnpm in this repo.".to_string()),
        }
    }

    fn parsed(actions: Vec<MemoryAction>) -> ParsedMemoryActions {
        ParsedMemoryActions {
            actions,
            rejections: Vec::new(),
        }
    }

    #[test]
    fn a_create_becomes_a_create_proposal_carrying_the_model_chosen_metadata() {
        let proposals = proposals_from_actions(
            &parsed(vec![action(MemoryActionKind::Create, "npm-only")]),
            &[],
        );

        assert_eq!(
            proposals,
            vec![AgentMemoryProposal::Create {
                name: "npm-only".to_string(),
                description: "About npm-only".to_string(),
                memory_type: Some(MemoryType::Project),
                content: "Never pnpm in this repo.".to_string(),
            }]
        );
    }

    #[test]
    fn an_update_naming_an_eligible_memory_pins_its_id_and_revision() {
        let proposals = proposals_from_actions(
            &parsed(vec![action(MemoryActionKind::Update, "npm-only")]),
            &[eligible("npm-only", 7)],
        );

        assert_eq!(
            proposals,
            vec![AgentMemoryProposal::Update {
                target_id: "npm-only.md".to_string(),
                expected_revision: 7,
                description: Some("About npm-only".to_string()),
                content: Some("Never pnpm in this repo.".to_string()),
            }]
        );
    }

    /// The model is describing a memory that does not exist under that name. Dropping it would
    /// lose the observation because the model guessed at a name.
    #[test]
    fn an_update_naming_nothing_eligible_becomes_a_create_proposal() {
        let proposals = proposals_from_actions(
            &parsed(vec![action(MemoryActionKind::Update, "never-existed")]),
            &[eligible("something-else", 1)],
        );

        assert!(matches!(
            proposals.as_slice(),
            [AgentMemoryProposal::Create { name, .. }] if name == "never-existed"
        ));
    }

    /// A model proposing removal is not evidence strong enough to destroy a record the user may
    /// have written, and archive is reversible where delete is not.
    #[test]
    fn a_delete_becomes_an_archive_proposal_rather_than_a_deletion() {
        let proposals = proposals_from_actions(
            &parsed(vec![MemoryAction {
                kind: MemoryActionKind::Delete,
                name: "stale".to_string(),
                description: None,
                memory_type: None,
                body: None,
            }]),
            &[eligible("stale", 3)],
        );

        assert_eq!(
            proposals,
            vec![AgentMemoryProposal::Archive {
                target_id: "stale.md".to_string(),
                expected_revision: 3,
            }]
        );
    }

    /// Proposing an archive against a guessed id is the one thing this must not do.
    #[test]
    fn a_delete_naming_nothing_eligible_proposes_nothing_at_all() {
        let proposals = proposals_from_actions(
            &parsed(vec![MemoryAction {
                kind: MemoryActionKind::Delete,
                name: "never-seen".to_string(),
                description: None,
                memory_type: None,
                body: None,
            }]),
            &[eligible("something-else", 1)],
        );

        assert!(proposals.is_empty());
    }

    #[test]
    fn parser_rejections_produce_no_proposals_of_their_own() {
        let proposals = proposals_from_actions(
            &ParsedMemoryActions {
                actions: vec![action(MemoryActionKind::Create, "kept")],
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
            &[],
        );

        assert_eq!(proposals.len(), 1);
    }

    #[test]
    fn an_empty_action_list_proposes_nothing() {
        assert!(proposals_from_actions(&ParsedMemoryActions::default(), &[]).is_empty());
    }

    /// A manifest listing everything would show a session the names, types and descriptions of
    /// memories its own policy excluded — and would invite proposals against them.
    #[test]
    fn the_manifest_carries_only_the_eligible_set_and_never_a_body() {
        let manifest = render_existing_manifest(&[eligible("npm-only", 1)]);

        assert_eq!(manifest, "- [project] npm-only — About npm-only");
        assert!(!manifest.contains("Never pnpm"));
    }

    #[test]
    fn an_empty_eligible_set_renders_an_empty_manifest() {
        assert!(render_existing_manifest(&[]).is_empty());
    }
}
