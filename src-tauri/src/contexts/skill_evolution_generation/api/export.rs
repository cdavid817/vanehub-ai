use super::{GenerationApiError, SkillEvolutionGenerationApi};
use rusqlite::{params, OptionalExtension};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use tauri::AppHandle;
use tauri_plugin_dialog::DialogExt;

pub(crate) struct PreparedDossierExport {
    pub(crate) content: String,
    pub(crate) content_hash: String,
    pub(crate) redaction_manifest_hash: String,
    pub(crate) size_bytes: u64,
    pub(crate) complete: bool,
}

impl SkillEvolutionGenerationApi {
    pub(crate) fn export_dossier_to_user_file(
        &self,
        app: &AppHandle,
        dossier_id: &str,
        format: &str,
        now_ms: i64,
    ) -> Result<Value, GenerationApiError> {
        let prepared = self.prepare_export(dossier_id, format)?;
        let export_id = format!("generation-export-{}", uuid::Uuid::new_v4());
        let extension = if format == "markdown" { "md" } else { "json" };
        let selected = app
            .dialog()
            .file()
            .set_file_name(format!("skill-evolution-dossier-{export_id}.{extension}"))
            .blocking_save_file();
        let Some(selected) = selected else {
            return Ok(json!({
                "exportId": export_id,
                "status": "cancelled",
                "exportedFileRemainsUserManaged": true
            }));
        };
        let path = selected
            .into_path()
            .map_err(|_| GenerationApiError::InvalidRequest)?;
        std::fs::write(path, &prepared.content).map_err(|_| GenerationApiError::Storage)?;
        self.record_export(&export_id, dossier_id, format, &prepared, now_ms)?;
        Ok(json!({
            "exportId": export_id,
            "status": "exported",
            "contentHash": prepared.content_hash,
            "sizeBytes": prepared.size_bytes,
            "exportedFileRemainsUserManaged": true
        }))
    }

    pub(crate) fn prepare_export(
        &self,
        dossier_id: &str,
        format: &str,
    ) -> Result<PreparedDossierExport, GenerationApiError> {
        if dossier_id.trim().is_empty() || !matches!(format, "json" | "markdown") {
            return Err(GenerationApiError::InvalidRequest);
        }
        let connection = self
            .database
            .connection()
            .map_err(|_| GenerationApiError::Storage)?;
        let header = connection.query_row(
            "SELECT revision,input_witness_hash,builder_version,sanitizer_version,content_hash,created_at_ms FROM evolution_evidence_dossiers WHERE dossier_id=?1",
            [dossier_id],
            |row| Ok(json!({"dossierId":dossier_id,"revision":row.get::<_,i64>(0)?,"inputWitnessHash":row.get::<_,String>(1)?,"builderVersion":row.get::<_,String>(2)?,"sanitizerVersion":row.get::<_,String>(3)?,"contentHash":row.get::<_,String>(4)?,"createdAtMs":row.get::<_,i64>(5)?})),
        ).optional().map_err(|_| GenerationApiError::Storage)?.ok_or(GenerationApiError::NotFound)?;
        let mut statement = connection.prepare(
            "SELECT ordinal,section_kind,status,source_witnesses_json,records_json,truncation_json,unavailable_reason_code,section_hash FROM evolution_evidence_dossier_sections WHERE dossier_id=?1 ORDER BY ordinal"
        ).map_err(|_| GenerationApiError::Storage)?;
        let sections = statement.query_map([dossier_id], |row| {
            let witnesses: String = row.get(3)?;
            let records: String = row.get(4)?;
            let truncation: String = row.get(5)?;
            Ok(json!({"ordinal":row.get::<_,i64>(0)?,"kind":row.get::<_,String>(1)?,"status":row.get::<_,String>(2)?,"sourceWitnesses":serde_json::from_str::<Value>(&witnesses).unwrap_or_else(|_|json!([])),"records":serde_json::from_str::<Value>(&records).unwrap_or_else(|_|json!([])),"truncation":serde_json::from_str::<Value>(&truncation).unwrap_or_else(|_|json!({})),"unavailableReasonCode":row.get::<_,Option<String>>(6)?,"sectionHash":row.get::<_,String>(7)?}))
        }).map_err(|_| GenerationApiError::Storage)?.collect::<Result<Vec<_>,_>>().map_err(|_| GenerationApiError::Storage)?;
        if sections.len() != 13 {
            return Err(GenerationApiError::Storage);
        }
        let complete = sections
            .iter()
            .all(|section| section["truncation"]["complete"] == true);
        let document =
            json!({"schemaVersion":1,"complete":complete,"dossier":header,"sections":sections});
        let canonical =
            serde_json::to_string_pretty(&document).map_err(|_| GenerationApiError::Storage)?;
        let content = if format == "json" {
            canonical
        } else {
            format!("# Skill Evolution Evidence Dossier\n\n- Dossier: `{dossier_id}`\n- Complete: `{complete}`\n\n```json\n{canonical}\n```\n")
        };
        if content.len() > 1_048_576 {
            return Err(GenerationApiError::InvalidRequest);
        }
        let content_hash = hex_hash(content.as_bytes());
        let redaction_manifest_hash = hex_hash(document["sections"][9].to_string().as_bytes());
        Ok(PreparedDossierExport {
            size_bytes: content.len() as u64,
            content,
            content_hash,
            redaction_manifest_hash,
            complete,
        })
    }

    pub(crate) fn record_export(
        &self,
        export_id: &str,
        dossier_id: &str,
        format: &str,
        prepared: &PreparedDossierExport,
        now_ms: i64,
    ) -> Result<(), GenerationApiError> {
        let connection = self
            .database
            .connection()
            .map_err(|_| GenerationApiError::Storage)?;
        connection.execute("INSERT INTO evolution_generation_exports(export_id,dossier_id,format,schema_version,complete,redaction_manifest_hash,content_hash,size_bytes,created_at_ms) VALUES (?1,?2,?3,1,?4,?5,?6,?7,?8)",params![export_id,dossier_id,format,i64::from(prepared.complete),prepared.redaction_manifest_hash,prepared.content_hash,i64::try_from(prepared.size_bytes).map_err(|_|GenerationApiError::InvalidRequest)?,now_ms]).map_err(|_|GenerationApiError::Storage)?;
        Ok(())
    }
}

fn hex_hash(content: &[u8]) -> String {
    crate::platform::hashing::sha256_hex(content)
}
