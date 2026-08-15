use super::{
    merge_overlay_resources, BaseSkillResource, EffectiveResourceSource, OverlayFile, OverlayScope,
    ScopedOverlayFiles, SkillLayer,
};

fn base_resource(path: &str, media_type: &str, hash: &str) -> BaseSkillResource {
    BaseSkillResource {
        logical_path: path.to_string(),
        media_type: media_type.to_string(),
        size_bytes: 10,
        content_hash: hash.to_string(),
        source_layer: SkillLayer::System,
    }
}

fn overlay_file(id: &str, path: &str, media_type: &str, hash: &str) -> OverlayFile {
    OverlayFile::new(
        id,
        path,
        media_type,
        12,
        hash,
        &format!("sha256/{hash}"),
        "2026-08-11T10:00:00Z",
    )
    .expect("Overlay file")
}

#[test]
fn overlay_resources_follow_system_user_project_precedence_and_track_shadows() {
    let base = [
        base_resource("references/team.md", "text/markdown", "base-hash"),
        base_resource("references/base-only.md", "text/markdown", "base-only"),
    ];
    let system = [overlay_file(
        "system-file",
        "references/team.md",
        "text/plain",
        "system-hash",
    )];
    let user = [overlay_file(
        "user-file",
        "references/team.md",
        "text/markdown",
        "user-hash",
    )];
    let project = [overlay_file(
        "project-file",
        "references/team.md",
        "text/markdown",
        "project-hash",
    )];

    let replay = merge_overlay_resources(
        &base,
        &[
            ScopedOverlayFiles::new(OverlayScope::Project, Some("D:/work"), &project),
            ScopedOverlayFiles::new(OverlayScope::User, None, &user),
            ScopedOverlayFiles::new(OverlayScope::System, None, &system),
        ],
        Some("D:/work"),
        8,
    );

    let effective = replay
        .entry("references/team.md")
        .expect("effective resource");
    assert_eq!(effective.content_hash, "project-hash");
    assert_eq!(effective.media_type, "text/markdown");
    assert_eq!(
        effective.source,
        EffectiveResourceSource::Overlay {
            scope: OverlayScope::Project,
            workspace_identity: Some("D:/work".to_string()),
            mutation_id: "project-file".to_string(),
            payload_ref: "sha256/project-hash".to_string(),
        }
    );
    assert_eq!(effective.shadowed.len(), 3);
    assert!(matches!(
        effective.shadowed[0].source,
        EffectiveResourceSource::Overlay {
            scope: OverlayScope::User,
            ..
        }
    ));
    assert!(matches!(
        effective.shadowed[1].source,
        EffectiveResourceSource::Overlay {
            scope: OverlayScope::System,
            ..
        }
    ));
    assert!(matches!(
        effective.shadowed[2].source,
        EffectiveResourceSource::Base {
            layer: SkillLayer::System
        }
    ));
    assert!(replay.entry("references/base-only.md").is_some());
}

#[test]
fn project_resources_are_isolated_to_the_active_canonical_workspace() {
    let base = [base_resource("references/team.md", "text/markdown", "base")];
    let project_a = [overlay_file(
        "project-a",
        "references/team.md",
        "text/markdown",
        "hash-a",
    )];
    let project_b = [overlay_file(
        "project-b",
        "references/team.md",
        "text/markdown",
        "hash-b",
    )];
    let scopes = [
        ScopedOverlayFiles::new(OverlayScope::Project, Some("D:/a"), &project_a),
        ScopedOverlayFiles::new(OverlayScope::Project, Some("D:/b"), &project_b),
    ];

    let a = merge_overlay_resources(&base, &scopes, Some("D:/a"), 8);
    assert_eq!(
        a.entry("references/team.md")
            .expect("workspace a")
            .content_hash,
        "hash-a"
    );
    let no_workspace = merge_overlay_resources(&base, &scopes, None, 8);
    assert_eq!(
        no_workspace
            .entry("references/team.md")
            .expect("base without workspace")
            .content_hash,
        "base"
    );
}

#[test]
fn effective_resource_preserves_the_winning_media_type() {
    let base = [base_resource("assets/logo.png", "image/png", "base")];
    let user = [overlay_file(
        "user-logo",
        "assets/logo.png",
        "image/png",
        "overlay",
    )];
    let replay = merge_overlay_resources(
        &base,
        &[ScopedOverlayFiles::new(OverlayScope::User, None, &user)],
        None,
        8,
    );

    let logo = replay.entry("assets/logo.png").expect("logo");
    assert_eq!(logo.media_type, "image/png");
    assert_eq!(logo.size_bytes, 12);
}

#[test]
fn disabled_and_reverted_files_do_not_shadow_healthy_resources() {
    let base = [base_resource("references/team.md", "text/markdown", "base")];
    let mut disabled = overlay_file("disabled", "references/team.md", "text/plain", "disabled");
    disabled
        .disable("2026-08-11T11:00:00Z")
        .expect("disable file");
    let mut reverted = overlay_file("reverted", "references/team.md", "text/plain", "reverted");
    reverted
        .revert("2026-08-11T11:00:00Z")
        .expect("revert file");
    let files = [disabled, reverted];

    let replay = merge_overlay_resources(
        &base,
        &[ScopedOverlayFiles::new(OverlayScope::User, None, &files)],
        None,
        8,
    );
    let effective = replay.entry("references/team.md").expect("base resource");
    assert_eq!(effective.content_hash, "base");
    assert!(effective.shadowed.is_empty());
}

#[test]
fn shadow_summaries_are_bounded_and_report_truncation() {
    let base = [base_resource("references/team.md", "text/markdown", "base")];
    let system = [overlay_file(
        "system",
        "references/team.md",
        "text/plain",
        "system",
    )];
    let user = [overlay_file(
        "user",
        "references/team.md",
        "text/plain",
        "user",
    )];
    let project = [overlay_file(
        "project",
        "references/team.md",
        "text/plain",
        "project",
    )];
    let replay = merge_overlay_resources(
        &base,
        &[
            ScopedOverlayFiles::new(OverlayScope::System, None, &system),
            ScopedOverlayFiles::new(OverlayScope::User, None, &user),
            ScopedOverlayFiles::new(OverlayScope::Project, Some("D:/work"), &project),
        ],
        Some("D:/work"),
        2,
    );

    let effective = replay
        .entry("references/team.md")
        .expect("effective resource");
    assert_eq!(effective.shadowed.len(), 2);
    assert!(effective.shadowed_truncated);
}

#[test]
fn a_persisted_overlay_row_cannot_shadow_or_introduce_executable_skill_tool_content() {
    // `OverlayFile` construction does not revalidate the logical path, so a row written before a
    // rule existed — or straight into storage — reaches assembly with a reserved path intact.
    let base = [
        base_resource("scripts/tools.json", "application/json", "shipped-manifest"),
        base_resource("references/team.md", "text/markdown", "base-hash"),
    ];
    let hostile = [
        overlay_file(
            "forged-manifest",
            "scripts/tools.json",
            "application/json",
            "forged-manifest",
        ),
        overlay_file(
            "forged-module",
            "scripts/modules/score.wasm",
            "application/wasm",
            "forged-module",
        ),
        overlay_file(
            "legitimate",
            "references/team.md",
            "text/markdown",
            "overlay-hash",
        ),
    ];

    let replay = merge_overlay_resources(
        &base,
        &[ScopedOverlayFiles::new(OverlayScope::User, None, &hostile)],
        None,
        8,
    );

    let manifest = replay
        .entry("scripts/tools.json")
        .expect("shipped manifest stays effective");
    assert_eq!(manifest.content_hash, "shipped-manifest");
    assert!(matches!(
        manifest.source,
        EffectiveResourceSource::Base { .. }
    ));
    assert!(manifest.shadowed.is_empty());
    assert!(replay.entry("scripts/modules/score.wasm").is_none());

    // A non-reserved Overlay file in the same batch is still applied.
    assert_eq!(
        replay
            .entry("references/team.md")
            .expect("overlay resource")
            .content_hash,
        "overlay-hash"
    );
}
