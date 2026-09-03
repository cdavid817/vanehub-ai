use super::project_root::{ProcessKey, ProjectRootError, ProjectRootResolver};
use crate::contexts::code_intelligence::domain::models::ConfigurationFingerprint;
use crate::contexts::code_intelligence::domain::registry;
use std::path::{Path, PathBuf};

#[test]
fn rust_uses_the_nearest_cargo_marker_for_nested_projects() {
    let fixture = ProjectFixture::new();
    fixture.marker("Cargo.toml");
    fixture.marker("crates/nested/Cargo.toml");
    let file = fixture.file("crates/nested/src/lib.rs");

    let root = ProjectRootResolver::resolve(fixture.root(), &file, registry::rust())
        .expect("nested Rust root");

    assert_eq!(root, fixture.canonical("crates/nested"));
}

#[test]
fn typescript_and_javascript_accept_every_supported_marker() {
    for (marker, source) in [
        ("tsconfig.json", "src/main.ts"),
        ("jsconfig.json", "src/main.js"),
        ("package.json", "src/index.js"),
    ] {
        let fixture = ProjectFixture::new();
        fixture.marker(&format!("apps/client/{marker}"));
        let file = fixture.file(&format!("apps/client/{source}"));

        let root = ProjectRootResolver::resolve(fixture.root(), &file, registry::typescript())
            .expect("TypeScript root");

        assert_eq!(root, fixture.canonical("apps/client"));
    }
}

#[test]
fn one_directory_holding_several_markers_resolves_the_same_as_one_holding_a_single_marker() {
    let several = ProjectFixture::new();
    several.marker("apps/client/tsconfig.json");
    several.marker("apps/client/package.json");
    let single = ProjectFixture::new();
    single.marker("apps/client/tsconfig.json");

    let several_root = ProjectRootResolver::resolve(
        several.root(),
        &several.file("apps/client/src/main.ts"),
        registry::typescript(),
    )
    .expect("root from several markers");
    let single_root = ProjectRootResolver::resolve(
        single.root(),
        &single.file("apps/client/src/main.ts"),
        registry::typescript(),
    )
    .expect("root from one marker");

    assert_eq!(
        several_root.strip_prefix(several.canonical("")),
        single_root.strip_prefix(single.canonical("")),
    );
}

#[test]
fn a_nearer_directory_wins_regardless_of_which_marker_each_holds() {
    // Proximity is the whole rule. A nested package with its own manifest is a real project root,
    // and skipping past it to an outer one would hand the server the wrong scope.
    let fixture = ProjectFixture::new();
    fixture.marker("tsconfig.json");
    fixture.marker("packages/inner/package.json");
    let file = fixture.file("packages/inner/src/main.ts");

    let root = ProjectRootResolver::resolve(fixture.root(), &file, registry::typescript())
        .expect("nested root");

    assert_eq!(root, fixture.canonical("packages/inner"));
}

#[test]
fn every_python_marker_identifies_a_root_on_its_own() {
    for marker in [
        "pyproject.toml",
        "setup.py",
        "setup.cfg",
        "requirements.txt",
    ] {
        let fixture = ProjectFixture::new();
        fixture.marker(&format!("services/api/{marker}"));
        let file = fixture.file("services/api/app/main.py");

        let root = ProjectRootResolver::resolve(fixture.root(), &file, registry::python())
            .unwrap_or_else(|error| panic!("{marker} did not identify a root: {error:?}"));

        assert_eq!(root, fixture.canonical("services/api"));
    }
}

#[test]
fn a_marker_naming_a_nested_path_resolves_the_candidate_directory() {
    // C/C++ declares `build/compile_commands.json`. The candidate directory is the one holding the
    // build directory, not the build directory itself.
    let fixture = ProjectFixture::new();
    fixture.marker("project/build/compile_commands.json");
    let file = fixture.file("project/src/main.cpp");

    let root = ProjectRootResolver::resolve(fixture.root(), &file, registry::cpp())
        .expect("compilation database in a build directory");

    assert_eq!(root, fixture.canonical("project"));
}

#[test]
fn a_language_requiring_a_marker_fails_instead_of_falling_back_to_the_workspace() {
    // clangd without a compilation database assumes default flags and answers confidently wrong,
    // so refusing is better than the session-root fallback every other language gets.
    let fixture = ProjectFixture::new();
    let file = fixture.file("src/main.cpp");

    let result = ProjectRootResolver::resolve(fixture.root(), &file, registry::cpp());

    assert_eq!(result, Err(ProjectRootError::RequiredMarkerMissing));
}

#[test]
fn a_required_marker_is_still_bounded_by_the_session_workspace() {
    let outer = tempfile::tempdir().expect("outer directory");
    std::fs::write(outer.path().join("compile_commands.json"), b"[]").expect("outer marker");
    let workspace = outer.path().join("workspace");
    std::fs::create_dir_all(workspace.join("src")).expect("workspace directories");
    let file = workspace.join("src/main.cpp");
    std::fs::write(&file, b"int main() { return 0; }").expect("source file");

    let result = ProjectRootResolver::resolve(&workspace, &file, registry::cpp());

    assert_eq!(result, Err(ProjectRootError::RequiredMarkerMissing));
}

#[test]
fn go_resolves_the_nearest_module_root() {
    let fixture = ProjectFixture::new();
    fixture.marker("go.mod");
    fixture.marker("services/worker/go.mod");
    let file = fixture.file("services/worker/main.go");

    let root = ProjectRootResolver::resolve(fixture.root(), &file, registry::go())
        .expect("Go module root");

    assert_eq!(root, fixture.canonical("services/worker"));
}

#[test]
fn distinct_nested_typescript_projects_produce_distinct_process_keys() {
    let fixture = ProjectFixture::new();
    fixture.marker("apps/a/tsconfig.json");
    fixture.marker("apps/b/package.json");
    let file_a = fixture.file("apps/a/src/main.ts");
    let file_b = fixture.file("apps/b/src/main.ts");
    let root_a = ProjectRootResolver::resolve(fixture.root(), &file_a, registry::typescript())
        .expect("project a");
    let root_b = ProjectRootResolver::resolve(fixture.root(), &file_b, registry::typescript())
        .expect("project b");
    let fingerprint = ConfigurationFingerprint::new("fixture-config").expect("fingerprint");

    let key_a = ProcessKey::new(
        fixture.root(),
        &root_a,
        registry::typescript(),
        fingerprint.clone(),
    )
    .expect("process key a");
    let key_b = ProcessKey::new(fixture.root(), &root_b, registry::typescript(), fingerprint)
        .expect("process key b");

    assert_ne!(key_a, key_b);
    assert_eq!(key_a.project_root(), root_a);
    assert_eq!(key_a.session_root(), fixture.canonical(""));
}

#[test]
fn markerless_files_use_the_canonical_session_root() {
    let fixture = ProjectFixture::new();
    let file = fixture.file("src/main.rs");
    let non_normalized_root = fixture.root().join("src").join("..");

    let root = ProjectRootResolver::resolve(&non_normalized_root, &file, registry::rust())
        .expect("fallback root");

    assert_eq!(root, fixture.canonical(""));
}

#[test]
fn detection_never_uses_a_marker_above_the_session_boundary() {
    let outer = tempfile::tempdir().expect("outer directory");
    std::fs::write(outer.path().join("Cargo.toml"), b"[workspace]").expect("outer marker");
    let workspace = outer.path().join("workspace");
    std::fs::create_dir_all(workspace.join("src")).expect("workspace directories");
    let file = workspace.join("src/main.rs");
    std::fs::write(&file, b"fn main() {}").expect("source file");

    let root = ProjectRootResolver::resolve(&workspace, &file, registry::rust())
        .expect("bounded fallback");

    assert_eq!(root, workspace.canonicalize().expect("canonical workspace"));
}

#[test]
fn files_outside_the_session_root_are_rejected() {
    let workspace = ProjectFixture::new();
    let outside = ProjectFixture::new();
    let file = outside.file("src/lib.rs");

    let result = ProjectRootResolver::resolve(workspace.root(), &file, registry::rust());

    assert_eq!(result, Err(ProjectRootError::OutsideSessionRoot));
}

struct ProjectFixture {
    directory: tempfile::TempDir,
}

impl ProjectFixture {
    fn new() -> Self {
        Self {
            directory: tempfile::tempdir().expect("project directory"),
        }
    }

    fn root(&self) -> &Path {
        self.directory.path()
    }

    fn marker(&self, relative: &str) {
        self.write(relative, b"{}");
    }

    fn file(&self, relative: &str) -> PathBuf {
        self.write(relative, b"fixture");
        self.root().join(relative)
    }

    fn write(&self, relative: &str, content: &[u8]) {
        let path = self.root().join(relative);
        std::fs::create_dir_all(path.parent().expect("parent")).expect("parent directories");
        std::fs::write(path, content).expect("fixture file");
    }

    fn canonical(&self, relative: &str) -> PathBuf {
        self.root()
            .join(relative)
            .canonicalize()
            .expect("canonical fixture path")
    }
}

#[test]
fn a_java_project_root_is_the_nearest_directory_holding_any_build_file() {
    let workspace = tempfile::tempdir().expect("workspace");
    let module = workspace.path().join("services/api");
    std::fs::create_dir_all(module.join("src/main/java")).expect("module tree");
    // Two markers in one directory, which must resolve to that directory rather than to two
    // candidates, and a Kotlin-DSL build file so the resolution does not depend on `build.gradle`.
    std::fs::write(
        module.join("build.gradle.kts"),
        b"plugins {}
",
    )
    .expect("build file");
    std::fs::write(
        module.join("settings.gradle"),
        b"
",
    )
    .expect("settings file");
    let document = module.join("src/main/java/Api.java");
    std::fs::write(
        &document,
        b"public class Api {}
",
    )
    .expect("source");

    let resolved = ProjectRootResolver::resolve(workspace.path(), &document, registry::java())
        .expect("java project root");

    assert_eq!(
        resolved.canonicalize().expect("canonical resolved"),
        module.canonicalize().expect("canonical module")
    );
}
