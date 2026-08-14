use super::{
    ManualNativeToolOperationPort, ManualNativeToolRequest, NativeToolProgress,
    NativeToolProgressPhase, NativeToolProgressSink, NativeToolResultEnvelope,
    NativeToolResultStatus, StoredToolOperation, StoredToolOperationStatus,
};
use chrono::Utc;
use serde_json::Value;
use std::sync::{Arc, Mutex};

pub(super) struct ManualOperationRecorder {
    port: Arc<dyn ManualNativeToolOperationPort>,
    record: Mutex<StoredToolOperation>,
}

impl std::fmt::Debug for ManualOperationRecorder {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ManualOperationRecorder")
            .finish_non_exhaustive()
    }
}

impl ManualOperationRecorder {
    pub(super) fn new(
        port: Arc<dyn ManualNativeToolOperationPort>,
        id: &str,
        request: &ManualNativeToolRequest,
        generation_id: &str,
    ) -> Self {
        let now = Utc::now().to_rfc3339();
        let recorder = Self {
            port,
            record: Mutex::new(StoredToolOperation {
                contract_version: 1,
                id: id.to_owned(),
                session_id: request.session_id.clone(),
                generation_id: generation_id.to_owned(),
                tool_name: request.tool_name.clone(),
                status: StoredToolOperationStatus::Queued,
                progress_sequence: 0,
                progress_message: None,
                result_artifact_ids: Vec::new(),
                error_code: None,
                created_at: now.clone(),
                updated_at: now,
            }),
        };
        recorder.persist();
        recorder
    }

    pub(super) fn transition(
        &self,
        status: StoredToolOperationStatus,
        error: Option<String>,
        artifacts: Vec<String>,
    ) {
        if let Ok(mut record) = self.record.lock() {
            record.status = status;
            record.progress_sequence = record.progress_sequence.saturating_add(1);
            record.error_code = error;
            record.result_artifact_ids = artifacts;
            record.updated_at = Utc::now().to_rfc3339();
        }
        self.persist();
    }

    pub(super) fn complete(&self, result: &NativeToolResultEnvelope) {
        let status = match result.status {
            NativeToolResultStatus::Succeeded => StoredToolOperationStatus::Succeeded,
            NativeToolResultStatus::Cancelled => StoredToolOperationStatus::Cancelled,
            _ => StoredToolOperationStatus::Failed,
        };
        self.transition(
            status,
            result.error_code.map(|code| code.as_str().to_owned()),
            artifact_ids(result.output.as_ref()),
        );
    }

    fn persist(&self) {
        if let Ok(record) = self.record.lock() {
            let _ = self.port.save(&record);
        }
    }
}

impl NativeToolProgressSink for ManualOperationRecorder {
    fn publish(&self, progress: NativeToolProgress) {
        if let Ok(mut record) = self.record.lock() {
            record.progress_sequence = record.progress_sequence.saturating_add(1);
            record.status = if progress.phase == NativeToolProgressPhase::AwaitingHuman {
                StoredToolOperationStatus::AwaitingHuman
            } else {
                StoredToolOperationStatus::Running
            };
            record.progress_message = progress.message;
            record.updated_at = Utc::now().to_rfc3339();
        }
        self.persist();
    }
}

fn artifact_ids(output: Option<&Value>) -> Vec<String> {
    fn visit(value: &Value, ids: &mut Vec<String>) {
        match value {
            Value::String(value) if value.starts_with("artifact-") && ids.len() < 64 => {
                if !ids.contains(value) {
                    ids.push(value.clone());
                }
            }
            Value::Array(values) => values.iter().for_each(|value| visit(value, ids)),
            Value::Object(values) => values.values().for_each(|value| visit(value, ids)),
            _ => {}
        }
    }
    let mut ids = Vec::new();
    if let Some(output) = output {
        visit(output, &mut ids);
    }
    ids
}
