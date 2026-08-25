//! The fixture assembly must not exist in a build without `desktop-e2e`.
//!
//! These run in the default feature set, which is the configuration that ships. They assert on
//! source rather than on a binary because the compiler has already removed the code: the modules
//! are `#[cfg(feature = "desktop-e2e")]`, so in this configuration there is nothing to search for.
//! What is worth pinning is that the gating itself has not been loosened.

use std::fs;
use std::path::{Path, PathBuf};

fn native_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn read(relative: &str) -> String {
    fs::read_to_string(native_root().join(relative)).expect("read a native source")
}

/// Every runtime switch the fixture assembly reads.
///
/// One list, iterated by every scan below. Searching for a single literal is how
/// `VANEHUB_LOCAL_MEDIA_E2E_OCR_SOURCE` -- the most reusable of the four, because it names a real
/// file -- stayed outside the whole-tree guard while the other three were covered.
const ACTIVATION_VARIABLES: [&str; 4] = [
    "VANEHUB_LOCAL_MEDIA_E2E_FIXTURES",
    "VANEHUB_LOCAL_MEDIA_E2E_SCENARIO_FILE",
    "VANEHUB_LOCAL_MEDIA_E2E_PYTHON_ROOT",
    "VANEHUB_LOCAL_MEDIA_E2E_OCR_SOURCE",
];

/// The only files allowed to name an activation variable.
///
/// The third is a test that spawns a real worker and has to put the scenario path into that child's
/// environment. It never reads the variable from its own environment, and it is a `#[cfg(test)]`
/// module, so nothing it contains reaches a shipped binary -- but it is listed here rather than
/// waved through by a path pattern, because an exception that is spelled out is one a reader can
/// audit.
const ACTIVATION_FILES: [&str; 3] = [
    "src/bootstrap/local_media_fixture_boundary_tests.rs",
    "src/contexts/local_media/infrastructure/fixtures/scenario.rs",
    "src/contexts/local_media/infrastructure/workers/shutdown_process_tests.rs",
];

/// Names that exist only because the fixture assembly exists.
const FIXTURE_TOKENS: [&str; 7] = [
    "VANEHUB_LOCAL_MEDIA_E2E_FIXTURES",
    "VANEHUB_LOCAL_MEDIA_E2E_SCENARIO_FILE",
    "VANEHUB_LOCAL_MEDIA_E2E_PYTHON_ROOT",
    "VANEHUB_LOCAL_MEDIA_E2E_OCR_SOURCE",
    "FixtureAudioCapture",
    "FixtureAudioPlayback",
    "FixtureAudioDeviceCatalog",
];

#[test]
fn the_fixture_module_is_gated_behind_the_feature() {
    let infrastructure = read("src/contexts/local_media/infrastructure/mod.rs");
    let gated = infrastructure
        .lines()
        .zip(infrastructure.lines().skip(1))
        .any(|(attribute, declaration)| {
            attribute.contains("cfg(feature = \"desktop-e2e\")")
                && declaration.contains("mod fixtures")
        });
    assert!(
        gated,
        "the fixtures module is not behind `#[cfg(feature = \"desktop-e2e\")]`"
    );
}

#[test]
fn no_fixture_name_appears_in_ungated_production_source() {
    let mut offenders = Vec::new();
    for path in native_sources() {
        let relative = relative_of(&path);
        // The fixture module itself and the files listed above are where the names belong.
        if relative.contains("local_media/infrastructure/fixtures/")
            || ACTIVATION_FILES.contains(&relative.as_str())
        {
            continue;
        }
        let source = fs::read_to_string(&path).expect("read a native source");
        for (index, line) in source.lines().enumerate() {
            let Some(token) = FIXTURE_TOKENS.iter().find(|token| line.contains(**token)) else {
                continue;
            };
            // A mention is acceptable only inside a region the feature gates.
            if !gated_region(&source, index) {
                offenders.push(format!("{relative}:{} ({token})", index + 1));
            }
        }
    }

    offenders.sort();
    assert!(
        offenders.is_empty(),
        "fixture names reachable from an ungated build:\n{}",
        offenders.join("\n")
    );
}

/// Whether the line sits in a region the `desktop-e2e` feature gates.
///
/// Two ways to qualify: an attribute directly above the statement, or an enclosing function whose
/// own attributes carry the cfg. Anything else counts as ungated, because a rule that guessed
/// generously here would stop being a boundary.
fn gated_region(source: &str, line_index: usize) -> bool {
    let lines: Vec<&str> = source.lines().collect();
    let cfg = "cfg(feature = \"desktop-e2e\")";

    // A statement-level attribute immediately above, skipping any interleaved attributes.
    let mut cursor = line_index;
    while cursor > 0 {
        let candidate = lines[cursor - 1].trim();
        if candidate.contains(cfg) {
            return true;
        }
        if !candidate.starts_with("#[") {
            break;
        }
        cursor -= 1;
    }

    // Otherwise the enclosing function: find the nearest preceding signature, then read the
    // attribute block above it.
    let mut signature = None;
    for index in (0..line_index).rev() {
        let candidate = lines[index].trim_start();
        if candidate.starts_with("fn ")
            || candidate.starts_with("pub fn ")
            || candidate.starts_with("pub(crate) fn ")
            || candidate.starts_with("pub(super) fn ")
        {
            signature = Some(index);
            break;
        }
    }
    let Some(signature) = signature else {
        return false;
    };
    for index in (0..signature).rev() {
        let candidate = lines[index].trim();
        if candidate.contains(cfg) {
            return true;
        }
        if !candidate.starts_with("#[") && !candidate.starts_with("///") {
            return false;
        }
    }
    false
}

#[test]
fn the_python_fixture_packages_are_not_inside_the_bundled_bridge() {
    let bridge = native_root().join("resources").join("local-media-worker");
    for package in ["paddleocr", "faster_whisper", "sherpa_onnx"] {
        assert!(
            !bridge.join(package).exists(),
            "{package} is inside the bundled bridge and would ship"
        );
    }
    // They live under `tests/`, which no bundle glob covers.
    let fixtures = native_root()
        .parent()
        .expect("repository root")
        .join("tests/desktop/fixtures/local-media-python");
    assert!(fixtures.is_dir(), "the Python fixture root is missing");
    for package in ["paddleocr", "faster_whisper", "sherpa_onnx"] {
        assert!(fixtures.join(package).is_dir(), "{package} fixture missing");
    }
}

#[test]
fn the_bundle_resource_list_excludes_the_fixture_python() {
    let configuration = read("tauri.conf.json");
    assert!(!configuration.contains("local-media-python"));
    assert!(!configuration.contains("tests/desktop"));
    // The bundled glob stays scoped to the real worker package.
    assert!(configuration.contains("resources/local-media-worker/vane_local_media_worker/**/*"));
}

#[test]
fn every_fixture_command_registration_is_feature_gated() {
    let registry = read("src/commands/supplemental_registry.rs");
    let lines: Vec<&str> = registry.lines().collect();
    let mut ungated = Vec::new();
    for (index, line) in lines.iter().enumerate() {
        if !line.to_lowercase().contains("fixture") {
            continue;
        }
        // A fixture command may be registered, but only directly beneath the feature attribute.
        // Without that, a default build would expose a test-only entry point.
        let gated = index > 0 && lines[index - 1].contains("cfg(feature = \"desktop-e2e\")");
        if !gated {
            ungated.push(format!("supplemental_registry.rs:{}", index + 1));
        }
    }

    assert!(
        ungated.is_empty(),
        "ungated fixture command registration:
{}",
        ungated.join(
            "
"
        )
    );
    for token in FIXTURE_TOKENS {
        assert!(
            !registry.contains(token),
            "the command registry mentions `{token}`"
        );
    }
}

#[test]
fn the_fixture_command_is_routed_only_behind_the_feature() {
    // `is_command` is the name-based router. The fixture name may appear, but only under the
    // feature gate: an ungated arm would make a default build answer a test-only command.
    let registry = read("src/commands/supplemental_registry.rs");
    let router_start = registry
        .find("fn is_command")
        .expect("the registry declares a name-based router");
    let router = &registry[router_start..];
    let lines: Vec<&str> = router.lines().collect();
    let mut seen = false;
    for (index, line) in lines.iter().enumerate() {
        if !line.contains("fixture_local_media_ocr_source") {
            continue;
        }
        seen = true;
        let gated = lines[..index]
            .iter()
            .rev()
            .take(3)
            .any(|above| above.contains("cfg(feature = \"desktop-e2e\")"));
        assert!(
            gated,
            "the fixture command is routed without the feature gate"
        );
    }
    assert!(seen, "the fixture command is registered but never routed");
}

#[test]
fn every_activation_variable_is_read_only_from_the_gated_module() {
    // The module that defines the switches. Requiring it in every reader set is what keeps this
    // from passing vacuously if a variable is renamed or dropped on one side only.
    let definition = "src/contexts/local_media/infrastructure/fixtures/scenario.rs";
    for variable in ACTIVATION_VARIABLES {
        let mut readers = Vec::new();
        for path in native_sources() {
            let source = fs::read_to_string(&path).expect("read a native source");
            if source.contains(variable) {
                readers.push(relative_of(&path));
            }
        }
        readers.sort();
        assert!(
            readers.iter().any(|reader| reader == definition),
            "`{variable}` is not declared in the gated module"
        );
        let strays: Vec<&String> = readers
            .iter()
            .filter(|reader| !ACTIVATION_FILES.contains(&reader.as_str()))
            .collect();
        assert!(strays.is_empty(), "`{variable}` is named in {strays:?}");
    }
}

/// Every `.rs` file under the native crate's source tree.
fn native_sources() -> Vec<PathBuf> {
    let mut found = Vec::new();
    let mut stack = vec![native_root().join("src")];
    while let Some(directory) = stack.pop() {
        for entry in fs::read_dir(&directory).expect("enumerate native sources") {
            let path = entry.expect("read an entry").path();
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().and_then(|value| value.to_str()) == Some("rs") {
                found.push(path);
            }
        }
    }
    found
}

fn relative_of(path: &Path) -> String {
    path.strip_prefix(native_root())
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}
