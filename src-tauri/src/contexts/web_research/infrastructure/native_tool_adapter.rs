use crate::contexts::agent_runtime::application::{
    NativeToolErrorCode, NativeToolOperation, NativeToolPortRequest, NativeToolResultEnvelope,
    NativeToolResultStatus, WebResearchPort, NATIVE_TOOL_CONTRACT_VERSION,
};
use crate::contexts::web_research::application::{
    ExtractionLimits, FetchError, FetchLimits, FetchRequest, FetchedBinaryArtifactRequest,
    FetchedBinaryRouter, GuardedFetchService, SearchProviderError, SearchRequest, SearchSafeMode,
    WebContentExtractor,
};
use crate::contexts::web_research::infrastructure::DuckDuckGoSearchAdapter;
use chrono::Utc;
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::time::{Duration, Instant};

pub(crate) struct WebResearchNativeToolAdapter {
    search: DuckDuckGoSearchAdapter,
    fetch: GuardedFetchService,
    binaries: FetchedBinaryRouter,
}

impl WebResearchNativeToolAdapter {
    pub(crate) fn new(
        search: DuckDuckGoSearchAdapter,
        fetch: GuardedFetchService,
        binaries: FetchedBinaryRouter,
    ) -> Self {
        Self {
            search,
            fetch,
            binaries,
        }
    }

    fn execute(&self, request: NativeToolPortRequest) -> Result<Value, AdapterError> {
        if request.context.is_cancelled() {
            return Err(AdapterError::Cancelled);
        }
        if request.context.deadline_reached() {
            return Err(AdapterError::Deadline);
        }
        match request.input.operation {
            NativeToolOperation::WebSearch => self.search(request),
            NativeToolOperation::WebFetch => self.fetch(request),
            _ => Err(AdapterError::InvalidInput),
        }
    }

    fn search(&self, request: NativeToolPortRequest) -> Result<Value, AdapterError> {
        let input = &request.input.value;
        let safe_mode = match input
            .get("safe_search")
            .and_then(Value::as_str)
            .unwrap_or("moderate")
        {
            "strict" => SearchSafeMode::Strict,
            "moderate" => SearchSafeMode::Moderate,
            "off" => SearchSafeMode::Off,
            _ => return Err(AdapterError::InvalidInput),
        };
        let count = u8::try_from(input.get("count").and_then(Value::as_u64).unwrap_or(5))
            .map_err(|_| AdapterError::InvalidInput)?;
        let response = self
            .search
            .search(
                SearchRequest {
                    query: string(input, "query")?.to_owned(),
                    locale: input
                        .get("locale")
                        .and_then(Value::as_str)
                        .unwrap_or("us-en")
                        .to_owned(),
                    safe_mode,
                    count,
                    deadline: request.context.deadline,
                    cancelled: request.context.cancelled,
                },
                Utc::now().to_rfc3339(),
            )
            .map_err(AdapterError::Search)?;
        let results = response
            .results
            .into_iter()
            .map(|result| {
                json!({
                    "rank": result.rank,
                    "title": result.title,
                    "url": result.url,
                    "snippet": result.snippet,
                    "provider": result.provider,
                    "evidence_kind": result.evidence_kind,
                })
            })
            .collect::<Vec<_>>();
        Ok(json!({
            "contract_version": response.contract_version,
            "provider": response.provider,
            "query": response.query,
            "captured_at": response.captured_at,
            "results": results,
            "truncated": response.truncated,
        }))
    }

    fn fetch(&self, request: NativeToolPortRequest) -> Result<Value, AdapterError> {
        let remaining = request
            .context
            .deadline
            .checked_duration_since(Instant::now())
            .ok_or(AdapterError::Deadline)?;
        let limits = FetchLimits {
            max_duration: remaining.min(Duration::from_secs(15)),
            ..FetchLimits::default()
        };
        let fetched = self
            .fetch
            .fetch(FetchRequest {
                url: string(&request.input.value, "url")?.to_owned(),
                limits,
                cancelled: request.context.cancelled,
            })
            .map_err(AdapterError::Fetch)?;
        if matches!(
            fetched.media_type.as_str(),
            "application/pdf" | "image/png" | "image/jpeg"
        ) {
            if !request
                .input
                .value
                .get("persist_binary")
                .and_then(Value::as_bool)
                .unwrap_or(false)
            {
                return Err(AdapterError::BinaryPersistenceRequired);
            }
            let artifact = self
                .binaries
                .route(
                    &FetchedBinaryArtifactRequest {
                        operation_id: request.context.call_id,
                        creator_id: request.context.agent_id,
                        expires_at: None,
                    },
                    &fetched,
                )
                .map_err(|_| AdapterError::Binary)?;
            return Ok(json!({
                "contract_version": artifact.contract_version,
                "artifact_id": artifact.artifact_id,
                "content_hash": artifact.content_hash,
                "size_bytes": artifact.size_bytes,
                "media_type": artifact.media_type,
                "normalized_url": artifact.normalized_url,
                "final_url": artifact.final_url,
                "evidence_kind": artifact.evidence_kind,
            }));
        }
        let max_text_chars = usize::try_from(
            request
                .input
                .value
                .get("max_text_chars")
                .and_then(Value::as_u64)
                .unwrap_or(30_000),
        )
        .map_err(|_| AdapterError::InvalidInput)?;
        let page = WebContentExtractor::extract(fetched, ExtractionLimits { max_text_chars })
            .map_err(|_| AdapterError::UnsafeContent)?;
        Ok(json!({
            "contract_version": page.contract_version,
            "provider": page.provider,
            "evidence_kind": page.evidence_kind,
            "normalized_url": page.normalized_url,
            "final_url": page.final_url,
            "title": page.title,
            "media_type": page.media_type,
            "captured_at": page.captured_at,
            "text": page.text,
            "truncated": page.truncated,
        }))
    }
}

impl WebResearchPort for WebResearchNativeToolAdapter {
    fn execute_web(&self, request: NativeToolPortRequest) -> NativeToolResultEnvelope {
        match self.execute(request) {
            Ok(output) => envelope(NativeToolResultStatus::Succeeded, Some(output), None),
            Err(error) => envelope(error.status(), None, Some(error)),
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum AdapterError {
    InvalidInput,
    Cancelled,
    Deadline,
    Search(SearchProviderError),
    Fetch(FetchError),
    Binary,
    BinaryPersistenceRequired,
    UnsafeContent,
}

impl AdapterError {
    const fn status(self) -> NativeToolResultStatus {
        match self {
            Self::Cancelled | Self::Search(SearchProviderError::Cancelled) => {
                NativeToolResultStatus::Cancelled
            }
            Self::Deadline | Self::Search(SearchProviderError::DeadlineExceeded) => {
                NativeToolResultStatus::Failed
            }
            Self::Fetch(FetchError::Cancelled) => NativeToolResultStatus::Cancelled,
            Self::Fetch(FetchError::CompressedBodyTooLarge | FetchError::ExpandedBodyTooLarge) => {
                NativeToolResultStatus::LimitExceeded
            }
            _ => NativeToolResultStatus::Failed,
        }
    }

    const fn code(self) -> NativeToolErrorCode {
        match self {
            Self::InvalidInput | Self::BinaryPersistenceRequired => {
                NativeToolErrorCode::InvalidInput
            }
            Self::Cancelled
            | Self::Search(SearchProviderError::Cancelled)
            | Self::Fetch(FetchError::Cancelled) => NativeToolErrorCode::Cancelled,
            Self::Deadline
            | Self::Search(SearchProviderError::DeadlineExceeded)
            | Self::Fetch(FetchError::Timeout) => NativeToolErrorCode::DeadlineExceeded,
            Self::Fetch(FetchError::CompressedBodyTooLarge | FetchError::ExpandedBodyTooLarge) => {
                NativeToolErrorCode::LimitExceeded
            }
            Self::Fetch(FetchError::UrlPolicy(_)) => NativeToolErrorCode::PermissionDenied,
            Self::Binary | Self::UnsafeContent => NativeToolErrorCode::IntegrityFailure,
            Self::Search(_) | Self::Fetch(_) => NativeToolErrorCode::ExternalFailure,
        }
    }

    const fn message(self) -> &'static str {
        match self {
            Self::InvalidInput => "Web tool input is invalid.",
            Self::Cancelled
            | Self::Search(SearchProviderError::Cancelled)
            | Self::Fetch(FetchError::Cancelled) => "Web operation was cancelled.",
            Self::Deadline
            | Self::Search(SearchProviderError::DeadlineExceeded)
            | Self::Fetch(FetchError::Timeout) => "Web operation deadline was reached.",
            Self::BinaryPersistenceRequired => "Binary Web content requires Artifact persistence.",
            Self::Fetch(FetchError::UrlPolicy(_)) => "Web target is not a permitted public URL.",
            Self::Fetch(FetchError::CompressedBodyTooLarge | FetchError::ExpandedBodyTooLarge) => {
                "Web response exceeded the configured limit."
            }
            Self::Binary | Self::UnsafeContent => "Web content could not be admitted safely.",
            Self::Search(_) => "Web search provider is unavailable.",
            Self::Fetch(_) => "Web page could not be fetched safely.",
        }
    }
}

fn string<'a>(input: &'a Value, name: &str) -> Result<&'a str, AdapterError> {
    input
        .get(name)
        .and_then(Value::as_str)
        .ok_or(AdapterError::InvalidInput)
}

fn envelope(
    status: NativeToolResultStatus,
    output: Option<Value>,
    error: Option<AdapterError>,
) -> NativeToolResultEnvelope {
    NativeToolResultEnvelope {
        contract_version: NATIVE_TOOL_CONTRACT_VERSION,
        status,
        output,
        error_code: error.map(AdapterError::code),
        safe_error: error.map(|value| value.message().to_owned()),
        truncated: status == NativeToolResultStatus::LimitExceeded,
        metadata: BTreeMap::new(),
    }
}
