use super::document_snapshot::{DocumentAdmission, DocumentAdmissionError};
use crate::contexts::code_intelligence::domain::registry;
use std::path::{Path, PathBuf};

#[test]
fn relative_source_file_resolves_to_a_bounded_canonical_snapshot() {
    let fixture = WorkspaceFixture::new();
    fixture.write("src/main.rs", b"fn main() {}\n");

    let snapshot = DocumentAdmission::new(fixture.root())
        .expect("admission")
        .read("src/main.rs")
        .expect("snapshot");

    assert_eq!(snapshot.relative_path(), "src/main.rs");
    assert_eq!(snapshot.language(), registry::rust());
    assert_eq!(snapshot.text(), "fn main() {}\n");
    assert_eq!(snapshot.canonical_path(), fixture.canonical("src/main.rs"));
}

#[test]
fn absolute_traversal_and_hidden_paths_are_rejected_before_reading() {
    let fixture = WorkspaceFixture::new();
    fixture.write("src/main.ts", b"export {};\n");
    fixture.write(".hidden/secret.ts", b"secret\n");
    let admission = DocumentAdmission::new(fixture.root()).expect("admission");

    assert_eq!(
        admission.read(&fixture.root().join("src/main.ts").to_string_lossy()),
        Err(DocumentAdmissionError::AbsolutePath)
    );
    assert_eq!(
        admission.read("../secret.ts"),
        Err(DocumentAdmissionError::Traversal)
    );
    assert_eq!(
        admission.read(".hidden/secret.ts"),
        Err(DocumentAdmissionError::HiddenPath)
    );
}

#[test]
fn adversarial_nested_path_forms_cannot_escape_or_enter_hidden_directories() {
    let fixture = WorkspaceFixture::new();
    fixture.write("src/main.ts", b"export {};\n");
    fixture.write("src/.private/secret.ts", b"secret\n");
    let admission = DocumentAdmission::new(fixture.root()).expect("admission");
    let nested_traversal = PathBuf::from("src").join("..").join("..").join("secret.ts");

    assert_eq!(
        admission.read(nested_traversal.to_string_lossy().as_ref()),
        Err(DocumentAdmissionError::Traversal)
    );
    assert_eq!(
        admission.read("src/.private/secret.ts"),
        Err(DocumentAdmissionError::HiddenPath)
    );
    assert_eq!(
        admission.read(fixture.canonical("src/main.ts").to_string_lossy().as_ref()),
        Err(DocumentAdmissionError::AbsolutePath)
    );
}

#[test]
fn directories_and_unsupported_languages_are_not_admitted() {
    let fixture = WorkspaceFixture::new();
    std::fs::create_dir_all(fixture.root().join("src")).expect("directory");
    fixture.write("README.md", b"text\n");
    let admission = DocumentAdmission::new(fixture.root()).expect("admission");

    assert_eq!(admission.read("src"), Err(DocumentAdmissionError::NotFile));
    assert_eq!(
        admission.read("README.md"),
        Err(DocumentAdmissionError::UnsupportedLanguage)
    );
}

#[test]
fn binary_and_invalid_utf8_content_are_rejected() {
    let fixture = WorkspaceFixture::new();
    fixture.write("src/binary.rs", b"text\0binary");
    fixture.write("src/invalid.ts", &[0xf0, 0x28, 0x8c, 0x28]);
    let admission = DocumentAdmission::new(fixture.root()).expect("admission");

    assert_eq!(
        admission.read("src/binary.rs"),
        Err(DocumentAdmissionError::BinaryContent)
    );
    assert_eq!(
        admission.read("src/invalid.ts"),
        Err(DocumentAdmissionError::InvalidUtf8)
    );
}

#[test]
fn exact_size_limit_is_accepted_and_over_limit_is_rejected() {
    let fixture = WorkspaceFixture::new();
    fixture.write("src/exact.js", b"12345678");
    fixture.write("src/large.js", b"123456789");
    let admission = DocumentAdmission::with_max_bytes(fixture.root(), 8).expect("admission");

    assert_eq!(
        admission.read("src/exact.js").expect("exact").text(),
        "12345678"
    );
    assert_eq!(
        admission.read("src/large.js"),
        Err(DocumentAdmissionError::FileTooLarge)
    );
}

#[test]
fn supported_typescript_and_javascript_extensions_share_one_language_family() {
    let fixture = WorkspaceFixture::new();
    let admission = DocumentAdmission::new(fixture.root()).expect("admission");
    for (extension, language_id) in [
        ("ts", "typescript"),
        ("tsx", "typescriptreact"),
        ("js", "javascript"),
        ("jsx", "javascriptreact"),
        ("mjs", "javascript"),
        ("cjs", "javascript"),
    ] {
        let relative = format!("src/file.{extension}");
        fixture.write(&relative, b"export {};\n");
        let snapshot = admission.read(&relative).expect("snapshot");
        assert_eq!(snapshot.language(), registry::typescript());
        assert_eq!(snapshot.language_id(), language_id);
    }
}

#[test]
fn symlink_escape_is_rejected_when_links_are_supported() {
    let workspace = WorkspaceFixture::new();
    let outside = WorkspaceFixture::new();
    outside.write("secret.rs", b"private\n");
    let link = workspace.root().join("escape.rs");
    if create_file_symlink(&outside.root().join("secret.rs"), &link).is_err() {
        return;
    }

    assert_eq!(
        DocumentAdmission::new(workspace.root())
            .expect("admission")
            .read("escape.rs"),
        Err(DocumentAdmissionError::OutsideWorkspace)
    );
}

#[test]
fn symlinked_directory_escape_is_rejected_when_links_are_supported() {
    let workspace = WorkspaceFixture::new();
    let outside = WorkspaceFixture::new();
    outside.write("nested/secret.ts", b"private\n");
    let link = workspace.root().join("linked");
    if create_directory_symlink(outside.root(), &link).is_err() {
        return;
    }

    assert_eq!(
        DocumentAdmission::new(workspace.root())
            .expect("admission")
            .read("linked/nested/secret.ts"),
        Err(DocumentAdmissionError::OutsideWorkspace)
    );
}

struct WorkspaceFixture {
    directory: tempfile::TempDir,
}

impl WorkspaceFixture {
    fn new() -> Self {
        Self {
            directory: tempfile::tempdir().expect("workspace"),
        }
    }

    fn root(&self) -> &Path {
        self.directory.path()
    }

    fn write(&self, relative: &str, content: &[u8]) {
        let path = self.root().join(relative);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).expect("parent");
        }
        std::fs::write(path, content).expect("file");
    }

    fn canonical(&self, relative: &str) -> PathBuf {
        self.root()
            .join(relative)
            .canonicalize()
            .expect("canonical")
    }
}

#[cfg(unix)]
fn create_file_symlink(source: &Path, target: &Path) -> std::io::Result<()> {
    std::os::unix::fs::symlink(source, target)
}

#[cfg(windows)]
fn create_file_symlink(source: &Path, target: &Path) -> std::io::Result<()> {
    std::os::windows::fs::symlink_file(source, target)
}

#[cfg(unix)]
fn create_directory_symlink(source: &Path, target: &Path) -> std::io::Result<()> {
    std::os::unix::fs::symlink(source, target)
}

#[cfg(windows)]
fn create_directory_symlink(source: &Path, target: &Path) -> std::io::Result<()> {
    std::os::windows::fs::symlink_dir(source, target)
}
