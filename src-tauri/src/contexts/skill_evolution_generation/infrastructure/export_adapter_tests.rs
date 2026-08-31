use crate::{
    contexts::skill_evolution_generation::application::{
        DossierExportServiceError, DossierExportWriterPort,
    },
    test_support::TempDirectory,
};

use super::BoundedDossierExportWriter;

#[test]
fn writer_stays_inside_the_user_selected_directory() {
    let directory = TempDirectory::new("generation-export-boundary");
    let writer = BoundedDossierExportWriter;
    let root = directory.path().to_string_lossy();
    let path = writer
        .write_user_selected_export(&root, "dossier.json", "{}")
        .expect("safe export");
    // Both sides canonical. The writer resolves the destination before writing, and on Windows
    // `canonicalize` returns the extended-length form (a `\\?\` prefix) — so comparing its answer
    // against the path the test handed in fails for a reason unrelated to containment. The
    // assertion means "inside the selected directory", and that is a question about two canonical
    // paths.
    let selected = directory
        .path()
        .canonicalize()
        .expect("canonical directory");
    assert!(
        std::path::Path::new(&path).starts_with(&selected),
        "{path} is not inside {}",
        selected.display()
    );
    assert_eq!(
        writer.write_user_selected_export(&root, "../escape.json", "{}"),
        Err(DossierExportServiceError::Write)
    );
    assert_eq!(
        writer.write_user_selected_export(&root, ".hidden.json", "{}"),
        Err(DossierExportServiceError::Write)
    );
}
