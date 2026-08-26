use super::registry::LanguageDefinition;
use thiserror::Error;

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub(crate) enum DomainModelError {
    #[error("{0} must not be empty")]
    EmptyValue(&'static str),
    #[error("document version cannot advance")]
    DocumentVersionOverflow,
    #[error("workspace trust revision cannot advance")]
    TrustRevisionOverflow,
    #[error("normalized range coordinates must be one-based and ordered")]
    InvalidRange,
    #[error("ready query outcomes require a value")]
    ReadyOutcomeWithoutValue,
    #[error("startup arguments must be a bounded list of strings")]
    InvalidStartupArguments,
    #[error("executable override must be an absolute path")]
    InvalidExecutableOverride,
    #[error("initialization options must be a bounded JSON object")]
    InvalidInitializationOptions,
    #[error("workspace root must resolve to an existing canonical directory")]
    InvalidWorkspaceRoot,
    #[error("code intelligence storage operation failed")]
    Storage,
}

/// A registered language, carrying both its own id and its server's. The two used to be separate
/// enums that every call site had to keep in agreement; as one reference they cannot disagree, and
/// the reference is `Copy` where an owned id would not be.
pub(crate) type Language = &'static LanguageDefinition;

/// Resolves a stored or wire-supplied language id against the registry.
///
/// Unlike the enum this replaces, an unregistered id is an ordinary `None` rather than a parse
/// error: storage no longer constrains the id set, so a row naming a language this build does not
/// register is a case every reader has to handle rather than an impossibility.
pub(crate) fn resolve_language(language_id: &str) -> Option<Language> {
    super::registry::definition(language_id)
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct ConfigurationFingerprint(String);

impl ConfigurationFingerprint {
    pub(crate) fn new(value: impl Into<String>) -> Result<Self, DomainModelError> {
        let value = value.into();
        if value.trim().is_empty() {
            return Err(DomainModelError::EmptyValue("configuration fingerprint"));
        }
        Ok(Self(value))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct WorkspaceTrust {
    canonical_root: String,
    trusted: bool,
    revision: u64,
}

impl WorkspaceTrust {
    pub(crate) fn new(
        canonical_root: impl Into<String>,
        trusted: bool,
        revision: u64,
    ) -> Result<Self, DomainModelError> {
        let canonical_root = canonical_root.into();
        if canonical_root.trim().is_empty() {
            return Err(DomainModelError::EmptyValue("canonical workspace root"));
        }
        Ok(Self {
            canonical_root,
            trusted,
            revision,
        })
    }

    pub(crate) const fn is_trusted(&self) -> bool {
        self.trusted
    }

    pub(crate) const fn revision(&self) -> u64 {
        self.revision
    }

    pub(crate) fn canonical_root(&self) -> &str {
        &self.canonical_root
    }

    pub(crate) fn with_trusted(&self, trusted: bool) -> Result<Self, DomainModelError> {
        let revision = self
            .revision
            .checked_add(1)
            .ok_or(DomainModelError::TrustRevisionOverflow)?;
        Ok(Self {
            canonical_root: self.canonical_root.clone(),
            trusted,
            revision,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ProcessState {
    Absent,
    Starting,
    Initializing,
    Ready,
    Stopping,
    Backoff,
    Failed,
}

impl ProcessState {
    pub(crate) const fn is_warming(self) -> bool {
        matches!(self, Self::Starting | Self::Initializing)
    }

    pub(crate) const fn is_ready(self) -> bool {
        matches!(self, Self::Ready)
    }

    pub(crate) const fn is_terminal(self) -> bool {
        matches!(self, Self::Failed)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PositionEncoding {
    Utf8,
    Utf16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DocumentSyncMode {
    None,
    Full,
    Incremental,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum SemanticMethod {
    Definition,
    References,
    Hover,
    Diagnostics,
    TypeDefinition,
    Implementation,
    WorkspaceSymbols,
    DocumentSymbols,
}

impl SemanticMethod {
    /// Every method this client implements, in the order every negotiated record lists them. Two
    /// servers negotiating the same set therefore report it identically, and nothing that renders
    /// the list has to sort it to be deterministic.
    ///
    /// A variant missing from here is negotiated for no server and offered to nobody. The
    /// compiler cannot catch that, so `all_lists_every_semantic_method` does.
    ///
    /// Append, never insert. The order is what the settings card renders, and reordering it moves
    /// rows under a reader for no reason a reader can see.
    pub(crate) const ALL: &'static [Self] = &[
        Self::Definition,
        Self::References,
        Self::Hover,
        Self::Diagnostics,
        Self::TypeDefinition,
        Self::Implementation,
        Self::WorkspaceSymbols,
        Self::DocumentSymbols,
    ];

    /// Stable wire and localization identifier. Not the LSP method name: that is a protocol
    /// detail the transport owns, while this crosses the command boundary.
    pub(crate) const fn id(self) -> &'static str {
        match self {
            Self::Definition => "definition",
            Self::References => "references",
            Self::Hover => "hover",
            Self::Diagnostics => "diagnostics",
            Self::TypeDefinition => "type_definition",
            Self::Implementation => "implementation",
            Self::WorkspaceSymbols => "workspace_symbols",
            Self::DocumentSymbols => "document_symbols",
        }
    }
}

/// One method the client implements, and whether this server advertised it.
///
/// `supported: false` is deliberately different from the method being absent. Absent means the
/// client does not implement it at all; present-and-false means the server does not offer it, and
/// only the second is something a user can fix by changing servers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct NegotiatedMethod {
    pub(crate) method: SemanticMethod,
    pub(crate) supported: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NegotiatedCapabilities {
    // Position encoding and synchronization stay fields. Neither has a supported-or-not axis, so
    // folding them into the method list would mean inventing a `supported` value for a setting.
    pub(crate) position_encoding: PositionEncoding,
    pub(crate) document_sync: DocumentSyncMode,
    pub(crate) methods: Vec<NegotiatedMethod>,
}

impl NegotiatedCapabilities {
    pub(crate) fn supports(&self, method: SemanticMethod) -> bool {
        self.methods
            .iter()
            .any(|entry| entry.method == method && entry.supported)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct DocumentVersion(u64);

impl DocumentVersion {
    pub(crate) const fn initial() -> Self {
        Self(1)
    }

    pub(crate) const fn new(value: u64) -> Self {
        Self(value)
    }

    pub(crate) const fn value(self) -> u64 {
        self.0
    }

    pub(crate) fn next(self) -> Result<Self, DomainModelError> {
        self.0
            .checked_add(1)
            .map(Self)
            .ok_or(DomainModelError::DocumentVersionOverflow)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct NormalizedRange {
    pub(crate) start_line: u32,
    pub(crate) start_column: u32,
    pub(crate) end_line: u32,
    pub(crate) end_column: u32,
}

impl NormalizedRange {
    pub(crate) fn new(
        start_line: u32,
        start_column: u32,
        end_line: u32,
        end_column: u32,
    ) -> Result<Self, DomainModelError> {
        let starts_at_zero = [start_line, start_column, end_line, end_column].contains(&0);
        let reversed = (end_line, end_column) < (start_line, start_column);
        if starts_at_zero || reversed {
            return Err(DomainModelError::InvalidRange);
        }
        Ok(Self {
            start_line,
            start_column,
            end_line,
            end_column,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NormalizedLocation {
    file: String,
    pub(crate) range: NormalizedRange,
    pub(crate) preview: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NormalizedHover {
    pub(crate) signature: Option<String>,
    pub(crate) documentation: Option<String>,
    pub(crate) range: Option<NormalizedRange>,
    pub(crate) truncated: bool,
}

impl NormalizedLocation {
    pub(crate) fn new(
        file: impl Into<String>,
        range: NormalizedRange,
        preview: Option<String>,
    ) -> Result<Self, DomainModelError> {
        let file = file.into();
        if file.trim().is_empty() {
            return Err(DomainModelError::EmptyValue("normalized relative file"));
        }
        Ok(Self {
            file,
            range,
            preview,
        })
    }

    pub(crate) fn file(&self) -> &str {
        &self.file
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DiagnosticSeverity {
    Error,
    Warning,
    Information,
    Hint,
}

/// A symbol as the Agent sees it: workspace-relative, with its enclosing symbol named so a
/// flattened list still says where each entry sits.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NormalizedSymbol {
    pub(crate) name: String,
    /// One of a closed set this build maps from the protocol's numeric kinds, so it is a `&'static
    /// str` rather than whatever the server sent.
    pub(crate) kind: &'static str,
    pub(crate) container: Option<String>,
    pub(crate) location: NormalizedLocation,
}

impl NormalizedSymbol {
    pub(crate) fn new(
        name: impl Into<String>,
        kind: &'static str,
        container: Option<String>,
        location: NormalizedLocation,
    ) -> Result<Self, DomainModelError> {
        let name = name.into();
        if name.trim().is_empty() {
            return Err(DomainModelError::EmptyValue("normalized symbol name"));
        }
        Ok(Self {
            name,
            kind,
            container: container.filter(|value| !value.trim().is_empty()),
            location,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NormalizedDiagnostic {
    pub(crate) range: NormalizedRange,
    pub(crate) severity: Option<DiagnosticSeverity>,
    pub(crate) message: String,
    pub(crate) source: Option<String>,
    pub(crate) code: Option<String>,
    pub(crate) related_information: Vec<NormalizedRelatedDiagnostic>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NormalizedRelatedDiagnostic {
    pub(crate) location: NormalizedLocation,
    pub(crate) message: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DiagnosticSnapshot {
    pub(crate) server_version: Option<DocumentVersion>,
    local_document_version: DocumentVersion,
    diagnostics: Vec<NormalizedDiagnostic>,
    pub(crate) received_at_epoch_ms: u64,
}

impl DiagnosticSnapshot {
    pub(crate) fn new(
        server_version: Option<DocumentVersion>,
        local_document_version: DocumentVersion,
        diagnostics: Vec<NormalizedDiagnostic>,
        received_at_epoch_ms: u64,
    ) -> Self {
        Self {
            server_version,
            local_document_version,
            diagnostics,
            received_at_epoch_ms,
        }
    }

    pub(crate) const fn is_current_for(&self, version: DocumentVersion) -> bool {
        self.local_document_version.0 == version.0
    }

    pub(crate) fn diagnostics(&self) -> &[NormalizedDiagnostic] {
        &self.diagnostics
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum QueryStatus {
    Ready,
    Warming,
    Timeout,
    Unavailable,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct QueryOutcome<T> {
    status: QueryStatus,
    value: Option<T>,
    // One field where there were two. The server was always the one the language declares, so a
    // separate field could only ever agree or be a bug.
    pub(crate) language: Option<Language>,
    document_version: Option<DocumentVersion>,
    pub(crate) stale: bool,
    pub(crate) returned_count: usize,
    pub(crate) total: usize,
    pub(crate) truncated: bool,
    pub(crate) filtered_count: usize,
    reason_code: Option<String>,
}

impl<T> QueryOutcome<T> {
    pub(crate) fn ready(value: T, document_version: u64) -> Self {
        Self {
            status: QueryStatus::Ready,
            value: Some(value),
            language: None,
            document_version: Some(DocumentVersion::new(document_version)),
            stale: false,
            returned_count: 0,
            total: 0,
            truncated: false,
            filtered_count: 0,
            reason_code: None,
        }
    }

    pub(crate) fn degraded(
        status: QueryStatus,
        reason_code: impl Into<String>,
    ) -> Result<Self, DomainModelError> {
        if status == QueryStatus::Ready {
            return Err(DomainModelError::ReadyOutcomeWithoutValue);
        }
        Ok(Self {
            status,
            value: None,
            language: None,
            document_version: None,
            stale: false,
            returned_count: 0,
            total: 0,
            truncated: false,
            filtered_count: 0,
            reason_code: Some(reason_code.into()),
        })
    }

    pub(crate) fn ready_with_metadata(
        value: T,
        language: Language,
        document_version: DocumentVersion,
        returned_count: usize,
        total: usize,
        truncated: bool,
        filtered_count: usize,
    ) -> Self {
        Self {
            status: QueryStatus::Ready,
            value: Some(value),
            language: Some(language),
            document_version: Some(document_version),
            stale: false,
            returned_count,
            total,
            truncated,
            filtered_count,
            reason_code: None,
        }
    }

    /// A ready outcome for a query that names no document. A workspace-wide answer has no version
    /// to report, and inventing one would let a caller compare it against a real one.
    pub(crate) fn ready_without_document(
        value: T,
        language: Language,
        returned_count: usize,
        total: usize,
        truncated: bool,
        filtered_count: usize,
    ) -> Self {
        Self {
            status: QueryStatus::Ready,
            value: Some(value),
            language: Some(language),
            document_version: None,
            stale: false,
            returned_count,
            total,
            truncated,
            filtered_count,
            reason_code: None,
        }
    }

    pub(crate) fn degraded_with_identity(
        status: QueryStatus,
        reason_code: impl Into<String>,
        language: Option<Language>,
        document_version: Option<DocumentVersion>,
    ) -> Self {
        debug_assert_ne!(status, QueryStatus::Ready);
        Self {
            status,
            value: None,
            language,
            document_version,
            stale: false,
            returned_count: 0,
            total: 0,
            truncated: false,
            filtered_count: 0,
            reason_code: Some(reason_code.into()),
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn status_with_value(
        status: QueryStatus,
        value: Option<T>,
        reason_code: Option<&str>,
        language: Language,
        document_version: DocumentVersion,
        stale: bool,
        returned_count: usize,
        total: usize,
        truncated: bool,
        filtered_count: usize,
    ) -> Self {
        Self {
            status,
            value,
            language: Some(language),
            document_version: Some(document_version),
            stale,
            returned_count,
            total,
            truncated,
            filtered_count,
            reason_code: reason_code.map(str::to_owned),
        }
    }

    pub(crate) const fn status(&self) -> QueryStatus {
        self.status
    }

    pub(crate) const fn document_version(&self) -> Option<DocumentVersion> {
        self.document_version
    }

    pub(crate) fn value(&self) -> Option<&T> {
        self.value.as_ref()
    }

    pub(crate) fn into_value(self) -> Option<T> {
        self.value
    }

    pub(crate) fn reason_code(&self) -> Option<&str> {
        self.reason_code.as_deref()
    }
}
