//! The frozen v1 reader.
//!
//! These cases are the contract migration depends on, so they are written against literal file
//! bytes rather than against a composer: v1's writer is being removed, and a test that round-trips
//! through it would keep passing after the format it describes no longer exists anywhere.

use super::legacy_memory_document::parse_legacy_document;

const CANONICAL: &str = "---\nname: user-role\ndescription: The user is a data scientist\ntype: user\nagent: onepiece\nfolder: D:/code/vanehub-ai\nsource: explicit\ncreated: 2026-08-01T10:00:00.000Z\n---\n\nPrefers concise answers.\n";

#[test]
fn a_canonical_v1_file_yields_every_field() {
    let document = parse_legacy_document(CANONICAL).expect("parses");

    assert_eq!(document.name, "user-role");
    assert_eq!(document.description, "The user is a data scientist");
    assert_eq!(document.memory_type.as_deref(), Some("user"));
    assert_eq!(document.agent_id.as_deref(), Some("onepiece"));
    assert_eq!(document.folder.as_deref(), Some("D:/code/vanehub-ai"));
    assert_eq!(document.save_source.as_deref(), Some("explicit"));
    assert_eq!(
        document.created_at.as_deref(),
        Some("2026-08-01T10:00:00.000Z")
    );
    assert_eq!(document.body, "Prefers concise answers.");
}

#[test]
fn the_save_source_is_read_verbatim_and_absent_when_the_file_omits_it() {
    // Read as a string rather than resolved here, so the one place that knows the v2 taxonomy is
    // also the one place that decides an unrecognized value means "unknown".
    for (raw, expected) in [
        ("source: explicit\n", Some("explicit")),
        ("source: automatic\n", Some("automatic")),
        ("source: something-new\n", Some("something-new")),
        // An empty value is dropped by the frontmatter reader, exactly as v1 dropped it.
        ("source: \n", None),
        ("", None),
    ] {
        let file = format!("---\nname: n\ndescription: d\n{raw}---\n\nBody.\n");
        assert_eq!(
            parse_legacy_document(&file)
                .expect("parses")
                .save_source
                .as_deref(),
            expected,
            "for {raw:?}"
        );
    }
}

#[test]
fn crlf_parses_identically_to_lf() {
    // The memory directory is host-level and shared across worktrees, so the same file genuinely
    // arrives with either ending. A reader that disagreed with itself here would migrate the same
    // memory twice, once per checkout.
    let crlf = CANONICAL.replace('\n', "\r\n");

    assert_eq!(
        parse_legacy_document(&crlf).expect("crlf parses"),
        parse_legacy_document(CANONICAL).expect("lf parses")
    );
}

#[test]
fn a_value_keeps_its_colons_and_loses_its_quotes() {
    let raw =
        "---\nname: \"quoted\"\ndescription: Ratio is 3:1, see http://example.test\n---\n\nBody.\n";

    let document = parse_legacy_document(raw).expect("parses");

    assert_eq!(document.name, "quoted");
    assert_eq!(
        document.description,
        "Ratio is 3:1, see http://example.test"
    );
}

#[test]
fn unknown_keys_are_ignored_rather_than_rejected() {
    // A branch running a different contract writes keys this build has never heard of into the same
    // shared directory. Refusing them would strand those files instead of migrating them.
    let raw = "---\nname: forward\ndescription: d\nsome_future_key: whatever\n---\n\nBody.\n";

    let document = parse_legacy_document(raw).expect("parses");

    assert_eq!(document.name, "forward");
}

#[test]
fn a_missing_description_still_yields_the_body() {
    // v1 refused this file, which made it invisible rather than absent: the text stayed on disk and
    // no surface showed it. v2 permits an empty description, so recovering the body wins.
    let raw = "---\nname: no-description\n---\n\nThe text that would otherwise be lost.\n";

    let document = parse_legacy_document(raw).expect("parses");

    assert_eq!(document.description, "");
    assert_eq!(document.body, "The text that would otherwise be lost.");
}

#[test]
fn a_name_longer_than_the_v1_limit_still_parses() {
    // v1 capped names at 100 characters because the name was the file stem. v2 addresses files by
    // id and allows 120, so a file v1 refused is migratable here.
    let long_name = "n".repeat(110);
    let raw = format!("---\nname: {long_name}\ndescription: d\n---\n\nBody.\n");

    assert_eq!(
        parse_legacy_document(&raw).expect("parses").name,
        long_name.as_str()
    );
}

#[test]
fn a_file_without_readable_frontmatter_is_refused() {
    for refused in [
        // No frontmatter at all.
        "Just a body.\n",
        // Opens with something other than the delimiter.
        "\n---\nname: x\ndescription: d\n---\n\nBody.\n",
        // Never terminated.
        "---\nname: x\ndescription: d\n",
    ] {
        assert!(
            parse_legacy_document(refused).is_none(),
            "expected {refused:?} to be refused"
        );
    }
}

#[test]
fn a_file_without_a_name_or_without_a_body_is_refused() {
    // Both are quarantined rather than migrated with invented values: a memory with no name cannot
    // be addressed, and a memory with no body has nothing to say.
    for refused in [
        "---\ndescription: d\n---\n\nBody.\n",
        "---\nname: \ndescription: d\n---\n\nBody.\n",
        "---\nname: empty-body\ndescription: d\n---\n\n",
        "---\nname: blank-body\ndescription: d\n---\n\n   \n\t\n",
    ] {
        assert!(
            parse_legacy_document(refused).is_none(),
            "expected {refused:?} to be refused"
        );
    }
}

#[test]
fn a_v2_file_also_parses_here_which_is_why_exclusion_is_by_declared_schema_version() {
    // This is the trap the enumeration rule exists for. A v2 file carries `name` and `description`
    // too, so a reader cannot be what separates the formats — only the declared `schema_version`
    // can. If this ever stops parsing, the enumeration gate is still the correct one and this test
    // is what says so.
    let v2 = "---\nschema_version: 2\nid: 01K2MEM0000000000000000001\nname: \"looks-legacy\"\ndescription: \"d\"\nmemory_type: project\nscope_kind: global\naudience: all_agents\nstatus: active\nsource: explicit_user\nsensitivity: normal\nrevision: 1\ncreated_at: 2026-08-01T10:00:00.000Z\nupdated_at: 2026-08-01T10:00:00.000Z\nuse_count: 0\ncontent_hash: sha256:deadbeef\n---\n\nBody.\n";

    assert_eq!(
        parse_legacy_document(v2)
            .expect("a v2 file is readable as v1 shaped text")
            .name,
        "looks-legacy"
    );
}

#[test]
fn a_body_containing_a_delimiter_survives() {
    // The first closing delimiter is the header's, because every header value is a single line. A
    // `---` further down belongs to the body and must not truncate it.
    let raw = "---\nname: with-rule\ndescription: d\n---\n\nBefore.\n\n---\n\nAfter.\n";

    assert_eq!(
        parse_legacy_document(raw).expect("parses").body,
        "Before.\n\n---\n\nAfter."
    );
}
