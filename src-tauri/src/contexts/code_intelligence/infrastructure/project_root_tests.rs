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
