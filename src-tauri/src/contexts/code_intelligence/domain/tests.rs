use super::models::*;
use crate::contexts::code_intelligence::domain::registry;

#[test]
fn resolving_a_language_id_yields_the_registry_entry_and_its_server() {
    // Replaces an assertion that a two-variant enum mapped to a two-variant server enum. The
    // mapping cannot be wrong any more -- there is one value -- so what is worth asserting now is
    // that resolution finds the entry and that an unregistered id resolves to nothing.
    let rust = resolve_language("rust").expect("rust resolves");
    assert_eq!(rust, registry::rust());
    assert_eq!(rust.id, "rust");
    assert_eq!(rust.server_id, "rust_analyzer");

    let typescript = resolve_language("typescript_javascript").expect("typescript resolves");
    assert_eq!(typescript, registry::typescript());
    assert_eq!(typescript.server_id, "typescript_language_server");

    assert!(resolve_language("ruby").is_none());
    assert!(resolve_language("").is_none());
    assert!(resolve_language("Rust").is_none());
}

#[test]
fn fingerprints_are_opaque_and_compared_by_value() {
    let first = ConfigurationFingerprint::new("sha256:first").expect("valid fingerprint");
    let same = ConfigurationFingerprint::new("sha256:first").expect("valid fingerprint");
    let changed = ConfigurationFingerprint::new("sha256:second").expect("valid fingerprint");

    assert_eq!(first, same);
    assert_ne!(first, changed);
    assert!(ConfigurationFingerprint::new(" ").is_err());
}

#[test]
fn workspace_trust_revision_changes_when_trust_changes() {
    let trust = WorkspaceTrust::new("C:/code/project", true, 3).expect("valid trust");
    let revoked = trust.with_trusted(false).expect("revision can advance");

    assert!(trust.is_trusted());
    assert!(!revoked.is_trusted());
    assert_eq!(revoked.revision(), 4);
    assert!(WorkspaceTrust::new("", true, 1).is_err());
}

#[test]
fn process_states_distinguish_warming_ready_and_terminal_failure() {
    assert!(!ProcessState::Absent.is_warming());
    assert!(ProcessState::Starting.is_warming());
    assert!(ProcessState::Initializing.is_warming());
    assert!(ProcessState::Ready.is_ready());
    assert!(!ProcessState::Stopping.is_ready());
    assert!(!ProcessState::Backoff.is_ready());
    assert!(ProcessState::Failed.is_terminal());
}

#[test]
fn negotiated_capabilities_report_supported_queries() {
    let supported_encodings = [PositionEncoding::Utf8, PositionEncoding::Utf16];
    let supported_sync_modes = [
        DocumentSyncMode::None,
        DocumentSyncMode::Full,
        DocumentSyncMode::Incremental,
    ];
    let capabilities = NegotiatedCapabilities {
        position_encoding: PositionEncoding::Utf16,
        document_sync: DocumentSyncMode::Incremental,
        methods: SemanticMethod::ALL
            .iter()
            .map(|method| NegotiatedMethod {
                method: *method,
                supported: *method != SemanticMethod::References,
            })
            .collect(),
    };

    assert!(capabilities.supports(SemanticMethod::Definition));
    assert!(!capabilities.supports(SemanticMethod::References));
    assert!(capabilities.supports(SemanticMethod::Hover));
    assert!(capabilities.supports(SemanticMethod::Diagnostics));
    assert_eq!(supported_encodings.len(), 2);
    assert_eq!(supported_sync_modes.len(), 3);
}

#[test]
fn all_lists_every_semantic_method() {
    // The compiler cannot check that `ALL` is complete, so this is where a new variant is caught:
    // the exhaustive match below stops compiling, and the arm you add to fix it is the reminder to
    // add the variant to `ALL` too. A variant missing from `ALL` is negotiated for no server and
    // offered to nobody, which looks like a server problem rather than a build one.
    for method in SemanticMethod::ALL {
        match method {
            SemanticMethod::Definition
            | SemanticMethod::References
            | SemanticMethod::Hover
            | SemanticMethod::Diagnostics
            | SemanticMethod::TypeDefinition
            | SemanticMethod::Implementation
            | SemanticMethod::WorkspaceSymbols
            | SemanticMethod::DocumentSymbols
            | SemanticMethod::CallHierarchy => (),
        }
    }

    let mut unique = SemanticMethod::ALL.to_vec();
    unique.sort_unstable();
    unique.dedup();
    assert_eq!(
        unique.len(),
        SemanticMethod::ALL.len(),
        "ALL lists a method twice"
    );

    let mut ids = SemanticMethod::ALL
        .iter()
        .map(|method| method.id())
        .collect::<Vec<_>>();
    let listed = ids.len();
    ids.sort_unstable();
    ids.dedup();
    assert_eq!(ids.len(), listed, "two methods share a wire identifier");
}

#[test]
fn an_unadvertised_method_is_reported_rather_than_omitted() {
    // "The server does not offer this" and "this client does not implement it" must stay
    // distinguishable: only the first is something a user can fix by changing servers.
    let capabilities = NegotiatedCapabilities {
        position_encoding: PositionEncoding::Utf16,
        document_sync: DocumentSyncMode::Incremental,
        methods: SemanticMethod::ALL
            .iter()
            .map(|method| NegotiatedMethod {
                method: *method,
                supported: false,
            })
            .collect(),
    };

    assert_eq!(capabilities.methods.len(), SemanticMethod::ALL.len());
    assert!(capabilities.methods.iter().all(|entry| !entry.supported));
    assert!(SemanticMethod::ALL
        .iter()
        .all(|method| !capabilities.supports(*method)));
}

#[test]
fn document_versions_advance_without_wrapping() {
    let version = DocumentVersion::initial();
    assert_eq!(version.value(), 1);
    assert_eq!(version.next().expect("advance version").value(), 2);
    assert!(DocumentVersion::new(u64::MAX).next().is_err());
}

#[test]
fn diagnostic_snapshots_track_current_and_stale_document_versions() {
    let current = DocumentVersion::new(7);
    let range = NormalizedRange::new(1, 1, 1, 2).expect("valid range");
    let diagnostics = [
        DiagnosticSeverity::Error,
        DiagnosticSeverity::Warning,
        DiagnosticSeverity::Information,
        DiagnosticSeverity::Hint,
    ]
    .into_iter()
    .map(|severity| NormalizedDiagnostic {
        range,
        severity: Some(severity),
        message: "bounded message".into(),
        source: None,
        code: None,
        related_information: Vec::new(),
    })
    .collect();
    let snapshot = DiagnosticSnapshot::new(Some(current), current, diagnostics, 1_000);

    assert!(snapshot.is_current_for(current));
    assert!(!snapshot.is_current_for(DocumentVersion::new(8)));
    assert_eq!(snapshot.diagnostics().len(), 4);
}

#[test]
fn normalized_locations_require_one_based_ordered_ranges() {
    let range = NormalizedRange::new(2, 3, 2, 8).expect("valid range");
    let location = NormalizedLocation::new("src/main.rs", range, Some("fn main() {}".into()))
        .expect("valid location");

    assert_eq!(location.file(), "src/main.rs");
    assert!(NormalizedRange::new(0, 1, 1, 1).is_err());
    assert!(NormalizedRange::new(3, 1, 2, 1).is_err());
    assert!(NormalizedLocation::new("", range, None).is_err());
}

#[test]
fn fail_soft_outcomes_keep_empty_ready_distinct_from_degradation() {
    let ready: QueryOutcome<Vec<NormalizedLocation>> = QueryOutcome::ready(Vec::new(), 4);
    let warming: QueryOutcome<Vec<NormalizedLocation>> =
        QueryOutcome::degraded(QueryStatus::Warming, "server_starting")
            .expect("valid degraded outcome");
    for status in [
        QueryStatus::Timeout,
        QueryStatus::Unavailable,
        QueryStatus::Failed,
    ] {
        assert!(QueryOutcome::<Vec<NormalizedLocation>>::degraded(status, "bounded").is_ok());
    }

    assert_eq!(ready.status(), QueryStatus::Ready);
    assert_eq!(ready.document_version(), Some(DocumentVersion::new(4)));
    assert_eq!(ready.value().expect("ready value").len(), 0);
    assert_eq!(warming.status(), QueryStatus::Warming);
    assert!(warming.value().is_none());
    assert_eq!(warming.reason_code(), Some("server_starting"));
}
