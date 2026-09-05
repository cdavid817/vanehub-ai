use serde::{Deserialize, Serialize};

use crate::contexts::skill_evolution_generation::{
    application::{canonical_hash, canonical_json, sha256_bytes},
    domain::{DossierSectionStatus, EvidenceDossierV1},
};

const EXPORT_SIZE_LIMIT_V1: usize = 512 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum DossierExportFormat {
    Json,
    Markdown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RenderedDossierExportV1 {
    pub(crate) format: DossierExportFormat,
    pub(crate) media_type: &'static str,
    pub(crate) extension: &'static str,
    pub(crate) content: String,
    pub(crate) content_hash: String,
    pub(crate) redaction_manifest_hash: String,
    pub(crate) complete: bool,
    pub(crate) size_bytes: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DossierExportError {
    InvalidDossier,
    Serialization,
    SizeLimitExceeded,
}

pub(crate) fn render_dossier_export(
    dossier: &EvidenceDossierV1,
    format: DossierExportFormat,
) -> Result<RenderedDossierExportV1, DossierExportError> {
    if !dossier.has_exact_section_shape() {
        return Err(DossierExportError::InvalidDossier);
    }
    let complete = dossier.sections.iter().all(|section| {
        section.truncation.complete
            && !matches!(
                section.status,
                DossierSectionStatus::Partial | DossierSectionStatus::Unavailable
            )
    });
    let redaction_manifest_hash = canonical_hash(&(
        dossier.schema_version,
        &dossier.sanitizer_version,
        dossier.sections[9].status,
        &dossier.sections[9].section_hash,
        &dossier.sections[9].truncation,
    ))
    .map_err(|_| DossierExportError::Serialization)?;
    let (media_type, extension, content) = match format {
        DossierExportFormat::Json => (
            "application/json",
            "json",
            canonical_json(dossier).map_err(|_| DossierExportError::Serialization)?,
        ),
        DossierExportFormat::Markdown => (
            "text/markdown",
            "md",
            render_markdown(dossier, complete, &redaction_manifest_hash)?,
        ),
    };
    if content.len() > EXPORT_SIZE_LIMIT_V1 {
        return Err(DossierExportError::SizeLimitExceeded);
    }
    let content_hash = sha256_bytes(content.as_bytes());
    Ok(RenderedDossierExportV1 {
        format,
        media_type,
        extension,
        size_bytes: content.len() as u64,
        content,
        content_hash,
        redaction_manifest_hash,
        complete,
    })
}

fn render_markdown(
    dossier: &EvidenceDossierV1,
    complete: bool,
    redaction_manifest_hash: &str,
) -> Result<String, DossierExportError> {
    let mut output = String::new();
    output.push_str("# Skill Evolution Evidence Dossier\n\n");
    push_field(&mut output, "Dossier", &dossier.dossier_id);
    push_field(&mut output, "Revision", &dossier.revision.to_string());
    push_field(&mut output, "Schema", &dossier.schema_version.to_string());
    push_field(&mut output, "Builder", &dossier.builder_version);
    push_field(&mut output, "Sanitizer", &dossier.sanitizer_version);
    push_field(&mut output, "Content hash", &dossier.content_hash);
    push_field(&mut output, "Redaction manifest", redaction_manifest_hash);
    push_field(&mut output, "Complete", if complete { "yes" } else { "no" });
    for section in &dossier.sections {
        let kind = enum_text(&section.kind)?;
        let status = enum_text(&section.status)?;
        output.push_str(&format!("\n## {}. {}\n\n", section.ordinal + 1, kind));
        push_field(&mut output, "Status", &status);
        push_field(&mut output, "Section hash", &section.section_hash);
        push_field(
            &mut output,
            "Records",
            &format!(
                "{} of {} ({})",
                section.truncation.retained_count,
                section.truncation.total_count,
                if section.truncation.complete {
                    "complete"
                } else {
                    "truncated"
                }
            ),
        );
        if let Some(reason) = &section.unavailable_reason_code {
            push_field(&mut output, "Reason", reason);
        }
        output.push_str("\n### Safe records\n\n");
        if section.records.is_empty() {
            output.push_str("- None\n");
        } else {
            for record in &section.records {
                let record =
                    canonical_json(record).map_err(|_| DossierExportError::Serialization)?;
                output.push_str("- `");
                output.push_str(&record.replace('`', "\\`"));
                output.push_str("`\n");
            }
        }
    }
    Ok(output)
}

fn enum_text<T: Serialize>(value: &T) -> Result<String, DossierExportError> {
    serde_json::to_value(value)
        .map_err(|_| DossierExportError::Serialization)?
        .as_str()
        .map(str::to_owned)
        .ok_or(DossierExportError::Serialization)
}

fn push_field(output: &mut String, label: &str, value: &str) {
    output.push_str("- ");
    output.push_str(label);
    output.push_str(": `");
    output.push_str(&value.replace('`', "\\`"));
    output.push_str("`\n");
}
