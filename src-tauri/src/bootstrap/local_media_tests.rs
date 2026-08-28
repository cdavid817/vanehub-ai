use super::*;

#[test]
fn the_development_tree_is_probed_before_any_bundle() {
    // A `cargo run` next to an installed copy must use the working tree; otherwise a bridge edit
    // appears to have no effect and the reason is invisible.
    let candidates = worker_bridge_candidates(Some(PathBuf::from("/bundle/resources")));
    assert!(candidates
        .first()
        .expect("at least one candidate")
        .ends_with("resources/local-media-worker"));
    assert!(candidates[0].is_absolute());
}

#[test]
fn the_tauri_up_directory_variant_is_probed() {
    // Tauri rewrites a `../`-relative bundled resource into `_up_/`. Probing only the plain path
    // works in development and silently fails in a packaged build.
    let candidates = worker_bridge_candidates(Some(PathBuf::from("/bundle/resources")));
    let rendered: Vec<String> = candidates
        .iter()
        .map(|path| path.to_string_lossy().replace('\\', "/"))
        .collect();
    assert!(rendered
        .iter()
        .any(|path| path.contains("_up_/resources/local-media-worker")));
    assert!(rendered
        .iter()
        .any(|path| path.ends_with("/bundle/resources/resources/local-media-worker")));
    assert!(rendered
        .iter()
        .any(|path| path.ends_with("/bundle/resources/local-media-worker")));
}

#[test]
fn a_build_with_no_resource_directory_still_has_the_development_candidate() {
    let candidates = worker_bridge_candidates(None);
    assert_eq!(candidates.len(), 1);
}

#[test]
fn the_development_candidate_resolves_against_this_checkout() {
    // The candidate list being well formed says nothing about whether the bridge is where the list
    // says it is. Resolution is what the supervisor actually calls, and a rename of either the
    // resource directory or the Python package makes every engine unavailable at runtime with no
    // compile-time signal at all.
    let resolved = resolve_worker_bridge_root(&worker_bridge_candidates(None))
        .expect("the development bridge must resolve from the repository tree");
    assert!(resolved
        .join("vane_local_media_worker")
        .join("__main__.py")
        .is_file());
}

#[test]
fn an_empty_bundle_directory_is_not_mistaken_for_a_packaged_bridge() {
    let empty = std::env::temp_dir().join("vanehub-empty-bridge-candidate");
    std::fs::create_dir_all(&empty).expect("create the empty candidate");

    // Accepting it would turn "the bridge was not packaged" into an import error at first use.
    assert!(resolve_worker_bridge_root(&[empty]).is_none());
}

#[test]
fn candidates_are_distinct() {
    let candidates = worker_bridge_candidates(Some(PathBuf::from("/bundle/resources")));
    let unique: std::collections::BTreeSet<&PathBuf> = candidates.iter().collect();
    assert_eq!(unique.len(), candidates.len());
}
