use super::*;
use std::path::PathBuf;

fn no_overlay() -> WorkerEnvironmentOverlay {
    WorkerEnvironmentOverlay::default()
}

fn inherited() -> BTreeMap<String, String> {
    [
        ("PATH", "/usr/bin:/bin"),
        ("HOME", "/home/user"),
        ("SYSTEMROOT", "C:\\Windows"),
        ("WINDIR", "C:\\Windows"),
        ("TEMP", "/tmp"),
        ("LANG", "en_US.UTF-8"),
        ("HTTP_PROXY", "http://corp-proxy:8080"),
        ("HTTPS_PROXY", "http://corp-proxy:8080"),
        ("ALL_PROXY", "socks5://127.0.0.1:1080"),
        ("http_proxy", "http://corp-proxy:8080"),
        ("https_proxy", "http://corp-proxy:8080"),
        ("all_proxy", "socks5://127.0.0.1:1080"),
        ("ANTHROPIC_API_KEY", "sk-should-not-be-inherited"),
        ("AWS_SECRET_ACCESS_KEY", "also-not-inherited"),
        ("PYTHONPATH", "/somewhere/else"),
    ]
    .into_iter()
    .map(|(key, value)| (key.to_string(), value.to_string()))
    .collect()
}

fn built() -> BTreeMap<String, String> {
    worker_environment(
        Path::new("/opt/vanehub/resources/local-media-worker"),
        Path::new("/tmp/local-media"),
        &inherited(),
        &no_overlay(),
    )
}

#[test]
fn offline_flags_are_set() {
    let environment = built();
    assert_eq!(
        environment.get("HF_HUB_OFFLINE").map(String::as_str),
        Some("1")
    );
    assert_eq!(
        environment.get("TRANSFORMERS_OFFLINE").map(String::as_str),
        Some("1")
    );
}

#[test]
fn every_proxy_variable_is_absent_in_both_cases() {
    let environment = built();
    for name in [
        "HTTP_PROXY",
        "HTTPS_PROXY",
        "ALL_PROXY",
        "http_proxy",
        "https_proxy",
        "all_proxy",
        "NO_PROXY",
        "no_proxy",
    ] {
        assert!(
            !environment.contains_key(name),
            "{name} survived into the worker environment"
        );
    }
}

#[test]
fn the_environment_is_an_allowlist_not_a_denylist() {
    // A denylist would carry every provider credential the parent happens to hold. Only names on
    // the allowlist survive, so a new secret in the parent needs no change here to stay out.
    let environment = built();
    assert!(!environment.contains_key("ANTHROPIC_API_KEY"));
    assert!(!environment.contains_key("AWS_SECRET_ACCESS_KEY"));
}

#[test]
fn the_platform_essentials_are_carried_through() {
    let environment = built();
    assert_eq!(
        environment.get("PATH").map(String::as_str),
        Some("/usr/bin:/bin")
    );
    assert_eq!(
        environment.get("SYSTEMROOT").map(String::as_str),
        Some("C:\\Windows")
    );
    assert_eq!(
        environment.get("WINDIR").map(String::as_str),
        Some("C:\\Windows")
    );
    assert_eq!(
        environment.get("HOME").map(String::as_str),
        Some("/home/user")
    );
}

#[test]
fn pythonpath_is_replaced_rather_than_extended() {
    // Inheriting the parent's PYTHONPATH would let an unrelated entry shadow the bundled bridge.
    let environment = built();
    assert_eq!(
        environment.get("PYTHONPATH").map(String::as_str),
        Some("/opt/vanehub/resources/local-media-worker")
    );
}

#[test]
fn python_runs_unbuffered_without_user_site_or_bytecode() {
    let environment = built();
    assert_eq!(
        environment.get("PYTHONUNBUFFERED").map(String::as_str),
        Some("1")
    );
    assert_eq!(
        environment.get("PYTHONNOUSERSITE").map(String::as_str),
        Some("1")
    );
    assert_eq!(
        environment
            .get("PYTHONDONTWRITEBYTECODE")
            .map(String::as_str),
        Some("1")
    );
}

#[test]
fn the_media_root_is_published_so_the_worker_can_repeat_the_containment_check() {
    let environment = built();
    assert_eq!(
        environment
            .get("VANEHUB_LOCAL_MEDIA_ROOT")
            .map(String::as_str),
        Some("/tmp/local-media")
    );
}

#[test]
fn a_parent_with_no_variables_still_produces_a_usable_environment() {
    let environment = worker_environment(
        Path::new("/bridge"),
        Path::new("/tmp/local-media"),
        &BTreeMap::new(),
        &no_overlay(),
    );
    assert_eq!(
        environment.get("HF_HUB_OFFLINE").map(String::as_str),
        Some("1")
    );
    assert!(!environment.contains_key("PATH"));
}

#[test]
fn an_overlay_appends_after_the_bridge_and_never_displaces_it() {
    let overlay = WorkerEnvironmentOverlay {
        python_path_suffix: vec![PathBuf::from("/fixtures/python")],
        variables: BTreeMap::from([("VANEHUB_FIXTURE".to_string(), "1".to_string())]),
    };
    let environment = worker_environment(
        Path::new("/bridge"),
        Path::new("/tmp/local-media"),
        &inherited(),
        &overlay,
    );

    let python_path = environment
        .get("PYTHONPATH")
        .map(String::as_str)
        .unwrap_or_default();
    // The real worker package must still win: a fixture root ahead of the bridge could shadow
    // `vane_local_media_worker` itself, which would replace the very code under test.
    assert!(python_path.starts_with("/bridge"));
    assert!(python_path.contains("/fixtures/python"));
    assert_eq!(
        environment.get("VANEHUB_FIXTURE").map(String::as_str),
        Some("1")
    );
}

#[test]
fn an_overlay_cannot_widen_what_is_inherited_from_the_parent() {
    let overlay = WorkerEnvironmentOverlay {
        python_path_suffix: vec![PathBuf::from("/fixtures/python")],
        variables: BTreeMap::new(),
    };
    let environment = worker_environment(
        Path::new("/bridge"),
        Path::new("/tmp/local-media"),
        &inherited(),
        &overlay,
    );

    // The allowlist still decides. An overlay adds names; it does not open the parent environment.
    for leaked in ["ANTHROPIC_API_KEY", "AWS_SECRET_ACCESS_KEY", "all_proxy"] {
        assert!(!environment.contains_key(leaked), "{leaked} leaked");
    }
}
