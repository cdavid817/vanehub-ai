use crate::contexts::skill_evolution_generation::domain::EvidenceDossierV1;

use super::{render_dossier_export, DossierExportError, DossierExportFormat};

pub(crate) struct DossierExportRequestV1<'a> {
    pub(crate) export_id: &'a str,
    pub(crate) destination_directory: Option<&'a str>,
    pub(crate) dossier: &'a EvidenceDossierV1,
    pub(crate) format: DossierExportFormat,
    pub(crate) created_at_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum DossierExportOutcomeV1 {
    Cancelled,
    Exported { path: String, content_hash: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DossierExportServiceError {
    InvalidInput,
    Render,
    Write,
    Audit,
}

pub(crate) struct DossierExportAuditRecordV1<'a> {
    pub(crate) export_id: &'a str,
    pub(crate) dossier_id: &'a str,
    pub(crate) format: DossierExportFormat,
    pub(crate) schema_version: u16,
    pub(crate) complete: bool,
    pub(crate) redaction_manifest_hash: &'a str,
    pub(crate) content_hash: &'a str,
    pub(crate) size_bytes: u64,
    pub(crate) created_at_ms: i64,
}

pub(crate) trait DossierExportWriterPort: Send + Sync {
    fn write_user_selected_export(
        &self,
        destination_directory: &str,
        filename: &str,
        content: &str,
    ) -> Result<String, DossierExportServiceError>;
}

pub(crate) trait DossierExportAuditPort: Send + Sync {
    fn record_export(
        &self,
        record: &DossierExportAuditRecordV1<'_>,
    ) -> Result<(), DossierExportServiceError>;
}

pub(crate) struct DossierExportService<'ports> {
    writer: &'ports dyn DossierExportWriterPort,
    audit: &'ports dyn DossierExportAuditPort,
}

impl<'ports> DossierExportService<'ports> {
    pub(crate) fn new(
        writer: &'ports dyn DossierExportWriterPort,
        audit: &'ports dyn DossierExportAuditPort,
    ) -> Self {
        Self { writer, audit }
    }

    pub(crate) fn export(
        &self,
        request: &DossierExportRequestV1<'_>,
    ) -> Result<DossierExportOutcomeV1, DossierExportServiceError> {
        validate_request(request)?;
        let Some(destination) = request
            .destination_directory
            .map(str::trim)
            .filter(|value| !value.is_empty())
        else {
            return Ok(DossierExportOutcomeV1::Cancelled);
        };
        let rendered =
            render_dossier_export(request.dossier, request.format).map_err(map_render_error)?;
        let filename = format!(
            "skill-evolution-dossier-{}.{}",
            request.export_id, rendered.extension
        );
        let path =
            self.writer
                .write_user_selected_export(destination, &filename, &rendered.content)?;
        self.audit.record_export(&DossierExportAuditRecordV1 {
            export_id: request.export_id,
            dossier_id: &request.dossier.dossier_id,
            format: request.format,
            schema_version: request.dossier.schema_version,
            complete: rendered.complete,
            redaction_manifest_hash: &rendered.redaction_manifest_hash,
            content_hash: &rendered.content_hash,
            size_bytes: rendered.size_bytes,
            created_at_ms: request.created_at_ms,
        })?;
        Ok(DossierExportOutcomeV1::Exported {
            path,
            content_hash: rendered.content_hash,
        })
    }
}

fn validate_request(request: &DossierExportRequestV1<'_>) -> Result<(), DossierExportServiceError> {
    if request.export_id.is_empty()
        || request.export_id.len() > 80
        || !request
            .export_id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
        || request.created_at_ms < 0
    {
        return Err(DossierExportServiceError::InvalidInput);
    }
    Ok(())
}

fn map_render_error(_: DossierExportError) -> DossierExportServiceError {
    DossierExportServiceError::Render
}
