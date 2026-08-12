use super::{
    replay_overlay_scope_chain, BaseSkillResource, EffectiveResourceSource, OverlayBaseWitness,
    OverlayDocument, OverlayFile, OverlayIntegrityFailure, OverlayLearnBlock, OverlayPatch,
    OverlayScope, OverlayScopeReplayInput, OverlayScopeReplayStatus, OverlayTrust, SkillId,
    SkillLayer,
};

const WORKSPACE: &str = "D:/code/project";

#[test]
fn failed_scope_rolls_back_all_tentative_changes_and_blocks_higher_scopes() {
    let mut system = document(OverlayScope::System, None);
    system.patches.push(patch("system-patch", "base", "system"));
    system.files.push(file("system-file", "hash-system"));

    let mut user = document(OverlayScope::User, None);
    user.patches.push(patch("user-first", "system", "user"));
    user.patches.push(patch("user-fails", "missing", "never"));
    user.files.push(file("user-file", "hash-user"));

    let mut project = document(OverlayScope::Project, Some(WORKSPACE));
    project
        .patches
        .push(patch("project-patch", "user", "project"));

    let replay = replay_overlay_scope_chain(
        "base",
        &base_resources(),
        &[
            OverlayScopeReplayInput::verified(&system),
            OverlayScopeReplayInput::verified(&user),
            OverlayScopeReplayInput::verified(&project),
        ],
        Some(WORKSPACE),
        4,
    );

    assert_eq!(replay.base().instructions(), "base");
    assert_eq!(replay.effective().instructions(), "system");
    assert_eq!(replay.scope_results().len(), 3);

    let system_result = &replay.scope_results()[0];
    assert_eq!(system_result.status(), &OverlayScopeReplayStatus::Applied);
    assert_eq!(system_result.scope(), OverlayScope::System);
    assert_eq!(system_result.revision(), 1);
    let system_snapshot = system_result.output().expect("healthy System snapshot");
    assert_eq!(system_snapshot.instructions(), "system");
    assert_eq!(system_snapshot.instruction_hash().len(), 64);
    assert_eq!(system_result.input_hash(), replay.base().effective_hash());
    assert_eq!(
        system_result.output_hash(),
        Some(system_snapshot.effective_hash())
    );

    let user_result = &replay.scope_results()[1];
    assert!(matches!(
        user_result.status(),
        OverlayScopeReplayStatus::Conflict(conflict)
            if conflict.mutation_id() == Some("user-fails")
    ));
    assert!(user_result.output().is_none());
    assert_eq!(
        user_result.last_healthy_hash(),
        system_snapshot.effective_hash()
    );

    assert_eq!(
        replay.scope_results()[2].status(),
        &OverlayScopeReplayStatus::Blocked {
            failed_scope: OverlayScope::User,
        }
    );
    let effective_resource = replay
        .effective()
        .resource("references/shared.md")
        .expect("System resource remains effective");
    assert_eq!(effective_resource.content_hash, "hash-system");
    assert!(matches!(
        effective_resource.source,
        EffectiveResourceSource::Overlay {
            scope: OverlayScope::System,
            ..
        }
    ));
}

#[test]
fn document_integrity_failure_is_reported_without_a_tentative_output() {
    let mut system = document(OverlayScope::System, None);
    system.patches.push(patch("system-patch", "base", "system"));

    let replay = replay_overlay_scope_chain(
        "base",
        &base_resources(),
        &[OverlayScopeReplayInput::integrity_failure(
            &system,
            OverlayIntegrityFailure::DocumentHashMismatch,
        )],
        Some(WORKSPACE),
        4,
    );

    assert_eq!(replay.effective(), replay.base());
    assert_eq!(
        replay.scope_results()[0].status(),
        &OverlayScopeReplayStatus::IntegrityFailure(OverlayIntegrityFailure::DocumentHashMismatch)
    );
    assert!(replay.scope_results()[0].output().is_none());
}

#[test]
fn clean_base_drift_keeps_tentative_output_separate_from_last_healthy_effective() {
    let mut user = document(OverlayScope::User, None);
    user.patches
        .push(patch("drifted-patch", "base", "tentative-reconciled"));

    let replay = replay_overlay_scope_chain(
        "base with new context",
        &base_resources(),
        &[OverlayScopeReplayInput::base_drift(&user)],
        Some(WORKSPACE),
        4,
    );

    let result = &replay.scope_results()[0];
    assert_eq!(
        result.status(),
        &OverlayScopeReplayStatus::NeedsReconciliation
    );
    assert_eq!(replay.effective(), replay.base());
    assert_eq!(result.last_healthy_hash(), replay.base().effective_hash());
    assert_eq!(
        result
            .output()
            .expect("tentative drift replay")
            .instructions(),
        "tentative-reconciled with new context"
    );
}

#[test]
fn imported_untrusted_overlay_cannot_change_instructions_or_resources() {
    let mut imported = document(OverlayScope::User, None);
    imported
        .patches
        .push(patch("imported-patch", "base", "imported"));
    imported
        .learn_blocks
        .push(guidance("imported-guidance", "Imported guidance"));
    imported.files.push(file("imported-file", "hash-imported"));
    imported.quarantine_import("team-overlay.zip".to_string());

    let replay = replay_overlay_scope_chain(
        "base",
        &base_resources(),
        &[OverlayScopeReplayInput::verified(&imported)],
        Some(WORKSPACE),
        4,
    );

    assert_eq!(replay.effective(), replay.base());
    assert_eq!(replay.effective().instructions(), "base");
    assert_eq!(
        replay
            .effective()
            .resource("references/shared.md")
            .expect("base resource remains effective")
            .content_hash,
        "hash-base"
    );
    assert_eq!(
        replay.scope_results()[0].status(),
        &OverlayScopeReplayStatus::Untrusted
    );
    assert!(replay.scope_results()[0].output().is_none());
    assert_eq!(
        replay.scope_results()[0].last_healthy_hash(),
        replay.base().effective_hash()
    );
}

#[test]
fn generated_healthy_snapshots_are_deterministic_across_repeated_replay_and_input_order() {
    for seed in 0_u64..64 {
        let base = generated_base(seed);
        let mut system = document(OverlayScope::System, None);
        system.patches.push(patch(
            &format!("system-{seed}"),
            &format!("alpha-{seed}"),
            &format!("system-{seed}"),
        ));
        system.learn_blocks.push(guidance(
            &format!("system-guidance-{seed}"),
            &format!("System guidance {seed}"),
        ));
        system.files.push(file_at(
            &format!("system-file-{seed}"),
            "references/shared.md",
            &format!("system-hash-{seed}"),
        ));

        let mut user = document(OverlayScope::User, None);
        user.patches.push(patch(
            &format!("user-{seed}"),
            &format!("beta-{seed}"),
            &format!("user-{seed}"),
        ));
        user.files.push(file_at(
            &format!("user-file-{seed}"),
            "templates/report.md",
            &format!("user-hash-{seed}"),
        ));

        let mut project = document(OverlayScope::Project, Some(WORKSPACE));
        project.patches.push(patch(
            &format!("project-{seed}"),
            &format!("gamma-{seed}"),
            &format!("project-{seed}"),
        ));
        project.files.push(file_at(
            &format!("project-file-{seed}"),
            "references/shared.md",
            &format!("project-hash-{seed}"),
        ));

        let target_revision = 1 + seed % 4;
        advance_to_revision(&mut system, target_revision);
        advance_to_revision(&mut user, target_revision);
        advance_to_revision(&mut project, target_revision);

        let resources = generated_resources(seed);
        let mut reversed_resources = resources.clone();
        reversed_resources.reverse();
        let ordered = replay_overlay_scope_chain(
            &base,
            &resources,
            &[
                OverlayScopeReplayInput::verified(&system),
                OverlayScopeReplayInput::verified(&user),
                OverlayScopeReplayInput::verified(&project),
            ],
            Some(WORKSPACE),
            4,
        );
        let permuted = replay_overlay_scope_chain(
            &base,
            &reversed_resources,
            &[
                OverlayScopeReplayInput::verified(&project),
                OverlayScopeReplayInput::verified(&system),
                OverlayScopeReplayInput::verified(&user),
            ],
            Some(WORKSPACE),
            4,
        );

        assert_eq!(
            ordered.effective().instructions(),
            permuted.effective().instructions()
        );
        assert_eq!(
            ordered.effective().resources(),
            permuted.effective().resources()
        );
        assert_eq!(
            ordered.effective().effective_hash(),
            permuted.effective().effective_hash()
        );
        assert_eq!(ordered.scope_results(), permuted.scope_results());
        assert_eq!(ordered, permuted, "determinism failed for seed {seed}");
    }
}

#[test]
fn generated_conflicts_are_deterministic_and_keep_the_same_last_healthy_snapshot() {
    for seed in 0_u64..64 {
        let base = generated_base(seed);
        let mut system = document(OverlayScope::System, None);
        system.patches.push(patch(
            &format!("system-{seed}"),
            &format!("alpha-{seed}"),
            &format!("system-{seed}"),
        ));
        let mut user = document(OverlayScope::User, None);
        user.patches.push(patch(
            &format!("missing-{seed}"),
            &format!("absent-{seed}"),
            "never-applied",
        ));
        let mut project = document(OverlayScope::Project, Some(WORKSPACE));
        project.patches.push(patch(
            &format!("project-{seed}"),
            &format!("gamma-{seed}"),
            &format!("project-{seed}"),
        ));
        let revision = 1 + seed % 3;
        advance_to_revision(&mut system, revision);
        advance_to_revision(&mut user, revision);
        advance_to_revision(&mut project, revision);

        let first = replay_overlay_scope_chain(
            &base,
            &generated_resources(seed),
            &[
                OverlayScopeReplayInput::verified(&system),
                OverlayScopeReplayInput::verified(&user),
                OverlayScopeReplayInput::verified(&project),
            ],
            Some(WORKSPACE),
            2,
        );
        let second = replay_overlay_scope_chain(
            &base,
            &generated_resources(seed),
            &[
                OverlayScopeReplayInput::verified(&project),
                OverlayScopeReplayInput::verified(&user),
                OverlayScopeReplayInput::verified(&system),
            ],
            Some(WORKSPACE),
            2,
        );

        assert!(matches!(
            first.scope_results()[1].status(),
            OverlayScopeReplayStatus::Conflict(conflict)
                if conflict.mutation_id() == Some(format!("missing-{seed}").as_str())
        ));
        assert_eq!(
            first.scope_results()[1].status(),
            second.scope_results()[1].status()
        );
        assert_eq!(first.effective(), second.effective());
        assert_eq!(first, second, "conflict determinism failed for seed {seed}");
    }
}

#[test]
fn integrity_failure_preserves_the_last_healthy_hash_and_blocks_project_replay() {
    let mut system = document(OverlayScope::System, None);
    system.patches.push(patch("system-patch", "base", "system"));

    let mut user = document(OverlayScope::User, None);
    user.patches.push(patch("user-patch", "system", "user"));
    user.files.push(file("user-file", "hash-user"));

    let mut project = document(OverlayScope::Project, Some(WORKSPACE));
    project
        .patches
        .push(patch("project-patch", "user", "project"));

    let replay = replay_overlay_scope_chain(
        "base",
        &base_resources(),
        &[
            OverlayScopeReplayInput::verified(&system),
            OverlayScopeReplayInput::integrity_failure(
                &user,
                OverlayIntegrityFailure::PayloadHashMismatch {
                    mutation_id: "user-file".to_string(),
                },
            ),
            OverlayScopeReplayInput::verified(&project),
        ],
        Some(WORKSPACE),
        4,
    );

    let healthy_hash = replay.scope_results()[0]
        .output_hash()
        .expect("System output hash");
    assert_eq!(replay.effective().effective_hash(), healthy_hash);
    assert_eq!(
        replay.scope_results()[1].status(),
        &OverlayScopeReplayStatus::IntegrityFailure(OverlayIntegrityFailure::PayloadHashMismatch {
            mutation_id: "user-file".to_string(),
        })
    );
    assert_eq!(replay.scope_results()[1].last_healthy_hash(), healthy_hash);
    assert_eq!(
        replay.scope_results()[2].status(),
        &OverlayScopeReplayStatus::Blocked {
            failed_scope: OverlayScope::User,
        }
    );
    assert_eq!(
        replay
            .effective()
            .resource("references/shared.md")
            .expect("base resource")
            .content_hash,
        "hash-base"
    );
}

fn document(scope: OverlayScope, workspace: Option<&str>) -> OverlayDocument {
    OverlayDocument::new(
        SkillId::parse("developer").expect("skill id"),
        scope,
        workspace,
        OverlayBaseWitness::new("system:developer", "instruction-hash", "package-hash")
            .expect("base witness"),
        OverlayTrust::trusted_local(1),
        "2026-08-11T00:00:00Z",
    )
    .expect("Overlay document")
}

fn patch(id: &str, old_string: &str, new_string: &str) -> OverlayPatch {
    OverlayPatch::new(
        id,
        old_string,
        new_string,
        false,
        "instruction-hash",
        "2026-08-11T00:00:00Z",
    )
    .expect("patch")
}

fn file(id: &str, hash: &str) -> OverlayFile {
    file_at(id, "references/shared.md", hash)
}

fn file_at(id: &str, logical_path: &str, hash: &str) -> OverlayFile {
    OverlayFile::new(
        id,
        logical_path,
        "text/markdown",
        12,
        hash,
        &format!("payloads/{hash}"),
        "2026-08-11T00:00:00Z",
    )
    .expect("file")
}

fn guidance(id: &str, value: &str) -> OverlayLearnBlock {
    OverlayLearnBlock::new(id, value, "2026-08-11T00:00:00Z").expect("guidance")
}

fn advance_to_revision(document: &mut OverlayDocument, target: u64) {
    while document.revision() < target {
        let next = document.revision() + 1;
        document
            .advance_revision(
                &format!("prior-revision-hash-{next}"),
                "2026-08-11T00:00:01Z",
            )
            .expect("advance revision");
    }
}

fn generated_base(seed: u64) -> String {
    let newline = if seed.is_multiple_of(2) { "\n" } else { "\r\n" };
    let suffix = if seed.is_multiple_of(3) {
        "中文"
    } else {
        "ASCII"
    };
    format!("alpha-{seed}{newline}beta-{seed}{newline}gamma-{seed}{newline}{suffix}-{seed}")
}

fn generated_resources(seed: u64) -> Vec<BaseSkillResource> {
    vec![
        BaseSkillResource {
            logical_path: "references/shared.md".to_string(),
            media_type: "text/markdown".to_string(),
            size_bytes: seed + 1,
            content_hash: format!("base-shared-{seed}"),
            source_layer: SkillLayer::System,
        },
        BaseSkillResource {
            logical_path: "assets/diagram.png".to_string(),
            media_type: "image/png".to_string(),
            size_bytes: seed + 10,
            content_hash: format!("base-image-{seed}"),
            source_layer: SkillLayer::System,
        },
    ]
}

fn base_resources() -> Vec<BaseSkillResource> {
    vec![BaseSkillResource {
        logical_path: "references/shared.md".to_string(),
        media_type: "text/markdown".to_string(),
        size_bytes: 9,
        content_hash: "hash-base".to_string(),
        source_layer: SkillLayer::System,
    }]
}
