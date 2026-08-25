use super::server_discovery::{
    locate_in_directories, resolved_startup_arguments, DiscoveryAvailability, DiscoveryReason,
    NativeExecutableLocationPort, ServerDiscovery, SystemNativeExecutableLocator,
};
use crate::contexts::code_intelligence::domain::registry;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

#[derive(Default)]
struct FakeExecutableLocator {
    result: Option<PathBuf>,
    requests: Mutex<Vec<String>>,
}

impl NativeExecutableLocationPort for FakeExecutableLocator {
    fn locate(&self, executable_name: &str) -> Option<PathBuf> {
        self.requests
            .lock()
            .expect("request lock")
            .push(executable_name.to_string());
        self.result.clone()
    }
}

#[test]
fn the_registry_pins_supported_executables_and_stdio_arguments() {
    assert_eq!(registry::rust().executables, &["rust-analyzer"]);
    assert!(registry::rust().default_startup_arguments.is_empty());

    assert_eq!(
        registry::typescript().executables,
        &["typescript-language-server"]
    );
    assert_eq!(
        registry::typescript().default_startup_arguments,
        &["--stdio"]
    );
}

#[test]
fn configured_startup_arguments_replace_the_registry_default_including_when_empty() {
    // Replacement rather than append, and an empty list is a choice. Appending would make a
    // declared default such as `--stdio` impossible to remove, and treating empty as unset would
    // silently put it back.
    assert_eq!(
        resolved_startup_arguments(registry::typescript(), None),
        vec!["--stdio".to_owned()]
    );
    assert_eq!(
        resolved_startup_arguments(registry::typescript(), Some(&Vec::new())),
        Vec::<String>::new()
    );
    assert_eq!(
        resolved_startup_arguments(registry::typescript(), Some(&vec!["--lsp".to_owned()])),
        vec!["--lsp".to_owned()]
    );
}

#[test]
fn discovery_selects_the_first_candidate_executable_in_registry_order() {
    let executable = fixture_executable("rust-analyzer");
    let locator = Arc::new(FakeExecutableLocator {
        result: Some(executable.clone()),
        ..FakeExecutableLocator::default()
    });
    let discovery = ServerDiscovery::new(locator.clone());

    let result = discovery.discover(registry::rust(), None, None);

    assert_eq!(
        result.selected_executable_name(),
        Some(registry::rust().executables[0])
    );
    assert_eq!(
        locator.requests.lock().expect("request lock").as_slice(),
        &["rust-analyzer"]
    );
}

#[test]
fn discovery_uses_native_location_without_starting_the_server() {
    let executable = fixture_executable("rust-analyzer");
    let locator = Arc::new(FakeExecutableLocator {
        result: Some(executable.clone()),
        ..FakeExecutableLocator::default()
    });
    let discovery = ServerDiscovery::new(locator.clone());

    let result = discovery.discover(registry::rust(), None, None);

    assert_eq!(result.availability(), DiscoveryAvailability::Available);
    assert_eq!(result.language(), registry::rust());
    assert_eq!(result.executable(), Some(executable.as_path()));
    assert_eq!(
        locator.requests.lock().expect("request lock").as_slice(),
        &["rust-analyzer"]
    );
}

#[test]
fn native_location_scans_directories_and_returns_a_canonical_file() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let executable = directory.path().join(executable_file_name("rust-analyzer"));
    write_executable(&executable);
    let _system_boundary = SystemNativeExecutableLocator;

    let located = locate_in_directories([directory.path().to_path_buf()], "rust-analyzer");

    assert_eq!(
        located,
        Some(std::fs::canonicalize(executable).expect("canonical executable"))
    );
}

#[test]
fn missing_native_executable_reports_a_safe_unavailable_reason() {
    let locator = Arc::new(FakeExecutableLocator::default());
    let discovery = ServerDiscovery::new(locator);

    let result = discovery.discover(registry::typescript(), None, None);

    assert_eq!(result.availability(), DiscoveryAvailability::Unavailable);
    assert_eq!(result.reason(), Some(DiscoveryReason::ExecutableNotFound));
    assert!(result.executable().is_none());
    assert_eq!(result.arguments(), &["--stdio"]);
}

#[test]
fn missing_manual_override_never_falls_back_to_native_discovery() {
    let fallback = fixture_executable("fallback-rust-analyzer");
    let locator = Arc::new(FakeExecutableLocator {
        result: Some(fallback),
        ..FakeExecutableLocator::default()
    });
    let discovery = ServerDiscovery::new(locator.clone());
    let missing = absolute_missing_path("missing-rust-analyzer");

    let result = discovery.discover(registry::rust(), Some(&missing), None);

    assert_eq!(result.availability(), DiscoveryAvailability::Unavailable);
    assert_eq!(result.reason(), Some(DiscoveryReason::OverrideMissing));
    assert!(locator.requests.lock().expect("request lock").is_empty());
}

#[test]
fn invalid_manual_override_is_rejected_without_native_discovery() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let locator = Arc::new(FakeExecutableLocator {
        result: Some(fixture_executable("fallback-typescript-server")),
        ..FakeExecutableLocator::default()
    });
    let discovery = ServerDiscovery::new(locator.clone());

    let result = discovery.discover(registry::typescript(), Some(directory.path()), None);

    assert_eq!(result.availability(), DiscoveryAvailability::Unavailable);
    assert_eq!(
        result.reason(),
        Some(DiscoveryReason::OverrideNotExecutable)
    );
    assert!(locator.requests.lock().expect("request lock").is_empty());
}

fn fixture_executable(name: &str) -> PathBuf {
    let directory = tempfile::tempdir().expect("temporary directory").keep();
    let path = directory.join(executable_file_name(name));
    write_executable(&path);
    path
}

fn write_executable(path: &Path) {
    std::fs::write(path, b"fixture").expect("write executable fixture");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut permissions = std::fs::metadata(path).expect("metadata").permissions();
        permissions.set_mode(0o755);
        std::fs::set_permissions(path, permissions).expect("executable permissions");
    }
}

fn absolute_missing_path(name: &str) -> PathBuf {
    tempfile::tempdir()
        .expect("temporary directory")
        .keep()
        .join(executable_file_name(name))
}

fn executable_file_name(name: &str) -> String {
    if cfg!(windows) {
        format!("{name}.exe")
    } else {
        name.to_string()
    }
}
