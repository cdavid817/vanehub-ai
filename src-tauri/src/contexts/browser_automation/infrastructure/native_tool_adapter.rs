use super::native_tool_adapter_support::{
    action, action_input, ensure_claimed_origin, envelope, string, AdapterError,
};
use super::BrowserHandoffCommandAdapter;
use crate::contexts::agent_runtime::application::{
    BrowserAutomationPort, NativeToolPortRequest, NativeToolResultEnvelope, NativeToolResultStatus,
};
use crate::contexts::browser_automation::application::{
    BrowserAction, BrowserActionPolicy, BrowserArtifactBridge, BrowserOperationRequest,
    BrowserOperationResult, BrowserOperationService, BrowserOwnership,
};
use crate::contexts::web_research::application::{GuardedUrlPolicy, UrlResolverPort};
use serde_json::{json, Map, Value};
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

pub(crate) struct BrowserNativeToolAdapter {
    operations: Arc<BrowserOperationService>,
    resolver: Arc<dyn UrlResolverPort>,
    artifacts: Option<Arc<BrowserArtifactBridge>>,
    handoff_control: Arc<BrowserHandoffCommandAdapter>,
    page_urls: Mutex<BTreeMap<(BrowserOwnership, String), String>>,
}

impl BrowserNativeToolAdapter {
    pub(crate) fn with_handoff_control(
        operations: Arc<BrowserOperationService>,
        resolver: Arc<dyn UrlResolverPort>,
        artifacts: Option<Arc<BrowserArtifactBridge>>,
        handoff_control: Arc<BrowserHandoffCommandAdapter>,
    ) -> Self {
        Self {
            operations,
            resolver,
            artifacts,
            handoff_control,
            page_urls: Mutex::new(BTreeMap::new()),
        }
    }

    fn execute(&self, request: NativeToolPortRequest) -> Result<Value, AdapterError> {
        if request.context.is_cancelled() {
            return Err(AdapterError::Cancelled);
        }
        if request.context.deadline_reached() {
            return Err(AdapterError::Deadline);
        }
        let object = request
            .input
            .value
            .as_object()
            .ok_or(AdapterError::InvalidInput)?;
        let operation = string(object, "operation")?;
        let ownership = BrowserOwnership {
            session_id: request.context.session_id.clone(),
            generation_id: request.context.generation_id.clone(),
        };
        match operation {
            "start" => {
                self.operations
                    .start(ownership)
                    .map_err(|_| AdapterError::BrowserFailure)?;
                Ok(json!({"contract_version": 1, "status": "ready"}))
            }
            "handoff" => self.begin_handoff(ownership, object),
            "resume" => Err(AdapterError::ExplicitUserResumeRequired),
            "close" => {
                self.operations
                    .close(&ownership)
                    .map_err(|_| AdapterError::BrowserFailure)?;
                self.remove_owned_pages(&ownership)?;
                Ok(json!({"contract_version": 1, "closed": true}))
            }
            _ => self.execute_page_action(ownership, operation, object, &request.context.call_id),
        }
    }

    fn begin_handoff(
        &self,
        ownership: BrowserOwnership,
        object: &Map<String, Value>,
    ) -> Result<Value, AdapterError> {
        let page_id = string(object, "page_id")?.to_owned();
        let seconds = object
            .get("handoff_seconds")
            .and_then(Value::as_u64)
            .ok_or(AdapterError::InvalidInput)?;
        let handoff = self
            .operations
            .begin_handoff(ownership, page_id, Duration::from_secs(seconds))
            .map_err(|_| AdapterError::BrowserFailure)?;
        Ok(json!({
            "contract_version": 1,
            "handoff_id": handoff.handoff_id,
            "page_id": handoff.page_id,
            "status": "awaiting_user"
        }))
    }

    fn execute_page_action(
        &self,
        ownership: BrowserOwnership,
        operation: &str,
        object: &Map<String, Value>,
        call_id: &str,
    ) -> Result<Value, AdapterError> {
        let action = action(operation)?;
        let page_id = object
            .get("page_id")
            .and_then(Value::as_str)
            .map(str::to_owned);
        let current_url = page_id
            .as_ref()
            .and_then(|page| self.current_url(&ownership, page).ok().flatten());
        if operation != "navigate" {
            ensure_claimed_origin(object, current_url.as_deref())?;
        }
        let browser_request = BrowserOperationRequest {
            ownership: ownership.clone(),
            action,
            page_id,
            input: action_input(operation, object),
        };
        let witness = BrowserActionPolicy::prepare(
            &browser_request,
            current_url.as_deref(),
            self.resolver.as_ref(),
        )
        .map_err(|_| AdapterError::UnsafeTarget)?;
        BrowserActionPolicy::revalidate(
            &witness,
            &browser_request,
            current_url.as_deref(),
            self.resolver.as_ref(),
        )
        .map_err(|_| AdapterError::StaleApproval)?;
        let result = self
            .operations
            .execute(browser_request)
            .map_err(|_| AdapterError::BrowserFailure)?;
        self.admit_result_url(&ownership, &result)?;
        self.handoff_control
            .record_page(call_id, ownership, result.page_id.clone())
            .map_err(|_| AdapterError::Internal)?;
        self.project_result(result, call_id)
    }

    fn admit_result_url(
        &self,
        ownership: &BrowserOwnership,
        result: &BrowserOperationResult,
    ) -> Result<(), AdapterError> {
        if let Some(url) = &result.url {
            let admitted = GuardedUrlPolicy::resolve_public(url, self.resolver.as_ref())
                .map_err(|_| AdapterError::UnsafeTarget)?;
            self.page_urls
                .lock()
                .map_err(|_| AdapterError::Internal)?
                .insert(
                    (ownership.clone(), result.page_id.clone()),
                    admitted.normalized_url,
                );
        }
        Ok(())
    }

    fn project_result(
        &self,
        mut result: BrowserOperationResult,
        call_id: &str,
    ) -> Result<Value, AdapterError> {
        if result.action == BrowserAction::Screenshot {
            let media_type = result
                .payload
                .get("media_type")
                .and_then(Value::as_str)
                .ok_or(AdapterError::UnsafeResult)?;
            let bytes = result
                .payload
                .get("bytes_base64")
                .and_then(Value::as_str)
                .ok_or(AdapterError::UnsafeResult)?;
            let artifact = self
                .artifacts
                .as_ref()
                .ok_or(AdapterError::Unavailable)?
                .seal_download(call_id, "browser-screenshot.png", media_type, bytes)
                .map_err(|_| AdapterError::UnsafeResult)?;
            result.payload = json!({
                "artifact_id": artifact.artifact_id,
                "content_hash": artifact.content_hash,
                "media_type": artifact.media_type,
                "size_bytes": artifact.size_bytes
            });
        }
        Ok(json!({
            "contract_version": result.contract_version,
            "page_id": result.page_id,
            "frame_id": result.frame_id,
            "url": result.url,
            "payload": result.payload,
            "truncated": result.truncated
        }))
    }

    fn current_url(
        &self,
        ownership: &BrowserOwnership,
        page_id: &str,
    ) -> Result<Option<String>, AdapterError> {
        Ok(self
            .page_urls
            .lock()
            .map_err(|_| AdapterError::Internal)?
            .get(&(ownership.clone(), page_id.to_owned()))
            .cloned())
    }

    fn remove_owned_pages(&self, ownership: &BrowserOwnership) -> Result<(), AdapterError> {
        self.page_urls
            .lock()
            .map_err(|_| AdapterError::Internal)?
            .retain(|(owner, _), _| owner != ownership);
        Ok(())
    }
}

impl BrowserAutomationPort for BrowserNativeToolAdapter {
    fn execute_browser(&self, request: NativeToolPortRequest) -> NativeToolResultEnvelope {
        match self.execute(request) {
            Ok(output) => envelope(NativeToolResultStatus::Succeeded, Some(output), None),
            Err(error) => envelope(error.status(), None, Some(error)),
        }
    }
}
