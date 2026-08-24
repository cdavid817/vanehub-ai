use super::scope::WorkspaceKind;
use super::workspace_identity::{
    local_paths_fold_case, normalize_local_root, WorkspaceIdentitySource,
};

fn local(path: &str) -> WorkspaceIdentitySource {
    WorkspaceIdentitySource::LocalRoot {
        path: path.to_string(),
    }
}

fn remote(host: &str, port: u16, user: Option<&str>, path: &str) -> WorkspaceIdentitySource {
    WorkspaceIdentitySource::Remote {
        host: host.to_string(),
        port,
        user: user.map(str::to_string),
        path: path.to_string(),
    }
}

fn key(source: &WorkspaceIdentitySource, fold: bool) -> String {
    source
        .derive_key(fold)
        .expect("derive")
        .as_str()
        .to_string()
}

#[test]
fn a_stable_id_is_used_verbatim_rather_than_hashed() {
    // An id the workspace subsystem already assigns is always better than one derived here:
    // deriving one means two subsystems can disagree about what "the same workspace" means.
    let source = WorkspaceIdentitySource::StableId("proj_01K2ABCDEF".to_string());
    assert_eq!(key(&source, true), "proj_01K2ABCDEF");
    assert_eq!(key(&source, false), "proj_01K2ABCDEF");
}

#[test]
fn normalization_is_deterministic_and_filesystem_free() {
    // Separator, repeat, and trailing-separator differences all describe one directory.
    for path in [
        r"D:\work\app",
        r"D:/work/app",
        r"D:\work\app\",
        r"D:\\work\\\app",
        r"\\?\D:\work\app",
        "  D:\\work\\app  ",
    ] {
        assert_eq!(
            normalize_local_root(path, false),
            "D:/work/app",
            "{path:?} should normalize to the same root"
        );
    }
}

#[test]
fn a_unc_root_stays_distinguishable_from_an_absolute_path() {
    assert_eq!(
        normalize_local_root(r"\\server\share\app", false),
        "//server/share/app"
    );
    assert_eq!(
        normalize_local_root(r"\\?\UNC\server\share\app", false),
        "//server/share/app"
    );
    assert_ne!(
        normalize_local_root(r"\\server\share", false),
        normalize_local_root(r"\server\share", false)
    );
}

#[test]
fn a_bare_root_keeps_its_separator() {
    assert_eq!(normalize_local_root("/", false), "/");
    assert_eq!(normalize_local_root(r"C:\", false), "C:");
}

#[test]
fn case_folding_is_a_parameter_so_the_rule_is_testable_everywhere() {
    // The key never leaves the machine that derived it, so following the local filesystem's
    // folding rule is what makes two spellings of one directory agree.
    assert_eq!(
        key(&local(r"D:\Work\App"), true),
        key(&local(r"d:\work\app"), true),
        "a case-folding filesystem must treat these as one workspace"
    );
    assert_ne!(
        key(&local(r"D:\Work\App"), false),
        key(&local(r"d:\work\app"), false),
        "a case-sensitive filesystem must keep them apart"
    );
    // The production selector is a fact about the platform, not a preference.
    assert_eq!(
        local_paths_fold_case(),
        cfg!(any(target_os = "windows", target_os = "macos"))
    );
}

#[test]
fn two_different_local_roots_do_not_collide() {
    assert_ne!(
        key(&local(r"D:\work\app"), false),
        key(&local(r"D:\work\api"), false)
    );
    // Length-prefixed hashing: `a` + `bc` must not collide with `ab` + `c`.
    assert_ne!(key(&local("/a/bc"), false), key(&local("/ab/c"), false));
}

#[test]
fn the_same_remote_path_on_different_hosts_never_shares_a_scope() {
    // The concrete leak this prevents: one project's memories surfacing in another because both
    // live at /srv/app.
    let left = remote("alpha.example", 22, Some("deploy"), "/srv/app");
    let right = remote("beta.example", 22, Some("deploy"), "/srv/app");
    assert_ne!(key(&left, false), key(&right, false));
}

#[test]
fn remote_connection_identity_includes_port_and_user() {
    let base = remote("alpha.example", 22, Some("deploy"), "/srv/app");
    assert_ne!(
        key(&base, false),
        key(
            &remote("alpha.example", 2222, Some("deploy"), "/srv/app"),
            false
        ),
        "a different port is a different connection"
    );
    assert_ne!(
        key(&base, false),
        key(&remote("alpha.example", 22, Some("ci"), "/srv/app"), false),
        "a different user is a different connection"
    );
    assert_ne!(
        key(&base, false),
        key(&remote("alpha.example", 22, None, "/srv/app"), false)
    );
}

#[test]
fn a_remote_host_folds_case_but_its_path_does_not() {
    // DNS names are case-insensitive; the remote filesystem's rules are not knowable from here, so
    // folding a remote path could merge distinct directories.
    assert_eq!(
        key(
            &remote("Alpha.Example", 22, Some("deploy"), "/srv/app"),
            false
        ),
        key(
            &remote("alpha.example", 22, Some("deploy"), "/srv/app"),
            false
        )
    );
    assert_ne!(
        key(
            &remote("alpha.example", 22, Some("deploy"), "/srv/App"),
            false
        ),
        key(
            &remote("alpha.example", 22, Some("deploy"), "/srv/app"),
            false
        )
    );
}

#[test]
fn a_local_and_a_remote_workspace_never_share_a_key() {
    assert_ne!(
        key(&local("/srv/app"), false),
        key(&remote("alpha.example", 22, None, "/srv/app"), false)
    );
}

#[test]
fn a_derived_key_carries_no_path_or_credential_material() {
    // The key appears in diagnostics and in the revision token. Anything recoverable from it would
    // be a leak, and an identity derived from a secret would change when the secret rotated.
    let source = remote("alpha.example", 22, Some("deploy"), "/srv/secret-project");
    let derived = key(&source, false);

    assert!(derived.starts_with("ws_"));
    assert_eq!(derived.len(), "ws_".len() + 32);
    for fragment in ["alpha", "example", "deploy", "srv", "secret", "project"] {
        assert!(
            !derived.contains(fragment),
            "{fragment:?} must not be recoverable from {derived}"
        );
    }
    assert!(derived
        .trim_start_matches("ws_")
        .chars()
        .all(|character| character.is_ascii_hexdigit()));
}

#[test]
fn the_display_path_is_never_the_identity() {
    // Renaming a label must not move a workspace's memories, and two workspaces may share a label.
    let source = remote("alpha.example", 22, Some("deploy"), "/srv/app");
    let identity = source.resolve(false).expect("resolve");
    assert_eq!(identity.display_path(), "deploy@alpha.example:/srv/app");
    assert_ne!(identity.key().as_str(), identity.display_path());
    assert_eq!(identity.kind(), WorkspaceKind::Remote);

    let local_identity = local(r"D:\work\app").resolve(false).expect("resolve");
    assert_eq!(local_identity.display_path(), r"D:\work\app");
    assert_eq!(local_identity.kind(), WorkspaceKind::Local);
}

#[test]
fn derivation_is_stable_across_calls() {
    let source = local(r"D:\work\app");
    assert_eq!(key(&source, true), key(&source, true));
}
