use std::path::Path;

use crate::{
    contexts::skill_evolution_generation::application::{
        DossierExportAuditPort, DossierExportAuditRecordV1, DossierExportFormat,
        DossierExportServiceError, DossierExportWriterPort,
    },
    platform::{database::NativeDatabase, filesystem::BoundedFilesystem},
};
use rusqlite::params;

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct BoundedDossierExportWriter;

impl DossierExportWriterPort for BoundedDossierExportWriter {
    fn write_user_selected_export(
        &self,
        destination_directory: &str,
        filename: &str,
        content: &str,
    ) -> Result<String, DossierExportServiceError> {
        let filesystem = BoundedFilesystem::new(Path::new(destination_directory))
            .map_err(|_| DossierExportServiceError::Write)?;
        let (path, _) = filesystem
            .resolve_with_existing_parent(filename)
            .map_err(|_| DossierExportServiceError::Write)?;
        std::fs::write(&path, content).map_err(|_| DossierExportServiceError::Write)?;
        Ok(path.to_string_lossy().into_owned())
    }
}

#[derive(Clone)]
pub(crate) struct SqliteDossierExportAudit {
    database: NativeDatabase,
}

impl SqliteDossierExportAudit {
    pub(crate) fn new(database: NativeDatabase) -> Self {
        Self { database }
    }
}

impl DossierExportAuditPort for SqliteDossierExportAudit {
    fn record_export(
        &self,
        record: &DossierExportAuditRecordV1<'_>,
    ) -> Result<(), DossierExportServiceError> {
        let size_bytes = i64::try_from(record.size_bytes)
            .map_err(|_| DossierExportServiceError::InvalidInput)?;
        let connection = self
            .database
            .connection()
            .map_err(|_| DossierExportServiceError::Audit)?;
        connection
            .execute(
                "INSERT INTO evolution_generation_exports
             (export_id,dossier_id,format,schema_version,complete,redaction_manifest_hash,
              content_hash,size_bytes,created_at_ms) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9)",
                params![
                    record.export_id,
                    record.dossier_id,
                    format_name(record.format),
                    record.schema_version,
                    i64::from(record.complete),
                    record.redaction_manifest_hash,
                    record.content_hash,
                    size_bytes,
                    record.created_at_ms
                ],
            )
            .map_err(|_| DossierExportServiceError::Audit)?;
        Ok(())
    }
}

fn format_name(format: DossierExportFormat) -> &'static str {
    match format {
        DossierExportFormat::Json => "json",
        DossierExportFormat::Markdown => "markdown",
    }
}
