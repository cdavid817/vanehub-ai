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

/// An install directory laid out the way the Java entry declares, with the given launcher names.
fn fixture_install(launchers: &[&str]) -> PathBuf {
    let directory = tempfile::tempdir().expect("temporary directory").keep();
    std::fs::create_dir_all(directory.join("plugins")).expect("plugins directory");
    std::fs::create_dir_all(directory.join(config_directory_name())).expect("config directory");
    for launcher in launchers {
        std::fs::write(directory.join("plugins").join(launcher), b"jar").expect("launcher");
    }
    directory
}

fn config_directory_name() -> &'static str {
    if cfg!(windows) {
        "config_win"
    } else if cfg!(target_os = "macos") {
        "config_mac"
    } else {
        "config_linux"
    }
}

fn java_discovery(interpreter: Option<PathBuf>) -> ServerDiscovery {
    ServerDiscovery::new(Arc::new(FakeExecutableLocator {
        result: interpreter,
        requests: Mutex::default(),
    }))
}

#[test]
fn an_interpreter_shaped_language_without_its_runtime_reports_the_prerequisite() {
    let discovery = java_discovery(None);

    let result = discovery.discover(registry::java(), Some(&fixture_install(&[])), None);

    // "Install a JDK" is a different action from every other reason here, so it is its own reason
    // rather than a generic missing executable.
    assert_eq!(result.availability(), DiscoveryAvailability::Unavailable);
    assert_eq!(result.reason(), Some(DiscoveryReason::PrerequisiteMissing));
}

#[test]
fn an_interpreter_shaped_language_without_an_install_directory_says_so() {
    let discovery = java_discovery(Some(fixture_executable("java")));

    let result = discovery.discover(registry::java(), None, None);

    // Not `ExecutableNotFound`: the server is a directory, so there is nothing the executable
    // search path could have been asked for.
    assert_eq!(
        result.reason(),
        Some(DiscoveryReason::InstallDirectoryNotSet)
    );
}

#[test]
fn the_prerequisite_is_reported_before_the_missing_directory() {
    // A user with neither is told about the JDK, because that is the one they hit first anyway.
    let discovery = java_discovery(None);

    let result = discovery.discover(registry::java(), None, None);

    assert_eq!(result.reason(), Some(DiscoveryReason::PrerequisiteMissing));
}

#[test]
fn an_install_directory_without_a_launcher_is_distinct_from_a_missing_one() {
    let discovery = java_discovery(Some(fixture_executable("java")));
    let empty = fixture_install(&[]);

    assert_eq!(
        discovery
            .discover(registry::java(), Some(&empty), None)
            .reason(),
        Some(DiscoveryReason::LauncherNotFound)
    );
    assert_eq!(
        discovery
            .discover(registry::java(), Some(&empty.join("absent")), None)
            .reason(),
        Some(DiscoveryReason::OverrideMissing)
    );
}

#[test]
fn several_launchers_are_refused_rather_than_chosen() {
    let discovery = java_discovery(Some(fixture_executable("java")));
    let ambiguous = fixture_install(&[
        "org.eclipse.equinox.launcher_1.6.500.jar",
        "org.eclipse.equinox.launcher_1.7.0.jar",
    ]);

    let result = discovery.discover(registry::java(), Some(&ambiguous), None);

    // Picking the newest would start a server whose version the settings page does not name.
    assert_eq!(result.reason(), Some(DiscoveryReason::AmbiguousInstall));
    assert_eq!(result.executable(), None);
}

#[test]
fn a_resolved_install_reports_the_launcher_rather_than_the_directory() {
    let interpreter = fixture_executable("java");
    let discovery = java_discovery(Some(interpreter.clone()));
    let install = fixture_install(&["org.eclipse.equinox.launcher_1.6.500.jar"]);

    let result = discovery.discover(registry::java(), Some(&install), None);

    assert_eq!(result.availability(), DiscoveryAvailability::Available);
    // The executable is the JVM; the server is the launcher, and a reader needs the second to tell
    // which version will run.
    assert_eq!(result.executable(), Some(interpreter.as_path()));
    assert_eq!(
        result
            .resolved_launcher()
            .and_then(|path| path.file_name())
            .and_then(|name| name.to_str()),
        Some("org.eclipse.equinox.launcher_1.6.500.jar")
    );
}

#[test]
fn a_launcher_outside_the_declared_directory_is_not_found() {
    let discovery = java_discovery(Some(fixture_executable("java")));
    let install = fixture_install(&[]);
    // One level deeper than the entry declares. Matching it would start a server from a directory
    // that only looks like the layout the entry describes.
    std::fs::create_dir_all(install.join("plugins/nested")).expect("nested");
    std::fs::write(
        install.join("plugins/nested/org.eclipse.equinox.launcher_1.6.500.jar"),
        b"jar",
    )
    .expect("nested launcher");

    assert_eq!(
        discovery
            .discover(registry::java(), Some(&install), None)
            .reason(),
        Some(DiscoveryReason::LauncherNotFound)
    );
}

#[test]
fn a_managed_install_is_used_when_the_user_set_no_override() {
    let discovery = java_discovery(Some(fixture_executable("java")));
    let managed = fixture_install(&["org.eclipse.equinox.launcher_1.7.0.jar"]);

    let result =
        discovery.discover_with_managed_install(registry::java(), None, None, Some(&managed));

    assert_eq!(result.availability(), DiscoveryAvailability::Available);
    assert!(result
        .resolved_launcher()
        .is_some_and(|path| path.starts_with(&managed)));
}

#[test]
fn an_override_wins_over_a_managed_install_that_also_exists() {
    let discovery = java_discovery(Some(fixture_executable("java")));
    let managed = fixture_install(&["org.eclipse.equinox.launcher_1.7.0.jar"]);
    let chosen = fixture_install(&["org.eclipse.equinox.launcher_1.6.500.jar"]);

    let result = discovery.discover_with_managed_install(
        registry::java(),
        Some(&chosen),
        None,
        Some(&managed),
    );

    // Installing through VaneHub must not silently retarget a user who already pointed somewhere.
    // Both are on disk here, so a wrong precedence would start a server rather than fail visibly.
    assert!(result
        .resolved_launcher()
        .is_some_and(|path| path.starts_with(&chosen)));
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
