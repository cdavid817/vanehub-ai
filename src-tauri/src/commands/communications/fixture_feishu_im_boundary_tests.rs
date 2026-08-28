use std::fs;
use std::path::{Path, PathBuf};

const FEATURE_GATE: &str = "cfg(feature = \"desktop-e2e\")";
const ACTIVATION: &str = "VANEHUB_FEISHU_IM_FIXTURE";
const COMMANDS: [&str; 5] = [
    "fixture_feishu_im_setup",
    "fixture_feishu_im_inject",
    "fixture_feishu_im_set_fault",
    "fixture_feishu_im_ledger",
    "fixture_feishu_im_reset",
];

fn native_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn read(relative: &str) -> String {
    fs::read_to_string(native_root().join(relative)).expect("read native source")
}

fn native_sources(root: &Path) -> Vec<PathBuf> {
    let mut sources = Vec::new();
    for entry in fs::read_dir(root).expect("read native source directory") {
        let path = entry.expect("read source entry").path();
        if path.is_dir() {
            sources.extend(native_sources(&path));
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            sources.push(path);
        }
    }
    sources
}

#[test]
fn fixture_module_and_command_registrations_require_desktop_e2e() {
    let module = read("src/commands/communications/mod.rs");
    assert!(module
        .lines()
        .zip(module.lines().skip(1))
        .any(|(attribute, declaration)| attribute.contains(FEATURE_GATE)
            && declaration.contains("mod fixture_feishu_im")));

    let registry = read("src/commands/supplemental_registry.rs");
    let lines: Vec<&str> = registry.lines().collect();
    for command in COMMANDS {
        let occurrences: Vec<usize> = lines
            .iter()
            .enumerate()
            .filter_map(|(index, line)| line.contains(command).then_some(index))
            .collect();
        assert_eq!(
            occurrences.len(),
            2,
            "unexpected registration count for {command}"
        );
        for index in occurrences {
            let gated = lines[..index]
                .iter()
                .rev()
                .take(8)
                .any(|line| line.contains(FEATURE_GATE));
            assert!(gated, "{command} is reachable without desktop-e2e");
        }
    }
}

#[test]
fn activation_switch_is_confined_to_test_only_native_source() {
    let fixture = native_root()
        .join("src")
        .join("contexts")
        .join("communications")
        .join("infrastructure")
        .join("feishu_fixture.rs");
    let boundary = native_root()
        .join("src")
        .join("commands")
        .join("communications")
        .join("fixture_feishu_im_boundary_tests.rs");
    let mut readers = native_sources(&native_root().join("src"))
        .into_iter()
        .filter(|path| fs::read_to_string(path).is_ok_and(|source| source.contains(ACTIVATION)))
        .collect::<Vec<_>>();
    readers.sort();
    let mut expected = vec![boundary, fixture];
    expected.sort();
    assert_eq!(readers, expected);
}

#[test]
fn production_permissions_do_not_expose_fixture_commands() {
    let permissions_root = native_root().join("capabilities");
    if !permissions_root.exists() {
        return;
    }
    for path in native_sources(&permissions_root) {
        let source = fs::read_to_string(&path).expect("read permission source");
        for command in COMMANDS {
            assert!(!source.contains(command), "{command} appears in {path:?}");
        }
    }
    for entry in fs::read_dir(&permissions_root).expect("read capabilities") {
        let path = entry.expect("read capability entry").path();
        if path
            .extension()
            .is_some_and(|extension| extension == "json")
        {
            let source = fs::read_to_string(&path).expect("read capability");
            for command in COMMANDS {
                assert!(!source.contains(command), "{command} appears in {path:?}");
            }
        }
    }
}
