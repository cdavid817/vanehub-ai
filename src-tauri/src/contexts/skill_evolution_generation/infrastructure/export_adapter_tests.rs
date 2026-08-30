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
    assert!(std::path::Path::new(&path).starts_with(directory.path()));
    assert_eq!(
        writer.write_user_selected_export(&root, "../escape.json", "{}"),
        Err(DossierExportServiceError::Write)
    );
    assert_eq!(
        writer.write_user_selected_export(&root, ".hidden.json", "{}"),
        Err(DossierExportServiceError::Write)
    );
}
