use super::overlay_layout::{is_overlay_manifest_path, OverlayStorageLayout};
use crate::contexts::tooling::skills::application::OverlayKey;
use crate::contexts::tooling::skills::domain::{OverlayScope, SkillId};
use std::path::{Path, PathBuf};

fn key(scope: OverlayScope, workspace_identity: Option<&Path>) -> OverlayKey {
    OverlayKey {
        canonical_skill_id: SkillId::parse("code-review").expect("valid Skill id"),
        scope,
        workspace_identity: workspace_identity.map(|path| path.to_string_lossy().into_owned()),
    }
}

#[test]
fn system_layout_uses_the_home_overlay_root() {
    let home = Path::new("C:/Users/tester");
    let layout = OverlayStorageLayout::resolve(home, &key(OverlayScope::System, None))
        .expect("system layout");

    assert_eq!(
        layout.manifest_path,
        home.join(".vanehub/skill_overlays/code-review.json")
    );
    assert_eq!(
        layout.payload_root,
        home.join(".vanehub/skill_overlays/.payloads")
    );
    assert_eq!(
        layout.history_root,
        home.join(".vanehub/skill_overlays/history/code-review")
    );
}

#[test]
fn user_layout_separates_manifests_but_shares_home_payload_and_history_roots() {
    let home = Path::new("C:/Users/tester");
    let layout =
        OverlayStorageLayout::resolve(home, &key(OverlayScope::User, None)).expect("user layout");

    assert_eq!(
        layout.manifest_path,
        home.join(".vanehub/skill_overlays/user/code-review.json")
    );
    assert_eq!(
        layout.payload_root,
        home.join(".vanehub/skill_overlays/.payloads")
    );
    assert_eq!(
        layout.history_root,
        home.join(".vanehub/skill_overlays/history/code-review")
    );
}

#[test]
fn project_layout_is_anchored_to_the_canonical_workspace_identity() {
    let home = Path::new("C:/Users/tester");
    let workspace = Path::new("D:/work/canonical-project");
    let layout = OverlayStorageLayout::resolve(home, &key(OverlayScope::Project, Some(workspace)))
        .expect("project layout");

    let overlay_root = workspace.join(".vanehub/skills/.overlays");
    assert_eq!(layout.manifest_path, overlay_root.join("code-review.json"));
    assert_eq!(layout.payload_root, overlay_root.join(".payloads"));
    assert_eq!(
        layout.history_root,
        overlay_root.join("history/code-review")
    );
}

#[test]
fn project_layout_requires_a_canonical_workspace_identity() {
    let error = OverlayStorageLayout::resolve(
        Path::new("C:/Users/tester"),
        &key(OverlayScope::Project, None),
    )
    .expect_err("project layout without workspace must fail");

    assert_eq!(
        error.to_string(),
        "Project Overlay requires a canonical workspace identity"
    );
}

#[test]
fn manifest_discovery_excludes_reserved_and_nested_directories() {
    let root = PathBuf::from("C:/Users/tester/.vanehub/skill_overlays");

    assert!(is_overlay_manifest_path(
        &root,
        &root.join("code-review.json")
    ));
    for candidate in [
        root.join(".payloads/code-review.json"),
        root.join("history/code-review.json"),
        root.join("user/code-review.json"),
        root.join(".transactions/code-review.json"),
        root.join(".staging/code-review.json"),
    ] {
        assert!(
            !is_overlay_manifest_path(&root, &candidate),
            "reserved or nested path was accepted: {}",
            candidate.display()
        );
    }
}
