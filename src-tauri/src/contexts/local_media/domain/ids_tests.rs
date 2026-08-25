use super::*;

const HEX32: &str = "0123456789abcdef0123456789abcdef";

#[test]
fn a_well_formed_id_parses_and_round_trips() {
    let raw = format!("{}{HEX32}", StagedInputId::PREFIX);
    let parsed = StagedInputId::parse(&raw).expect("well-formed id");
    assert_eq!(parsed.as_str(), raw);
    assert_eq!(parsed.to_string(), raw);
}

#[test]
fn each_id_type_rejects_another_types_prefix() {
    let recording = format!("{}{HEX32}", RecordingId::PREFIX);
    assert!(RecordingId::parse(&recording).is_some());
    assert!(StagedInputId::parse(&recording).is_none());
    assert!(PlaybackId::parse(&recording).is_none());
    assert!(LocalMediaOperationId::parse(&recording).is_none());
}

#[test]
fn prefixes_are_distinct() {
    let prefixes = [
        StagedInputId::PREFIX,
        RecordingId::PREFIX,
        LocalMediaOperationId::PREFIX,
        PlaybackId::PREFIX,
    ];
    let unique: std::collections::BTreeSet<&str> = prefixes.iter().copied().collect();
    assert_eq!(unique.len(), prefixes.len());
}

#[test]
fn path_traversal_cannot_be_expressed_as_an_id() {
    for hostile in [
        "lmi-../../../etc/passwd",
        "lmi-..",
        "lmi-/absolute",
        "lmi-a/b",
        "lmi-a\\b",
        "../lmi-0123456789abcdef0123456789abcdef",
    ] {
        assert!(
            StagedInputId::parse(hostile).is_none(),
            "{hostile} must not parse"
        );
    }
}

#[test]
fn the_suffix_must_be_exactly_thirty_two_hex_digits() {
    assert!(StagedInputId::parse("lmi-").is_none());
    assert!(StagedInputId::parse("lmi-0123456789abcdef0123456789abcde").is_none());
    assert!(StagedInputId::parse("lmi-0123456789abcdef0123456789abcdeff").is_none());
    assert!(StagedInputId::parse("lmi-0123456789ABCDEF0123456789abcdef").is_some());
    assert!(StagedInputId::parse("lmi-0123456789abcdef0123456789abcdeg").is_none());
}

#[test]
fn ids_serialize_as_bare_strings() {
    let raw = format!("{}{HEX32}", PlaybackId::PREFIX);
    let id = PlaybackId::new(raw.clone());
    assert_eq!(
        serde_json::to_value(&id).expect("serialize"),
        serde_json::json!(raw)
    );
    let restored: PlaybackId = serde_json::from_value(serde_json::json!(raw)).expect("deserialize");
    assert_eq!(restored, id);
}

#[test]
fn composer_scope_ids_are_bounded_and_printable() {
    assert!(ComposerScopeId::parse("session-42").is_some());
    assert!(ComposerScopeId::parse("session_42").is_some());
    assert!(ComposerScopeId::parse("").is_none());
    assert!(ComposerScopeId::parse("session 42").is_none());
    assert!(ComposerScopeId::parse("session/42").is_none());
    assert!(ComposerScopeId::parse("session\n42").is_none());
    assert!(ComposerScopeId::parse(&"a".repeat(128)).is_some());
    assert!(ComposerScopeId::parse(&"a".repeat(129)).is_none());
}

#[test]
fn composer_scopes_compare_by_value() {
    let left = ComposerScopeId::new("session-1");
    let right = ComposerScopeId::new("session-1");
    let other = ComposerScopeId::new("session-2");
    assert_eq!(left, right);
    assert_ne!(left, other);
}
