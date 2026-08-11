use super::document_snapshot::{DiskDocumentSnapshot, DocumentAdmission, DocumentAdmissionError};
use super::json_rpc_actor::JsonRpcClient;
use super::position_conversion::{AgentPosition, PositionConversionError, PositionConverter};
use crate::contexts::code_intelligence::domain::models::{
    DocumentSyncMode, DocumentVersion, PositionEncoding,
};
use async_trait::async_trait;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use thiserror::Error;
use url::Url;

pub(crate) const IDLE_DOCUMENT_LEASE_TIMEOUT: Duration = Duration::from_secs(10 * 60);
pub(crate) const MAX_DOCUMENT_LEASES: usize = 32;
const MAX_RETAINED_DOCUMENT_BYTES: usize = 64 * 1024 * 1024;

#[async_trait]
pub(crate) trait DocumentNotificationSink: Send + Sync {
    async fn notify(&self, method: &'static str, params: Value) -> Result<(), String>;
}

#[async_trait]
impl DocumentNotificationSink for JsonRpcClient {
    async fn notify(&self, method: &'static str, params: Value) -> Result<(), String> {
        JsonRpcClient::notify(self, method, params)
            .await
            .map_err(|_| "LSP notification unavailable".to_owned())
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub(crate) enum DocumentLeaseError {
    #[error(transparent)]
    Admission(#[from] DocumentAdmissionError),
    #[error("document URI is unavailable")]
    InvalidUri,
    #[error("document version cannot advance")]
    VersionOverflow,
    #[error("server does not support document changes")]
    UnsupportedSynchronization,
    #[error("document position conversion failed")]
    PositionConversion,
    #[error("document notification failed")]
    Notification,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PreparedDocument {
    uri: Url,
    version: DocumentVersion,
    text: String,
}

impl PreparedDocument {
    pub(crate) fn uri(&self) -> &Url {
        &self.uri
    }

    pub(crate) const fn version(&self) -> DocumentVersion {
        self.version
    }

    pub(crate) fn text(&self) -> &str {
        &self.text
    }
}

struct DocumentLease {
    relative_path: String,
    uri: Url,
    version: DocumentVersion,
    digest: [u8; 32],
    text: String,
    last_used: Duration,
    invalidated: bool,
}

pub(crate) struct DocumentLeaseManager {
    admission: DocumentAdmission,
    sync_mode: DocumentSyncMode,
    position_encoding: PositionEncoding,
    sink: Arc<dyn DocumentNotificationSink>,
    leases: HashMap<PathBuf, DocumentLease>,
}

impl DocumentLeaseManager {
    pub(crate) fn new(
        admission: DocumentAdmission,
        sync_mode: DocumentSyncMode,
        position_encoding: PositionEncoding,
        sink: Arc<dyn DocumentNotificationSink>,
    ) -> Self {
        Self {
            admission,
            sync_mode,
            position_encoding,
            sink,
            leases: HashMap::new(),
        }
    }

    pub(crate) async fn prepare(
        &mut self,
        relative_path: &str,
        now: Duration,
    ) -> Result<PreparedDocument, DocumentLeaseError> {
        let snapshot = self.admission.read(relative_path)?;
        let key = snapshot.canonical_path();
        let digest = digest(snapshot.text());
        if !self.leases.contains_key(&key) {
            return self.open(key, snapshot, digest, now).await;
        }

        let lease = self
            .leases
            .get(&key)
            .ok_or(DocumentLeaseError::Notification)?;
        if lease.digest == digest && !lease.invalidated {
            let lease = self
                .leases
                .get_mut(&key)
                .ok_or(DocumentLeaseError::Notification)?;
            lease.last_used = now;
            return Ok(prepared_from(lease));
        }

        let version = lease
            .version
            .next()
            .map_err(|_| DocumentLeaseError::VersionOverflow)?;
        let params = change_params(
            &lease.uri,
            version,
            &lease.text,
            snapshot.text(),
            self.sync_mode,
            self.position_encoding,
        )?;
        self.ensure_capacity(Some(&key), snapshot.text().len())
            .await?;
        self.sink
            .notify("textDocument/didChange", params)
            .await
            .map_err(|_| DocumentLeaseError::Notification)?;
        let lease = self
            .leases
            .get_mut(&key)
            .ok_or(DocumentLeaseError::Notification)?;
        lease.version = version;
        lease.digest = digest;
        lease.text = snapshot.text().to_owned();
        lease.last_used = now;
        lease.invalidated = false;
        Ok(prepared_from(lease))
    }

    pub(crate) fn invalidate(&mut self, relative_path: &str) -> bool {
        let normalized = relative_path.replace('\\', "/");
        let mut matched = false;
        for lease in self.leases.values_mut() {
            if lease.relative_path == normalized {
                lease.invalidated = true;
                matched = true;
            }
        }
        matched
    }

    pub(crate) async fn close_idle(&mut self, now: Duration) -> Result<usize, DocumentLeaseError> {
        let keys = self
            .leases
            .iter()
            .filter(|(_, lease)| now.saturating_sub(lease.last_used) >= IDLE_DOCUMENT_LEASE_TIMEOUT)
            .map(|(path, _)| path.clone())
            .collect::<Vec<_>>();
        self.close_keys(keys).await
    }

    pub(crate) async fn close_all(&mut self) -> Result<usize, DocumentLeaseError> {
        self.close_keys(self.leases.keys().cloned().collect()).await
    }

    pub(crate) fn server_restarted(&mut self) {
        self.leases.clear();
    }

    pub(crate) fn retained_bytes(&self) -> usize {
        self.leases.values().map(|lease| lease.text.len()).sum()
    }

    pub(crate) fn active_count(&self) -> usize {
        self.leases.len()
    }

    pub(crate) fn prepared_by_uri(&self, uri: &Url) -> Option<PreparedDocument> {
        self.leases
            .values()
            .find(|lease| lease.uri == *uri)
            .map(prepared_from)
    }

    async fn open(
        &mut self,
        key: PathBuf,
        snapshot: DiskDocumentSnapshot,
        digest: [u8; 32],
        now: Duration,
    ) -> Result<PreparedDocument, DocumentLeaseError> {
        self.ensure_capacity(None, snapshot.text().len()).await?;
        let uri = Url::from_file_path(&key).map_err(|_| DocumentLeaseError::InvalidUri)?;
        let version = DocumentVersion::initial();
        let params = json!({
            "textDocument": {
                "uri": uri,
                "languageId": snapshot.language_id(),
                "version": version.value(),
                "text": snapshot.text(),
            }
        });
        self.sink
            .notify("textDocument/didOpen", params)
            .await
            .map_err(|_| DocumentLeaseError::Notification)?;
        let lease = DocumentLease {
            relative_path: snapshot.relative_path().to_owned(),
            uri,
            version,
            digest,
            text: snapshot.text().to_owned(),
            last_used: now,
            invalidated: false,
        };
        let prepared = prepared_from(&lease);
        self.leases.insert(key, lease);
        Ok(prepared)
    }

    async fn ensure_capacity(
        &mut self,
        replacing: Option<&PathBuf>,
        new_len: usize,
    ) -> Result<(), DocumentLeaseError> {
        loop {
            let replaced_len = replacing
                .and_then(|key| self.leases.get(key))
                .map(|lease| lease.text.len())
                .unwrap_or(0);
            let retained_after = self
                .retained_bytes()
                .saturating_sub(replaced_len)
                .saturating_add(new_len);
            let active_after = self.leases.len() + usize::from(replacing.is_none());
            if active_after <= MAX_DOCUMENT_LEASES && retained_after <= MAX_RETAINED_DOCUMENT_BYTES
            {
                return Ok(());
            }
            let candidate = self
                .leases
                .iter()
                .filter(|(key, _)| replacing != Some(*key))
                .min_by(|(_, left), (_, right)| {
                    (left.last_used, &left.relative_path)
                        .cmp(&(right.last_used, &right.relative_path))
                })
                .map(|(key, _)| key.clone())
                .ok_or(DocumentLeaseError::Notification)?;
            self.close_keys(vec![candidate]).await?;
        }
    }

    async fn close_keys(&mut self, keys: Vec<PathBuf>) -> Result<usize, DocumentLeaseError> {
        let mut closed = 0;
        for key in keys {
            let Some(lease) = self.leases.get(&key) else {
                continue;
            };
            self.sink
                .notify(
                    "textDocument/didClose",
                    json!({ "textDocument": { "uri": lease.uri } }),
                )
                .await
                .map_err(|_| DocumentLeaseError::Notification)?;
            self.leases.remove(&key);
            closed += 1;
        }
        Ok(closed)
    }
}

fn prepared_from(lease: &DocumentLease) -> PreparedDocument {
    PreparedDocument {
        uri: lease.uri.clone(),
        version: lease.version,
        text: lease.text.clone(),
    }
}

fn digest(text: &str) -> [u8; 32] {
    Sha256::digest(text.as_bytes()).into()
}

fn change_params(
    uri: &Url,
    version: DocumentVersion,
    old_text: &str,
    new_text: &str,
    sync_mode: DocumentSyncMode,
    encoding: PositionEncoding,
) -> Result<Value, DocumentLeaseError> {
    let changes = match sync_mode {
        DocumentSyncMode::None => return Err(DocumentLeaseError::UnsupportedSynchronization),
        DocumentSyncMode::Full => json!([{ "text": new_text }]),
        DocumentSyncMode::Incremental => {
            let (old_start, old_end, new_start, new_end) = contiguous_change(old_text, new_text);
            let converter = PositionConverter::new(old_text, encoding);
            let start = converter
                .agent_to_lsp(agent_position_at(old_text, old_start)?)
                .map_err(map_position_error)?;
            let end = converter
                .agent_to_lsp(agent_position_at(old_text, old_end)?)
                .map_err(map_position_error)?;
            json!([{ "range": { "start": start, "end": end }, "text": &new_text[new_start..new_end] }])
        }
    };
    Ok(json!({
        "textDocument": { "uri": uri, "version": version.value() },
        "contentChanges": changes,
    }))
}

fn contiguous_change(old: &str, new: &str) -> (usize, usize, usize, usize) {
    let mut prefix = 0;
    for (old_character, new_character) in old.chars().zip(new.chars()) {
        if old_character != new_character {
            break;
        }
        prefix += old_character.len_utf8();
    }
    let mut old_end = old.len();
    let mut new_end = new.len();
    let old_tail = &old[prefix..];
    let new_tail = &new[prefix..];
    for ((old_index, old_character), (new_index, new_character)) in old_tail
        .char_indices()
        .rev()
        .zip(new_tail.char_indices().rev())
    {
        if old_character != new_character {
            break;
        }
        old_end = prefix + old_index;
        new_end = prefix + new_index;
    }
    (prefix, old_end, prefix, new_end)
}

fn agent_position_at(text: &str, byte_offset: usize) -> Result<AgentPosition, DocumentLeaseError> {
    if byte_offset > text.len() || !text.is_char_boundary(byte_offset) {
        return Err(DocumentLeaseError::PositionConversion);
    }
    let prefix = &text[..byte_offset];
    let line = prefix.bytes().filter(|byte| *byte == b'\n').count() + 1;
    let line_text = prefix.rsplit('\n').next().unwrap_or(prefix);
    let line_text = line_text.strip_suffix('\r').unwrap_or(line_text);
    let column = line_text.chars().count() + 1;
    Ok(AgentPosition::new(
        u32::try_from(line).map_err(|_| DocumentLeaseError::PositionConversion)?,
        u32::try_from(column).map_err(|_| DocumentLeaseError::PositionConversion)?,
    ))
}

fn map_position_error(_: PositionConversionError) -> DocumentLeaseError {
    DocumentLeaseError::PositionConversion
}
