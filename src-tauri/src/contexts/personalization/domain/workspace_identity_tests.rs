use super::scope::WorkspaceKind;
use super::workspace_identity::{normalize_local_root, LocalPathRules, WorkspaceIdentitySource};

/// The rules a Linux filesystem follows: neither spelling rule applies, so every distinct byte
/// sequence is a distinct directory.
const LINUX: LocalPathRules = LocalPathRules {
    fold_case: false,
    normalize_unicode: false,
};

/// Windows: folds case, keeps Unicode spellings apart.
const WINDOWS: LocalPathRules = LocalPathRules {
    fold_case: true,
    normalize_unicode: false,
};

/// macOS: folds case, and treats the composed and decomposed spellings of one name as one file.
const MACOS: LocalPathRules = LocalPathRules {
    fold_case: true,
    normalize_unicode: true,
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

fn key(source: &WorkspaceIdentitySource, rules: LocalPathRules) -> String {
    source
        .derive_key(rules)
        .expect("derive")
        .as_str()
        .to_string()
}

#[test]
fn a_stable_id_is_used_verbatim_rather_than_hashed() {
    // An id the workspace subsystem already assigns is always better than one derived here:
    // deriving one means two subsystems can disagree about what "the same workspace" means.
    let source = WorkspaceIdentitySource::StableId("proj_01K2ABCDEF".to_string());
    assert_eq!(key(&source, WINDOWS), "proj_01K2ABCDEF");
    assert_eq!(key(&source, LINUX), "proj_01K2ABCDEF");
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
            normalize_local_root(path, LINUX),
            "D:/work/app",
            "{path:?} should normalize to the same root"
        );
    }
}

#[test]
fn a_unc_root_stays_distinguishable_from_an_absolute_path() {
    assert_eq!(
        normalize_local_root(r"\\server\share\app", LINUX),
        "//server/share/app"
    );
    assert_eq!(
        normalize_local_root(r"\\?\UNC\server\share\app", LINUX),
        "//server/share/app"
    );
    assert_ne!(
        normalize_local_root(r"\\server\share", LINUX),
        normalize_local_root(r"\server\share", LINUX)
    );
}

#[test]
fn a_bare_root_keeps_its_separator() {
    assert_eq!(normalize_local_root("/", LINUX), "/");
    assert_eq!(normalize_local_root(r"C:\", LINUX), "C:");
}

#[test]
fn case_folding_is_a_parameter_so_the_rule_is_testable_everywhere() {
    // The key never leaves the machine that derived it, so following the local filesystem's
    // folding rule is what makes two spellings of one directory agree.
    assert_eq!(
        key(&local(r"D:\Work\App"), WINDOWS),
        key(&local(r"d:\work\app"), WINDOWS),
        "a case-folding filesystem must treat these as one workspace"
    );
    assert_ne!(
        key(&local(r"D:\Work\App"), LINUX),
        key(&local(r"d:\work\app"), LINUX),
        "a case-sensitive filesystem must keep them apart"
    );
    // The production selector is a fact about the platform, not a preference.
    assert_eq!(
        LocalPathRules::for_this_platform().fold_case,
        cfg!(any(target_os = "windows", target_os = "macos"))
    );
}

/// macOS stores one name in two spellings and opens both; Linux does not.
///
/// `cafe` with an acute accent written as U+00E9 and as `e` + U+0301 is the same directory on
/// macOS -- a path can arrive in either form depending on whether it came from a file dialog, a
/// shell, or git. Two keys for one directory would scope a workspace's memories to whichever
/// spelling happened to be recorded first, and the user would watch them vanish. On Linux they are
/// genuinely two files, so folding them there would merge two real directories into one scope.
#[test]
fn unicode_spellings_are_one_workspace_only_where_the_filesystem_says_so() {
    let composed = local("/Users/me/caf\u{e9}");
    let decomposed = local("/Users/me/cafe\u{301}");

    assert_eq!(
        key(&composed, MACOS),
        key(&decomposed, MACOS),
        "macOS opens both spellings as one directory, so they must be one workspace"
    );
    assert_ne!(
        key(&composed, LINUX),
        key(&decomposed, LINUX),
        "on Linux these are two files, and merging them would join two real directories"
    );
    assert_ne!(
        key(&composed, WINDOWS),
        key(&decomposed, WINDOWS),
        "NTFS does not normalize, so Windows must keep them apart too"
    );
}

/// Normalization runs before case folding, not after.
///
/// Lowercasing a decomposed name folds the base letter and leaves the combining mark untouched;
/// lowercasing the composed one produces a single precomposed lowercase character. Folding first
/// and normalizing second would therefore still yield two different strings for one directory.
#[test]
fn normalization_precedes_case_folding_so_a_mixed_case_accent_still_agrees() {
    assert_eq!(
        key(&local("/Users/me/CAF\u{c9}"), MACOS),
        key(&local("/Users/me/cafe\u{301}"), MACOS),
    );
}

/// The remote path keeps every spelling apart.
///
/// The far side's filesystem rules are not knowable from here. Applying this machine's -- folding
/// case, or normalizing Unicode -- would merge two directories that are distinct on the server.
#[test]
fn a_remote_path_is_never_normalized_by_this_machines_rules() {
    let composed = remote("build-box", 22, Some("dev"), "/srv/caf\u{e9}");
    let decomposed = remote("build-box", 22, Some("dev"), "/srv/cafe\u{301}");

    for rules in [LINUX, WINDOWS, MACOS] {
        assert_ne!(
            key(&composed, rules),
            key(&decomposed, rules),
            "a remote path must not take this machine's normalization rule"
        );
    }
}

/// The production selector reports what each platform's filesystem actually does.
#[test]
fn the_platform_rules_are_facts_about_the_platform() {
    let rules = LocalPathRules::for_this_platform();

    assert_eq!(
        rules.fold_case,
        cfg!(any(target_os = "windows", target_os = "macos"))
    );
    assert_eq!(rules.normalize_unicode, cfg!(target_os = "macos"));
}

#[test]
fn two_different_local_roots_do_not_collide() {
    assert_ne!(
        key(&local(r"D:\work\app"), LINUX),
        key(&local(r"D:\work\api"), LINUX)
    );
    // Length-prefixed hashing: `a` + `bc` must not collide with `ab` + `c`.
    assert_ne!(key(&local("/a/bc"), LINUX), key(&local("/ab/c"), LINUX));
}

#[test]
fn the_same_remote_path_on_different_hosts_never_shares_a_scope() {
    // The concrete leak this prevents: one project's memories surfacing in another because both
    // live at /srv/app.
    let left = remote("alpha.example", 22, Some("deploy"), "/srv/app");
    let right = remote("beta.example", 22, Some("deploy"), "/srv/app");
    assert_ne!(key(&left, LINUX), key(&right, LINUX));
}

#[test]
fn remote_connection_identity_includes_port_and_user() {
    let base = remote("alpha.example", 22, Some("deploy"), "/srv/app");
    assert_ne!(
        key(&base, LINUX),
        key(
            &remote("alpha.example", 2222, Some("deploy"), "/srv/app"),
            LINUX
        ),
        "a different port is a different connection"
    );
    assert_ne!(
        key(&base, LINUX),
        key(&remote("alpha.example", 22, Some("ci"), "/srv/app"), LINUX),
        "a different user is a different connection"
    );
    assert_ne!(
        key(&base, LINUX),
        key(&remote("alpha.example", 22, None, "/srv/app"), LINUX)
    );
}

#[test]
fn a_remote_host_folds_case_but_its_path_does_not() {
    // DNS names are case-insensitive; the remote filesystem's rules are not knowable from here, so
    // folding a remote path could merge distinct directories.
    assert_eq!(
        key(
            &remote("Alpha.Example", 22, Some("deploy"), "/srv/app"),
            LINUX
        ),
        key(
            &remote("alpha.example", 22, Some("deploy"), "/srv/app"),
            LINUX
        )
    );
    assert_ne!(
        key(
            &remote("alpha.example", 22, Some("deploy"), "/srv/App"),
            LINUX
        ),
        key(
            &remote("alpha.example", 22, Some("deploy"), "/srv/app"),
            LINUX
        )
    );
}

#[test]
fn a_local_and_a_remote_workspace_never_share_a_key() {
    assert_ne!(
        key(&local("/srv/app"), LINUX),
        key(&remote("alpha.example", 22, None, "/srv/app"), LINUX)
    );
}

#[test]
fn a_derived_key_carries_no_path_or_credential_material() {
    // The key appears in diagnostics and in the revision token. Anything recoverable from it would
    // be a leak, and an identity derived from a secret would change when the secret rotated.
    let source = remote("alpha.example", 22, Some("deploy"), "/srv/secret-project");
    let derived = key(&source, LINUX);

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
    let identity = source.resolve(LINUX).expect("resolve");
    assert_eq!(identity.display_path(), "deploy@alpha.example:/srv/app");
    assert_ne!(identity.key().as_str(), identity.display_path());
    assert_eq!(identity.kind(), WorkspaceKind::Remote);

    let local_identity = local(r"D:\work\app").resolve(LINUX).expect("resolve");
    assert_eq!(local_identity.display_path(), r"D:\work\app");
    assert_eq!(local_identity.kind(), WorkspaceKind::Local);
}

#[test]
fn derivation_is_stable_across_calls() {
    let source = local(r"D:\work\app");
    assert_eq!(key(&source, WINDOWS), key(&source, WINDOWS));
}
