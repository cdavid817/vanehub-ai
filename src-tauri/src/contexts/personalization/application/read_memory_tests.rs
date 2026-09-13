//! The frozen read context as a value: what freezes it, what it admits, and what it refuses.
//!
//! Pure domain and application logic over hand-built snapshots. The real-store behaviour --
//! files, projection, health -- is exercised in `api::read_scope_tests`.

use chrono::Utc;

use crate::contexts::personalization::domain::{
    AgentId, AgentRuntimeKind, EffectiveMemoryAccess, EffectivePersonalizationSnapshot,
    InstructionMergeMode, MemoryAudience, MemoryDeliveryMode, MemoryEligibilitySummary, MemoryId,
    MemoryProvenance, MemoryReadContext, MemoryReadHandle, MemoryRecord, MemoryScope,
    MemorySensitivity, MemorySource, MemoryStatus, MemoryType, PersonalizationExclusionReason,
    PersonalizationResolutionContext, SessionId, SessionPersonalizationMode, WorkspaceBinding,
    WorkspaceIdentity, WorkspaceKey, WorkspaceKind,
};

const EPOCH: &str = "epoch-a";

fn agent(id: &str) -> AgentId {
    AgentId::parse(id).expect("agent id")
}

fn workspace(key: &str) -> WorkspaceKey {
    WorkspaceKey::parse(key).expect("workspace key")
}

/// A resolved standard-mode snapshot that permits reading, with the given global allowance and
/// workspace. Everything else is irrelevant to the read context and left at safe defaults.
fn permissive_snapshot(
    agent_id: &str,
    mode: SessionPersonalizationMode,
    global: bool,
    workspace_key: Option<&str>,
) -> EffectivePersonalizationSnapshot {
    let identity = workspace_key.map(|key| {
        WorkspaceIdentity::new(workspace(key), format!("/code/{key}"), WorkspaceKind::Local)
    });
    EffectivePersonalizationSnapshot {
        revision_token: "policy-revision-7".to_string(),
        context: PersonalizationResolutionContext {
            agent_id: agent(agent_id),
            session_id: SessionId::parse("session-1").expect("session id"),
            workspace: identity,
            runtime_kind: AgentRuntimeKind::OnePiece,
            session_mode: mode,
        },
        effective_instruction_mode: InstructionMergeMode::Append,
        instruction_segments: Vec::new(),
        excluded_instruction_segments: Vec::new(),
        memory_access: EffectiveMemoryAccess {
            read: true,
            explicit_save: true,
            automatic_extraction: false,
            global_memory: global,
            workspace: workspace_key.map(workspace),
            candidate_creation: true,
            retrieval_write: true,
            delivery: MemoryDeliveryMode::IndexWithSelectedBodies,
            block_reason: None,
        },
        memory: MemoryEligibilitySummary::default(),
        exclusions: Vec::new(),
        warnings: Vec::new(),
    }
}

fn record(scope: MemoryScope, audience: MemoryAudience, status: MemoryStatus) -> MemoryRecord {
    MemoryRecord {
        id: MemoryId::parse("01K2MEM0000000000000000001").expect("memory id"),
        name: "note".to_string(),
        description: "a note".to_string(),
        memory_type: MemoryType::Project,
        content: "body".to_string(),
        scope,
        audience,
        status,
        source: MemorySource::ExplicitUser,
        // Produced by the very Agent that is reading, so any admission that leaks through must be
        // a scope/audience decision and never provenance.
        provenance: MemoryProvenance {
            source_agent_id: Some(agent("agent-a")),
            ..MemoryProvenance::default()
        },
        sensitivity: MemorySensitivity::Normal,
        revision: 3,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        verified_at: None,
        last_used_at: None,
        use_count: 0,
    }
}

fn freeze(
    snapshot: &EffectivePersonalizationSnapshot,
    binding: WorkspaceBinding,
) -> MemoryReadContext {
    MemoryReadContext::freeze(snapshot, binding, "generation-1", None, 1, EPOCH)
}

#[test]
fn a_standard_context_admits_global_and_its_own_workspace_only_by_audience() {
    let snapshot = permissive_snapshot(
        "agent-a",
        SessionPersonalizationMode::Standard,
        true,
        Some("ws_1"),
    );
    let context = freeze(&snapshot, WorkspaceBinding::Resolved(workspace("ws_1")));
    assert!(context.permits_any_read());

    let in_workspace = |key: &str| MemoryScope::Workspace {
        workspace_key: workspace(key),
    };
    assert!(context
        .admits(&record(
            MemoryScope::Global,
            MemoryAudience::AllAgents,
            MemoryStatus::Active
        ))
        .is_ok());
    assert!(context
        .admits(&record(
            in_workspace("ws_1"),
            MemoryAudience::AllAgents,
            MemoryStatus::Active
        ))
        .is_ok());
    assert_eq!(
        context.admits(&record(
            in_workspace("ws_2"),
            MemoryAudience::AllAgents,
            MemoryStatus::Active
        )),
        Err(PersonalizationExclusionReason::OtherWorkspace)
    );
    // Selected audiences compare the complete stable id. The producing Agent is the reader here,
    // and that still grants nothing.
    assert_eq!(
        context.admits(&record(
            MemoryScope::Global,
            MemoryAudience::SelectedAgents {
                agent_ids: vec![agent("agent-b")]
            },
            MemoryStatus::Active
        )),
        Err(PersonalizationExclusionReason::AgentAudience)
    );
    assert_eq!(
        context.admits(&record(
            MemoryScope::Global,
            MemoryAudience::AllAgents,
            MemoryStatus::Candidate
        )),
        Err(PersonalizationExclusionReason::PendingCandidate)
    );
    assert_eq!(
        context.admits(&record(
            MemoryScope::Global,
            MemoryAudience::AllAgents,
            MemoryStatus::Archived
        )),
        Err(PersonalizationExclusionReason::Archived)
    );
}

#[test]
fn project_only_and_disabled_global_exclude_global_but_keep_the_workspace() {
    for (mode, global) in [
        (SessionPersonalizationMode::ProjectOnly, false),
        (SessionPersonalizationMode::Standard, false),
    ] {
        let snapshot = permissive_snapshot("agent-a", mode, global, Some("ws_1"));
        let context = freeze(&snapshot, WorkspaceBinding::Resolved(workspace("ws_1")));
        assert!(context.permits_any_read());
        assert!(!context.global_allowed);
        assert!(context
            .admits(&record(
                MemoryScope::Global,
                MemoryAudience::AllAgents,
                MemoryStatus::Active
            ))
            .is_err());
        assert!(context
            .admits(&record(
                MemoryScope::Workspace {
                    workspace_key: workspace("ws_1")
                },
                MemoryAudience::AllAgents,
                MemoryStatus::Active
            ))
            .is_ok());
    }
}

#[test]
fn an_explicitly_absent_workspace_still_reads_global_but_an_unresolved_one_reads_nothing() {
    // MR-05: the two are different answers, and collapsing them into `None` is how a session with
    // a workspace this build could not identify would fall open into the global pool.
    let snapshot = permissive_snapshot("agent-a", SessionPersonalizationMode::Standard, true, None);
    let absent = freeze(&snapshot, WorkspaceBinding::Absent);
    assert!(absent.permits_any_read());
    assert!(absent.global_allowed);
    assert_eq!(absent.workspace_allowed, None);

    let unresolved = freeze(&snapshot, WorkspaceBinding::Unresolved);
    assert!(!unresolved.permits_any_read());
    assert!(!unresolved.read);
    assert!(unresolved
        .admits(&record(
            MemoryScope::Global,
            MemoryAudience::AllAgents,
            MemoryStatus::Active
        ))
        .is_err());
}

#[test]
fn a_bound_workspace_that_differs_from_the_policy_allowance_grants_no_workspace_scope() {
    // The context can only ever be narrower than the snapshot: an allowance for `ws_1` bound to a
    // session in `ws_2` admits neither.
    let snapshot = permissive_snapshot(
        "agent-a",
        SessionPersonalizationMode::Standard,
        true,
        Some("ws_1"),
    );
    let context = freeze(&snapshot, WorkspaceBinding::Resolved(workspace("ws_2")));
    assert_eq!(context.workspace_allowed, None);
    assert!(context
        .admits(&record(
            MemoryScope::Workspace {
                workspace_key: workspace("ws_1")
            },
            MemoryAudience::AllAgents,
            MemoryStatus::Active
        ))
        .is_err());
}

#[test]
fn a_temporary_or_read_disabled_snapshot_freezes_a_context_that_permits_nothing() {
    let temporary =
        permissive_snapshot("agent-a", SessionPersonalizationMode::Temporary, true, None);
    let context = freeze(&temporary, WorkspaceBinding::Absent);
    assert!(!context.permits_any_read());
    assert_eq!(
        context.admits(&record(
            MemoryScope::Global,
            MemoryAudience::AllAgents,
            MemoryStatus::Active
        )),
        Err(PersonalizationExclusionReason::TemporarySession)
    );

    let mut disabled =
        permissive_snapshot("agent-a", SessionPersonalizationMode::Standard, true, None);
    disabled.memory_access.read = false;
    let context = freeze(&disabled, WorkspaceBinding::Absent);
    assert!(!context.permits_any_read());
    assert_eq!(
        context.admits(&record(
            MemoryScope::Global,
            MemoryAudience::AllAgents,
            MemoryStatus::Active
        )),
        Err(PersonalizationExclusionReason::MemoryReadDisabled)
    );
}

#[test]
fn a_context_authenticates_only_under_the_epoch_that_minted_it_and_only_unaltered() {
    // MR-09/MR-10: a value assembled elsewhere, carried across a restart, or edited to widen its
    // own allowance is refused before any record is touched.
    let snapshot = permissive_snapshot(
        "agent-a",
        SessionPersonalizationMode::Standard,
        false,
        Some("ws_1"),
    );
    let context = freeze(&snapshot, WorkspaceBinding::Resolved(workspace("ws_1")));
    assert!(context.is_authentic(EPOCH));
    assert!(!context.is_authentic("epoch-b"));

    let mut widened = context.clone();
    widened.global_allowed = true;
    assert!(!widened.is_authentic(EPOCH));

    let mut other_seat = context.clone();
    other_seat.subject.seat_id = Some("seat-b".to_string());
    assert!(!other_seat.is_authentic(EPOCH));

    let mut other_generation = context.clone();
    other_generation.subject.generation_id = "generation-2".to_string();
    assert!(!other_generation.is_authentic(EPOCH));
}

#[test]
fn a_handle_matches_only_the_exact_version_and_authority_it_pinned() {
    // MR-16: an edit that kept the revision, a re-scoping that kept the body, and an archive
    // that kept both all fail the same comparison.
    let original = record(
        MemoryScope::Global,
        MemoryAudience::AllAgents,
        MemoryStatus::Active,
    );
    let handle = MemoryReadHandle::of(&original);
    assert!(handle.matches(&original));

    let mut edited_without_revision = original.clone();
    edited_without_revision.content = "a different body".to_string();
    assert!(!handle.matches(&edited_without_revision));

    let mut narrowed = original.clone();
    narrowed.audience = MemoryAudience::SelectedAgents {
        agent_ids: vec![agent("agent-b")],
    };
    assert!(!handle.matches(&narrowed));

    let mut archived = original.clone();
    archived.status = MemoryStatus::Archived;
    assert!(!handle.matches(&archived));

    let mut revised = original;
    revised.revision += 1;
    assert!(!handle.matches(&revised));
}

#[test]
fn the_source_id_mapping_is_derived_from_the_id_and_accepts_nothing_else() {
    let handle = MemoryReadHandle::of(&record(
        MemoryScope::Global,
        MemoryAudience::AllAgents,
        MemoryStatus::Active,
    ));
    assert_eq!(handle.source_id(), "01K2MEM0000000000000000001.md");
    assert!(MemoryReadHandle::id_from_source_id("01K2MEM0000000000000000001.md").is_ok());
    assert!(MemoryReadHandle::id_from_source_id("../other/01K2MEM0000000000000000001.md").is_err());
    assert!(MemoryReadHandle::id_from_source_id("MEMORY.md").is_err());
}

#[test]
fn the_scope_fingerprint_carries_no_identifiers_a_log_should_not_hold() {
    let snapshot = permissive_snapshot(
        "agent-a",
        SessionPersonalizationMode::Standard,
        true,
        Some("ws_1"),
    );
    let context = freeze(&snapshot, WorkspaceBinding::Resolved(workspace("ws_1")));
    let fingerprint = context.scope_fingerprint();
    assert_eq!(fingerprint.len(), 16);
    assert!(!fingerprint.contains("ws_1"));
    assert!(!fingerprint.contains("agent-a"));
}
