use super::*;

const INSTALLATION_KEY_A: &[u8; 32] = b"evidence-test-installation-key-a";
const INSTALLATION_KEY_B: &[u8; 32] = b"evidence-test-installation-key-b";

fn sanitizer() -> EvidenceSanitizer {
    EvidenceSanitizer::new(INSTALLATION_KEY_A).expect("test installation key")
}

#[test]
fn privacy_corpus_covers_all_twelve_registered_classes() {
    let cases = [
        (
            RedactionClass::PrivateKey,
            "-----BEGIN PRIVATE KEY-----\nabc123\n-----END PRIVATE KEY-----",
            "abc123",
        ),
        (
            RedactionClass::Token,
            "access_token=tok_live_1234567890",
            "tok_live_1234567890",
        ),
        (
            RedactionClass::Authorization,
            "Authorization: Bearer header.payload.signature",
            "header.payload.signature",
        ),
        (
            RedactionClass::CredentialAssignment,
            "password=correct-horse-battery-staple",
            "correct-horse-battery-staple",
        ),
        (
            RedactionClass::CredentialUrl,
            "https://alice:secret-pass@example.com/repo",
            "alice:secret-pass",
        ),
        (
            RedactionClass::ConnectionString,
            "mongodb://database.internal:27017/app",
            "mongodb://database.internal:27017/app",
        ),
        (
            RedactionClass::SecretEnvironment,
            "AWS_SECRET_ACCESS_KEY=ABCDEFGHIJKLMNOPQRSTUVWX",
            "ABCDEFGHIJKLMNOPQRSTUVWX",
        ),
        (
            RedactionClass::UserHomePath,
            r"C:\Users\alice\private\notes.txt",
            "alice",
        ),
        (
            RedactionClass::Email,
            "owner=alice@example.com",
            "alice@example.com",
        ),
        (
            RedactionClass::Phone,
            "call +86 138-0013-8000",
            "138-0013-8000",
        ),
        (
            RedactionClass::NetworkIdentifier,
            "host=10.24.8.7",
            "10.24.8.7",
        ),
        (
            RedactionClass::CloudAccount,
            "tenant_id=72f988bf-86f1-41af-91ab-2d7cd011db47",
            "72f988bf-86f1-41af-91ab-2d7cd011db47",
        ),
    ];

    for (class, input, prohibited_fragment) in cases {
        let result = sanitizer().sanitize(input).expect("sanitize corpus entry");
        assert_eq!(result.count(class), 1, "missing class {class:?}");
        assert!(!result.text().contains(prohibited_fragment));
        assert!(result.text().contains(&class.marker_prefix()));
        assert_eq!(result.sanitizer_version(), EVIDENCE_SANITIZER_V1);
    }
}

#[test]
fn markers_are_stable_within_one_installation_and_differ_between_installations() {
    let input = "email alice@example.com";
    let first = sanitizer().sanitize(input).expect("first result");
    let repeated = sanitizer().sanitize(input).expect("repeated result");
    let other = EvidenceSanitizer::new(INSTALLATION_KEY_B)
        .expect("second installation key")
        .sanitize(input)
        .expect("other result");

    assert_eq!(first.text(), repeated.text());
    assert_ne!(first.text(), other.text());
    assert!(!first.text().contains("alice@example.com"));
}

#[test]
fn repeated_values_share_a_marker_without_leaking_the_value() {
    let result = sanitizer()
        .sanitize("alice@example.com then alice@example.com")
        .expect("sanitize repeated value");
    let markers = result
        .text()
        .split_whitespace()
        .filter(|part| part.starts_with("<redacted:email:"))
        .collect::<Vec<_>>();

    assert_eq!(result.count(RedactionClass::Email), 2);
    assert_eq!(markers.len(), 2);
    assert_eq!(markers[0], markers[1]);
    assert!(!result.text().contains("alice@example.com"));
}

#[test]
fn preserves_unicode_and_already_redacted_markers() {
    let existing = "<redacted:email:7F3A>";
    let result = sanitizer()
        .sanitize(&format!("修复建议：{existing}，联系 bob@example.com。"))
        .expect("sanitize unicode");

    assert!(result.text().starts_with("修复建议："));
    assert!(result.text().contains(existing));
    assert_eq!(result.count(RedactionClass::Email), 1);
}

#[test]
fn ordered_rules_remove_overlapping_credentials_before_derivation() {
    let input = "Authorization: Bearer token@example.com from 10.0.0.4";
    let result = sanitizer().sanitize(input).expect("sanitize overlap");

    assert_eq!(result.count(RedactionClass::Authorization), 1);
    assert_eq!(result.count(RedactionClass::Email), 0);
    assert_eq!(result.count(RedactionClass::NetworkIdentifier), 1);
    assert!(!result.text().contains("token@example.com"));

    let fingerprint_input = result.text().as_bytes();
    assert!(!fingerprint_input
        .windows("token@example.com".len())
        .any(|window| window == b"token@example.com"));
}

#[test]
fn source_code_like_identifiers_are_not_treated_as_secrets() {
    let source = "let password = config.password; fn access_token() -> Token { todo!() }";
    let result = sanitizer().sanitize(source).expect("sanitize source");

    assert_eq!(result.text(), source);
    assert_eq!(result.total_redactions(), 0);
}

#[test]
fn rejects_unbounded_text_and_weak_installation_keys() {
    assert!(matches!(
        EvidenceSanitizer::new(b"short"),
        Err(SanitizationError::WeakInstallationKey)
    ));
    assert_eq!(
        sanitizer().sanitize(&"界".repeat(MAX_SANITIZER_INPUT_CHARS + 1)),
        Err(SanitizationError::InputTooLong {
            max: MAX_SANITIZER_INPUT_CHARS,
        })
    );
}

#[test]
fn envelope_projects_only_sanitized_registered_text() {
    let feedback: EvidenceSourceEnvelope = serde_json::from_value(serde_json::json!({
        "sourceKind": "explicit_feedback",
        "schemaVersion": 1,
        "common": {
            "sourceEventId": "feedback:1",
            "occurredAt": "2026-08-13T01:00:00Z",
            "stableAgentId": "onepiece",
            "sessionId": "session-1",
            "messageId": "message-1",
            "runId": "run-1",
            "attemptId": null,
            "workspace": "workspace:7f3a",
            "fidelity": "native",
            "observedSkillRevisions": []
        },
        "feedback": "corrected",
        "feedbackRevision": 1,
        "correctionNote": "Contact alice@example.com instead."
    }))
    .expect("feedback envelope");
    let native: EvidenceSourceEnvelope = serde_json::from_value(serde_json::json!({
        "sourceKind": "native_execution",
        "schemaVersion": 1,
        "common": {
            "sourceEventId": "native:1",
            "occurredAt": "2026-08-13T01:00:00Z",
            "stableAgentId": "onepiece",
            "sessionId": "session-1",
            "messageId": "message-1",
            "runId": "run-1",
            "attemptId": null,
            "workspace": "workspace:7f3a",
            "fidelity": "native",
            "observedSkillRevisions": []
        },
        "operationClass": "generation",
        "outcome": "succeeded",
        "failureClass": null,
        "safeCounts": { "attempts": 1, "failures": 0 }
    }))
    .expect("native envelope");

    let projected = feedback
        .sanitized_registered_text(&sanitizer())
        .expect("sanitize feedback")
        .expect("correction projection");
    assert!(!projected.text().contains("alice@example.com"));
    assert_eq!(projected.count(RedactionClass::Email), 1);
    assert_eq!(
        native
            .sanitized_registered_text(&sanitizer())
            .expect("metadata-only projection"),
        None
    );
}
