use super::json_rpc_actor::{JsonRpcClient, JsonRpcError};
use crate::contexts::code_intelligence::domain::models::{
    DocumentSyncMode, NegotiatedCapabilities, NegotiatedMethod, PositionEncoding, SemanticMethod,
};
use lsp_types::{
    HoverProviderCapability, ImplementationProviderCapability, InitializeResult, OneOf,
    PositionEncodingKind, ServerCapabilities, TextDocumentSyncCapability, TextDocumentSyncKind,
    TypeDefinitionProviderCapability,
};
use serde_json::{json, Value};
use thiserror::Error;

#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
pub(crate) enum InitializeNegotiationError {
    #[error("initialize transport failed")]
    Transport(#[from] JsonRpcError),
    #[error("initialize result is malformed")]
    MalformedResult,
    #[error("server selected an unsupported position encoding")]
    UnsupportedPositionEncoding,
    #[error("server selected an unsupported synchronization mode")]
    UnsupportedSynchronization,
}

pub(crate) async fn initialize_and_notify(
    client: &JsonRpcClient,
    params: Value,
) -> Result<NegotiatedCapabilities, InitializeNegotiationError> {
    let result: Value = client.request("initialize", params).await?;
    let negotiated = negotiate_initialize_result(result)?;
    client.notify("initialized", json!({})).await?;
    Ok(negotiated)
}

pub(crate) fn build_initialize_params(
    canonical_root_uri: &str,
    initialization_options: Value,
    process_id: Option<u32>,
) -> Value {
    json!({
        "processId": process_id,
        "clientInfo": {"name": "VaneHub AI", "version": env!("CARGO_PKG_VERSION")},
        "rootUri": canonical_root_uri,
        "workspaceFolders": [{"uri": canonical_root_uri, "name": "workspace"}],
        "capabilities": {
            "general": {"positionEncodings": ["utf-8", "utf-16"]},
            "window": {"workDoneProgress": true},
            "workspace": {
                "configuration": true,
                "didChangeConfiguration": {"dynamicRegistration": true},
                "workspaceFolders": true
            },
            "textDocument": {
                "synchronization": {
                    "dynamicRegistration": true,
                    "willSave": false,
                    "willSaveWaitUntil": false,
                    "didSave": true
                },
                "definition": {"dynamicRegistration": true, "linkSupport": true},
                "typeDefinition": {"dynamicRegistration": true, "linkSupport": true},
                "implementation": {"dynamicRegistration": true, "linkSupport": true},
                "references": {"dynamicRegistration": true},
                "hover": {
                    "dynamicRegistration": true,
                    "contentFormat": ["markdown", "plaintext"]
                },
                "publishDiagnostics": {
                    "relatedInformation": true,
                    "versionSupport": true
                }
            }
        },
        "initializationOptions": initialization_options
    })
}

pub(crate) fn negotiate_initialize_result(
    value: Value,
) -> Result<NegotiatedCapabilities, InitializeNegotiationError> {
    let result: InitializeResult =
        serde_json::from_value(value).map_err(|_| InitializeNegotiationError::MalformedResult)?;
    let capabilities = result.capabilities;
    let position_encoding = match &capabilities.position_encoding {
        None => PositionEncoding::Utf16,
        Some(encoding) if *encoding == PositionEncodingKind::UTF8 => PositionEncoding::Utf8,
        Some(encoding) if *encoding == PositionEncodingKind::UTF16 => PositionEncoding::Utf16,
        Some(_) => return Err(InitializeNegotiationError::UnsupportedPositionEncoding),
    };
    let document_sync = normalize_sync(capabilities.text_document_sync.clone())?;
    Ok(NegotiatedCapabilities {
        position_encoding,
        document_sync,
        // Built by iterating the client's own method list, so every record covers exactly the
        // methods this build implements and lists them in one order. A capability the server
        // advertises for something we do not implement is simply never asked about.
        methods: SemanticMethod::ALL
            .iter()
            .map(|method| NegotiatedMethod {
                method: *method,
                supported: advertised(&capabilities, *method),
            })
            .collect(),
    })
}

/// The one place a method is tied to the field the server advertises it in. This match is
/// exhaustive, so adding a `SemanticMethod` variant without deciding how it is advertised does not
/// compile — the compiler guarantee that `supports` used to carry lives here now.
fn advertised(capabilities: &ServerCapabilities, method: SemanticMethod) -> bool {
    match method {
        SemanticMethod::Definition => one_of_enabled(capabilities.definition_provider.clone()),
        SemanticMethod::References => one_of_enabled(capabilities.references_provider.clone()),
        SemanticMethod::Hover => hover_enabled(capabilities.hover_provider.clone()),
        // Diagnostics arrive as a server-initiated notification rather than a capability the
        // server advertises, so there is nothing to read and nothing that can be unsupported.
        SemanticMethod::Diagnostics => true,
        // These two carry their own provider types rather than `OneOf`, so they cannot go through
        // `one_of_enabled`. The shape is otherwise the same: absent or `false` means no.
        SemanticMethod::TypeDefinition => matches!(
            &capabilities.type_definition_provider,
            Some(
                TypeDefinitionProviderCapability::Options(_)
                    | TypeDefinitionProviderCapability::Simple(true)
            )
        ),
        SemanticMethod::Implementation => matches!(
            &capabilities.implementation_provider,
            Some(
                ImplementationProviderCapability::Options(_)
                    | ImplementationProviderCapability::Simple(true)
            )
        ),
    }
}

fn normalize_sync(
    capability: Option<TextDocumentSyncCapability>,
) -> Result<DocumentSyncMode, InitializeNegotiationError> {
    let kind = match capability {
        None => TextDocumentSyncKind::NONE,
        Some(TextDocumentSyncCapability::Kind(kind)) => kind,
        Some(TextDocumentSyncCapability::Options(options)) => {
            options.change.unwrap_or(TextDocumentSyncKind::NONE)
        }
    };
    if kind == TextDocumentSyncKind::NONE {
        Ok(DocumentSyncMode::None)
    } else if kind == TextDocumentSyncKind::FULL {
        Ok(DocumentSyncMode::Full)
    } else if kind == TextDocumentSyncKind::INCREMENTAL {
        Ok(DocumentSyncMode::Incremental)
    } else {
        Err(InitializeNegotiationError::UnsupportedSynchronization)
    }
}

fn one_of_enabled<T>(capability: Option<OneOf<bool, T>>) -> bool {
    capability.is_some_and(|value| match value {
        OneOf::Left(enabled) => enabled,
        OneOf::Right(_) => true,
    })
}

fn hover_enabled(capability: Option<HoverProviderCapability>) -> bool {
    capability.is_some_and(|value| match value {
        HoverProviderCapability::Simple(enabled) => enabled,
        HoverProviderCapability::Options(_) => true,
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum IndexingProgress {
    Idle,
    Running,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RuntimeReadiness {
    protocol_ready: bool,
    indexing_progress: IndexingProgress,
}

impl RuntimeReadiness {
    pub(crate) const fn protocol_ready() -> Self {
        Self {
            protocol_ready: true,
            indexing_progress: IndexingProgress::Idle,
        }
    }

    pub(crate) const fn is_protocol_ready(self) -> bool {
        self.protocol_ready
    }

    pub(crate) const fn indexing_progress(self) -> IndexingProgress {
        self.indexing_progress
    }

    pub(crate) fn observe_indexing(&mut self, running: bool) {
        self.indexing_progress = if running {
            IndexingProgress::Running
        } else {
            IndexingProgress::Idle
        };
    }
}

pub(crate) fn supports_method(
    capabilities: &NegotiatedCapabilities,
    method: SemanticMethod,
) -> bool {
    capabilities.supports(method)
}
