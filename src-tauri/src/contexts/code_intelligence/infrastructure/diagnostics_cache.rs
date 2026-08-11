use super::document_lease::PreparedDocument;
use super::document_snapshot::{DocumentAdmission, DocumentAdmissionError};
use super::position_conversion::PositionConverter;
use crate::contexts::code_intelligence::domain::models::{
    DiagnosticSeverity, DiagnosticSnapshot, DocumentVersion, NormalizedDiagnostic,
    NormalizedLocation, NormalizedRelatedDiagnostic, PositionEncoding, QueryStatus,
};
use lsp_types::{
    Diagnostic, DiagnosticRelatedInformation, DiagnosticSeverity as LspDiagnosticSeverity,
    NumberOrString, PublishDiagnosticsParams,
};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::Duration;
use thiserror::Error;
use tokio::sync::Notify;
use tokio::time::Instant;
use url::Url;

pub(crate) const MAX_DIAGNOSTICS_PER_DOCUMENT: usize = 200;
pub(crate) const MAX_DIAGNOSTIC_MESSAGE_BYTES: usize = 4 * 1024;
const MAX_DIAGNOSTIC_METADATA_BYTES: usize = 128;
const MAX_RELATED_PER_DIAGNOSTIC: usize = 8;
const MAX_RELATED_MESSAGE_BYTES: usize = 512;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DiagnosticsReadiness {
    Ready,
    Warming,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub(crate) enum DiagnosticsCacheError {
    #[error(transparent)]
    Admission(#[from] DocumentAdmissionError),
    #[error("diagnostics target does not match the document lease")]
    WrongDocument,
    #[error("diagnostics cache is unavailable")]
    Unavailable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct DiagnosticsPublishSummary {
    filtered_related_count: usize,
    truncated: bool,
}

impl DiagnosticsPublishSummary {
    pub(crate) const fn filtered_related_count(self) -> usize {
        self.filtered_related_count
    }

    pub(crate) const fn truncated(self) -> bool {
        self.truncated
    }
}

#[derive(Debug, Clone)]
pub(crate) struct DiagnosticsQueryResult {
    status: QueryStatus,
    stale: bool,
    snapshot: Option<DiagnosticSnapshot>,
    total: usize,
    truncated: bool,
    filtered_count: usize,
}

impl DiagnosticsQueryResult {
    pub(crate) const fn status(&self) -> QueryStatus {
        self.status
    }

    pub(crate) const fn stale(&self) -> bool {
        self.stale
    }

    pub(crate) fn snapshot(&self) -> Option<&DiagnosticSnapshot> {
        self.snapshot.as_ref()
    }

    pub(crate) const fn total(&self) -> usize {
        self.total
    }

    pub(crate) const fn truncated(&self) -> bool {
        self.truncated
    }

    pub(crate) const fn filtered_count(&self) -> usize {
        self.filtered_count
    }
}

#[derive(Debug, Clone)]
struct DiagnosticsCacheEntry {
    snapshot: DiagnosticSnapshot,
    total: usize,
    truncated: bool,
    filtered_count: usize,
}

pub(crate) struct DiagnosticsCache {
    workspace_root: PathBuf,
    admission: DocumentAdmission,
    encoding: PositionEncoding,
    snapshots: Mutex<HashMap<String, DiagnosticsCacheEntry>>,
    changed: Notify,
}

impl DiagnosticsCache {
    pub(crate) fn new(
        workspace_root: &Path,
        encoding: PositionEncoding,
    ) -> Result<Self, DiagnosticsCacheError> {
        let workspace_root = workspace_root
            .canonicalize()
            .map_err(|_| DocumentAdmissionError::Unavailable)?;
        Ok(Self {
            admission: DocumentAdmission::new(&workspace_root)?,
            workspace_root,
            encoding,
            snapshots: Mutex::new(HashMap::new()),
            changed: Notify::new(),
        })
    }

    pub(crate) fn publish(
        &self,
        document: &PreparedDocument,
        params: PublishDiagnosticsParams,
        received_at_epoch_ms: u64,
    ) -> Result<DiagnosticsPublishSummary, DiagnosticsCacheError> {
        if params.uri.as_str() != document.uri().as_str() {
            return Err(DiagnosticsCacheError::WrongDocument);
        }
        let total = params.diagnostics.len();
        let mut filtered_related_count = 0;
        let mut truncated = total > MAX_DIAGNOSTICS_PER_DOCUMENT;
        let mut diagnostics = Vec::with_capacity(total.min(MAX_DIAGNOSTICS_PER_DOCUMENT));
        for diagnostic in params
            .diagnostics
            .into_iter()
            .take(MAX_DIAGNOSTICS_PER_DOCUMENT)
        {
            truncated |= diagnostic_exceeds_limits(&diagnostic);
            if let Some(normalized) =
                self.normalize_diagnostic(document.text(), diagnostic, &mut filtered_related_count)
            {
                diagnostics.push(normalized);
            }
        }
        let server_version = params
            .version
            .and_then(|version| u64::try_from(version).ok())
            .map(DocumentVersion::new);
        let snapshot = DiagnosticSnapshot::new(
            server_version,
            document.version(),
            diagnostics,
            received_at_epoch_ms,
        );
        self.snapshots
            .lock()
            .map_err(|_| DiagnosticsCacheError::Unavailable)?
            .insert(
                document.uri().to_string(),
                DiagnosticsCacheEntry {
                    snapshot,
                    total,
                    truncated,
                    filtered_count: filtered_related_count,
                },
            );
        self.changed.notify_waiters();
        Ok(DiagnosticsPublishSummary {
            filtered_related_count,
            truncated,
        })
    }

    pub(crate) fn snapshot(&self, uri: &Url) -> Option<DiagnosticSnapshot> {
        self.snapshots
            .lock()
            .ok()?
            .get(uri.as_str())
            .map(|entry| entry.snapshot.clone())
    }

    pub(crate) fn diagnostic_count(&self) -> usize {
        self.snapshots
            .lock()
            .map(|snapshots| {
                snapshots
                    .values()
                    .map(|entry| entry.snapshot.diagnostics().len())
                    .sum()
            })
            .unwrap_or(0)
    }

    pub(crate) async fn wait_for_current(
        &self,
        uri: &Url,
        local_version: DocumentVersion,
        readiness: DiagnosticsReadiness,
        deadline: Duration,
    ) -> DiagnosticsQueryResult {
        if readiness == DiagnosticsReadiness::Warming {
            return self.result(uri, QueryStatus::Warming, true);
        }
        let end = Instant::now() + deadline;
        loop {
            let notified = self.changed.notified();
            if let Some(snapshot) = self.snapshot(uri) {
                let server_matches = snapshot
                    .server_version
                    .is_none_or(|version| version == local_version);
                if snapshot.is_current_for(local_version) && server_matches {
                    return self.result(uri, QueryStatus::Ready, false);
                }
            }
            let remaining = end.saturating_duration_since(Instant::now());
            if remaining.is_zero() || tokio::time::timeout(remaining, notified).await.is_err() {
                return self.result(uri, QueryStatus::Timeout, true);
            }
        }
    }

    pub(crate) fn clear_after_process_exit(&self) {
        if let Ok(mut snapshots) = self.snapshots.lock() {
            snapshots.clear();
        }
        self.changed.notify_waiters();
    }

    fn result(
        &self,
        uri: &Url,
        status: QueryStatus,
        stale_when_present: bool,
    ) -> DiagnosticsQueryResult {
        let entry = self
            .snapshots
            .lock()
            .ok()
            .and_then(|snapshots| snapshots.get(uri.as_str()).cloned());
        DiagnosticsQueryResult {
            status,
            stale: stale_when_present && entry.is_some(),
            snapshot: entry.as_ref().map(|entry| entry.snapshot.clone()),
            total: entry.as_ref().map_or(0, |entry| entry.total),
            truncated: entry.as_ref().is_some_and(|entry| entry.truncated),
            filtered_count: entry.map_or(0, |entry| entry.filtered_count),
        }
    }

    fn normalize_diagnostic(
        &self,
        document_text: &str,
        diagnostic: Diagnostic,
        filtered_related_count: &mut usize,
    ) -> Option<NormalizedDiagnostic> {
        let range = PositionConverter::new(document_text, self.encoding)
            .range_to_normalized(diagnostic.range)
            .ok()?;
        let related_information = diagnostic
            .related_information
            .unwrap_or_default()
            .into_iter()
            .take(MAX_RELATED_PER_DIAGNOSTIC)
            .filter_map(|related| match self.normalize_related(related) {
                Some(related) => Some(related),
                None => {
                    *filtered_related_count += 1;
                    None
                }
            })
            .collect();
        Some(NormalizedDiagnostic {
            range,
            severity: diagnostic.severity.and_then(normalize_severity),
            message: truncate_utf8(&diagnostic.message, MAX_DIAGNOSTIC_MESSAGE_BYTES),
            source: diagnostic
                .source
                .map(|source| truncate_utf8(&source, MAX_DIAGNOSTIC_METADATA_BYTES)),
            code: diagnostic.code.map(normalize_code),
            related_information,
        })
    }

    fn normalize_related(
        &self,
        related: DiagnosticRelatedInformation,
    ) -> Option<NormalizedRelatedDiagnostic> {
        let uri = Url::parse(related.location.uri.as_str()).ok()?;
        if uri.scheme() != "file" {
            return None;
        }
        let path = uri.to_file_path().ok()?.canonicalize().ok()?;
        let relative = path.strip_prefix(&self.workspace_root).ok()?;
        let relative = relative.to_string_lossy().replace('\\', "/");
        let snapshot = self.admission.read(&relative).ok()?;
        let range = PositionConverter::new(snapshot.text(), self.encoding)
            .range_to_normalized(related.location.range)
            .ok()?;
        let location = NormalizedLocation::new(snapshot.relative_path(), range, None).ok()?;
        Some(NormalizedRelatedDiagnostic {
            location,
            message: truncate_utf8(&related.message, MAX_RELATED_MESSAGE_BYTES),
        })
    }
}

fn normalize_severity(severity: LspDiagnosticSeverity) -> Option<DiagnosticSeverity> {
    match severity {
        LspDiagnosticSeverity::ERROR => Some(DiagnosticSeverity::Error),
        LspDiagnosticSeverity::WARNING => Some(DiagnosticSeverity::Warning),
        LspDiagnosticSeverity::INFORMATION => Some(DiagnosticSeverity::Information),
        LspDiagnosticSeverity::HINT => Some(DiagnosticSeverity::Hint),
        _ => None,
    }
}

fn normalize_code(code: NumberOrString) -> String {
    let value = match code {
        NumberOrString::Number(number) => number.to_string(),
        NumberOrString::String(value) => value,
    };
    truncate_utf8(&value, MAX_DIAGNOSTIC_METADATA_BYTES)
}

fn diagnostic_exceeds_limits(diagnostic: &Diagnostic) -> bool {
    diagnostic.message.len() > MAX_DIAGNOSTIC_MESSAGE_BYTES
        || diagnostic
            .source
            .as_ref()
            .is_some_and(|source| source.len() > MAX_DIAGNOSTIC_METADATA_BYTES)
        || diagnostic.code.as_ref().is_some_and(|code| match code {
            NumberOrString::Number(_) => false,
            NumberOrString::String(value) => value.len() > MAX_DIAGNOSTIC_METADATA_BYTES,
        })
        || diagnostic
            .related_information
            .as_ref()
            .is_some_and(|related| {
                related.len() > MAX_RELATED_PER_DIAGNOSTIC
                    || related
                        .iter()
                        .any(|item| item.message.len() > MAX_RELATED_MESSAGE_BYTES)
            })
}

fn truncate_utf8(value: &str, max_bytes: usize) -> String {
    if value.len() <= max_bytes {
        return value.to_owned();
    }
    let mut end = max_bytes;
    while !value.is_char_boundary(end) {
        end -= 1;
    }
    value[..end].to_owned()
}
