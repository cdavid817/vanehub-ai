use super::memory_document::{
    compose_memory_document, parse_memory_document, validate_name, MemoryDocument, MemoryMetadata,
    MemoryType,
};
use super::AgentRuntimeDomainError;

fn document(frontmatter: &str, body: &str) -> String {
    format!("---\n{frontmatter}\n---\n\n{body}\n")
}

#[test]
fn recognized_types_round_trip_and_unknown_ones_degrade() {
    for (raw, expected) in [
        ("user", Some(MemoryType::User)),
        ("feedback", Some(MemoryType::Feedback)),
        ("project", Some(MemoryType::Project)),
        ("reference", Some(MemoryType::Reference)),
        // Degrading rather than rejecting is what keeps migrated files (written untyped on
        // purpose) and files from a branch on a different contract readable.
        ("", None),
        ("unknown", None),
        ("User", None),
    ] {
        assert_eq!(MemoryType::parse(raw), expected, "parsing {raw:?}");
    }
    assert_eq!(MemoryType::Feedback.as_str(), "feedback");
}

#[test]
fn a_file_without_a_type_parses_as_untyped() {
    let parsed = parse_memory_document(&document(
        "name: user-role\ndescription: The user is a data scientist",
        "Prefers backend analogues.",
    ))
    .expect("untyped memory parses");

    assert_eq!(parsed.metadata.name, "user-role");
    assert_eq!(parsed.metadata.memory_type, None);
    assert_eq!(parsed.body, "Prefers backend analogues.");
}

#[test]
fn an_unrecognized_type_parses_as_untyped_rather_than_failing() {
    let parsed = parse_memory_document(&document(
        "name: user-role\ndescription: A description\ntype: preference",
        "Body.",
    ))
    .expect("unrecognized type degrades");

    assert_eq!(parsed.metadata.memory_type, None);
}

#[test]
fn unknown_keys_are_ignored_rather_than_rejected() {
    // The memory directory is shared across worktrees, so a branch on a newer contract will write
    // keys this build has never seen. Rejecting them would make the two branches mutually
    // destructive rather than merely redundant.
    let parsed = parse_memory_document(&document(
        "name: shared\ndescription: A description\nscope: team\nconfidence: 0.9",
        "Body.",
    ))
    .expect("unknown keys are ignored");

    assert_eq!(parsed.metadata.name, "shared");
}

#[test]
fn crlf_input_parses_identically_to_lf() {
    let lf = document("name: crlf\ndescription: A description", "Body line.");
    let parsed_lf = parse_memory_document(&lf).expect("lf");
    let parsed_crlf = parse_memory_document(&lf.replace('\n', "\r\n")).expect("crlf");

    assert_eq!(parsed_lf, parsed_crlf);
}

#[test]
fn a_file_without_frontmatter_is_a_typed_error() {
    assert_eq!(
        parse_memory_document("Just a body with no frontmatter.\n"),
        Err(AgentRuntimeDomainError::MemoryFrontmatterMissing)
    );
    assert_eq!(
        parse_memory_document("---\nname: unterminated\ndescription: d\n"),
        Err(AgentRuntimeDomainError::MemoryFrontmatterMissing)
    );
}

#[test]
fn name_and_description_are_required() {
    assert!(matches!(
        parse_memory_document(&document("description: A description", "Body.")),
        Err(AgentRuntimeDomainError::InvalidMemoryValue("name"))
    ));
    assert!(matches!(
        parse_memory_document(&document("name: no-description", "Body.")),
        Err(AgentRuntimeDomainError::InvalidMemoryValue("description"))
    ));
}

#[test]
fn an_empty_body_is_rejected() {
    assert!(matches!(
        parse_memory_document(&document("name: empty\ndescription: A description", "   ")),
        Err(AgentRuntimeDomainError::InvalidMemoryValue("body"))
    ));
}

#[test]
fn only_the_first_colon_separates_key_from_value() {
    // A Windows workspace path and a description containing a colon both reach here, and both
    // would be truncated by a naive split-on-every-colon.
    let parsed = parse_memory_document(&document(
        "name: colons\ndescription: Rule: never mock the database\nfolder: D:/code/vanehub-ai",
        "Body.",
    ))
    .expect("colons survive");

    assert_eq!(parsed.metadata.description, "Rule: never mock the database");
    assert_eq!(
        parsed.metadata.folder.as_deref(),
        Some("D:/code/vanehub-ai")
    );
}

#[test]
fn names_that_cannot_be_file_stems_are_rejected() {
    for rejected in [
        "",
        "   ",
        "with/slash",
        "with\\backslash",
        "..",
        "../escape",
        "trailing.",
        ".leading",
        "with:colon",
        "with\nnewline",
        // Windows treats these as devices regardless of extension, so `con.md` cannot be created.
        "con",
        "COM1",
        "nul",
    ] {
        assert!(
            validate_name(rejected).is_err(),
            "expected {rejected:?} to be rejected"
        );
    }

    assert_eq!(
        validate_name("  route-b-memory-design  ").expect("trimmed"),
        "route-b-memory-design"
    );
    assert!(validate_name(&"a".repeat(101)).is_err());
    assert!(validate_name(&"a".repeat(100)).is_ok());
}

#[test]
fn multiline_descriptions_are_rejected() {
    // A newline here would split one memory into two rows in both the index and the manifest.
    assert!(matches!(
        MemoryMetadata::new("name", "first line\nsecond line", None),
        Err(AgentRuntimeDomainError::InvalidMemoryValue(
            "description characters"
        ))
    ));
    assert!(MemoryMetadata::new("name", "a".repeat(301), None).is_err());
}

#[test]
fn compose_round_trips_through_parse() {
    let metadata = MemoryMetadata::new(
        "shared-toolchain",
        "Concurrent rustup updates corrupt std mid-build",
        Some(MemoryType::Project),
    )
    .expect("metadata")
    .with_provenance(
        Some("onepiece".to_string()),
        Some("D:/code/vanehub-ai".to_string()),
        Some("automatic".to_string()),
        Some("2026-08-15T09:12:44Z".to_string()),
    );
    let original = MemoryDocument::new(metadata, "Diagnose as contention, not as a broken build.")
        .expect("document");

    let parsed = parse_memory_document(&compose_memory_document(&original)).expect("round trip");

    assert_eq!(parsed, original);
}

#[test]
fn compose_omits_absent_optional_fields() {
    let document = MemoryDocument::new(
        MemoryMetadata::new("minimal", "A description", None).expect("metadata"),
        "Body.",
    )
    .expect("document");

    let composed = compose_memory_document(&document);

    assert!(!composed.contains("type:"));
    assert!(!composed.contains("agent:"));
    assert!(!composed.contains("migrated_from:"));
    assert_eq!(
        parse_memory_document(&composed).expect("round trip"),
        document
    );
}

#[test]
fn migrated_from_survives_a_round_trip() {
    // Migration idempotence rests entirely on this field surviving.
    let document = MemoryDocument::new(
        MemoryMetadata::new("migrated", "A description", None)
            .expect("metadata")
            .with_migrated_from("row-42"),
        "Body.",
    )
    .expect("document");

    let parsed = parse_memory_document(&compose_memory_document(&document)).expect("round trip");

    assert_eq!(parsed.metadata.migrated_from.as_deref(), Some("row-42"));
}

#[test]
fn file_name_is_the_name_plus_the_markdown_extension() {
    let metadata = MemoryMetadata::new("user-role", "A description", None).expect("metadata");
    assert_eq!(metadata.file_name(), "user-role.md");
}
