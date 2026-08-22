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

/// Names that exist only because the fixture assembly exists.
const FIXTURE_TOKENS: [&str; 6] = [
    "VANEHUB_LOCAL_MEDIA_E2E_FIXTURES",
    "VANEHUB_LOCAL_MEDIA_E2E_SCENARIO_FILE",
    "VANEHUB_LOCAL_MEDIA_E2E_PYTHON_ROOT",
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
    let mut stack = vec![native_root().join("src")];
    while let Some(directory) = stack.pop() {
        for entry in fs::read_dir(&directory).expect("enumerate native sources") {
            let path = entry.expect("read an entry").path();
            if path.is_dir() {
                stack.push(path);
                continue;
            }
            if path.extension().and_then(|value| value.to_str()) != Some("rs") {
                continue;
            }
            let relative = path
                .strip_prefix(native_root())
                .unwrap_or(&path)
                .to_string_lossy()
                .replace('\\', "/");
            // The fixture module itself and this test are the two places the names belong.
            if relative.contains("local_media/infrastructure/fixtures/")
                || relative.ends_with("local_media_fixture_boundary_tests.rs")
            {
                continue;
            }
            let source = fs::read_to_string(&path).expect("read a native source");
            for (index, line) in source.lines().enumerate() {
                if !FIXTURE_TOKENS.iter().any(|token| line.contains(token)) {
                    continue;
                }
                // A mention is acceptable only inside a region the feature gates.
                if !gated_region(&source, index) {
                    offenders.push(format!("{relative}:{}", index + 1));
                }
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
fn the_activation_is_read_only_from_the_gated_module() {
    let mut readers = Vec::new();
    let mut stack = vec![native_root().join("src")];
    while let Some(directory) = stack.pop() {
        for entry in fs::read_dir(&directory).expect("enumerate native sources") {
            let path = entry.expect("read an entry").path();
            if path.is_dir() {
                stack.push(path);
                continue;
            }
            if path.extension().and_then(|value| value.to_str()) != Some("rs") {
                continue;
            }
            let source = fs::read_to_string(&path).expect("read a native source");
            if source.contains("VANEHUB_LOCAL_MEDIA_E2E_FIXTURES") {
                readers.push(relative_of(&path));
            }
        }
    }

    readers.sort();
    // Exactly two: the module that defines the switch, and this test.
    assert_eq!(
        readers,
        vec![
            "src/bootstrap/local_media_fixture_boundary_tests.rs".to_string(),
            "src/contexts/local_media/infrastructure/fixtures/scenario.rs".to_string(),
        ]
    );
}

fn relative_of(path: &Path) -> String {
    path.strip_prefix(native_root())
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}
