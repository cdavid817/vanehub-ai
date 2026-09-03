use crate::contexts::skill_evolution_generation::{
    application::dossier_builder_tests::{build, input, snapshot},
    application::{canonical_json, sha256_bytes},
    domain::DossierRecordV1,
};

use std::sync::Mutex;

use super::{
    render_dossier_export, DossierExportAuditPort, DossierExportAuditRecordV1, DossierExportError,
    DossierExportFormat, DossierExportOutcomeV1, DossierExportRequestV1, DossierExportService,
    DossierExportServiceError, DossierExportWriterPort,
};

#[test]
fn json_and_markdown_exports_are_deterministic_and_manifested() {
    let dossier = build(&input(), &snapshot()).expect("dossier");
    let json = render_dossier_export(&dossier, DossierExportFormat::Json).expect("json");
    let markdown =
        render_dossier_export(&dossier, DossierExportFormat::Markdown).expect("markdown");
    assert_eq!(
        json.content,
        canonical_json(&dossier).expect("canonical dossier")
    );
    assert_eq!(json.content_hash, sha256_bytes(json.content.as_bytes()));
    assert_eq!(
        markdown.content_hash,
        sha256_bytes(markdown.content.as_bytes())
    );
    assert!(markdown.content.contains(&dossier.content_hash));
    assert!(markdown.content.contains(&markdown.redaction_manifest_hash));
    assert_eq!(
        markdown,
        render_dossier_export(&dossier, DossierExportFormat::Markdown).expect("repeat")
    );
}

#[test]
fn exports_keep_redaction_markers_and_exclude_unregistered_source_data() {
    let mut source = snapshot();
    source.seed.safe_summary = "failed at [REDACTED:PATH]".into();
    let dossier = build(&input(), &source).expect("dossier");
    for format in [DossierExportFormat::Json, DossierExportFormat::Markdown] {
        let export = render_dossier_export(&dossier, format).expect("export");
        assert!(export.content.contains("[REDACTED:PATH]"));
        assert!(!export.content.contains("/private/secret"));
        assert!(!export.content.contains("rawPrompt"));
        assert!(!export.content.contains("providerPayload"));
    }
}

#[test]
fn oversized_render_is_rejected() {
    let mut dossier = build(&input(), &snapshot()).expect("dossier");
    dossier.sections[1].records.push(DossierRecordV1::Identity {
        identity_kind: "oversized".into(),
        value: "x".repeat(600 * 1024),
    });
    assert_eq!(
        render_dossier_export(&dossier, DossierExportFormat::Markdown),
        Err(DossierExportError::SizeLimitExceeded)
    );
}

#[test]
fn export_service_treats_an_absent_user_destination_as_cancellation() {
    let writer = RecordingWriter::default();
    let audit = RecordingAudit::default();
    let dossier = build(&input(), &snapshot()).expect("dossier");
    let service = DossierExportService::new(&writer, &audit);
    let outcome = service
        .export(&DossierExportRequestV1 {
            export_id: "export-one",
            destination_directory: None,
            dossier: &dossier,
            format: DossierExportFormat::Json,
            created_at_ms: 1,
        })
        .expect("cancelled");
    assert_eq!(outcome, DossierExportOutcomeV1::Cancelled);
    assert_eq!(*writer.calls.lock().expect("writer calls"), 0);
    assert_eq!(*audit.calls.lock().expect("audit calls"), 0);
}

#[test]
fn export_service_writes_before_recording_safe_metadata() {
    let writer = RecordingWriter::default();
    let audit = RecordingAudit::default();
    let dossier = build(&input(), &snapshot()).expect("dossier");
    let service = DossierExportService::new(&writer, &audit);
    let outcome = service
        .export(&DossierExportRequestV1 {
            export_id: "export-two",
            destination_directory: Some("/selected"),
            dossier: &dossier,
            format: DossierExportFormat::Markdown,
            created_at_ms: 1,
        })
        .expect("export");
    assert!(matches!(outcome, DossierExportOutcomeV1::Exported { .. }));
    assert_eq!(*writer.calls.lock().expect("writer calls"), 1);
    assert_eq!(*audit.calls.lock().expect("audit calls"), 1);
}

#[derive(Default)]
struct RecordingWriter {
    calls: Mutex<u32>,
}

impl DossierExportWriterPort for RecordingWriter {
    fn write_user_selected_export(
        &self,
        _: &str,
        _: &str,
        _: &str,
    ) -> Result<String, DossierExportServiceError> {
        *self.calls.lock().expect("writer calls") += 1;
        Ok("/selected/export.md".into())
    }
}

#[derive(Default)]
struct RecordingAudit {
    calls: Mutex<u32>,
}

impl DossierExportAuditPort for RecordingAudit {
    fn record_export(
        &self,
        _: &DossierExportAuditRecordV1<'_>,
    ) -> Result<(), DossierExportServiceError> {
        *self.calls.lock().expect("audit calls") += 1;
        Ok(())
    }
}
