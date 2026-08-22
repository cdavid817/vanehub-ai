//! Network origin rules.
//!
//! Table-driven, one row per rule. The interesting cases are the ones that look like an origin to
//! a reader and are not one to a matcher — a path, a wildcard, a credential — because those are
//! where the text a user approves and the rule the broker enforces come apart.

use super::{NetworkOrigin, OriginRejection, MAX_ORIGIN_CHARACTERS};

fn rejection(value: &str) -> OriginRejection {
    NetworkOrigin::parse(value)
        .expect_err("origin should be rejected")
        .reason
}

fn canonical(value: &str) -> String {
    NetworkOrigin::parse(value)
        .unwrap_or_else(|error| panic!("{value} should parse: {error}"))
        .as_str()
        .to_string()
}

#[test]
fn a_plain_https_origin_is_accepted_unchanged() {
    for value in [
        "https://api.github.com",
        "https://example.com:8443",
        "https://sub.domain.example.com",
        "https://127.0.0.1:9000",
    ] {
        assert_eq!(canonical(value), value);
    }
}

#[test]
fn spellings_of_the_same_origin_canonicalize_together() {
    // Case and a default port are not distinctions in a URL, so two entries that differ only in
    // those would let a package appear to request less than it does.
    for value in [
        "HTTPS://API.GitHub.com",
        "https://api.github.com:443",
        "https://api.github.com/",
    ] {
        assert_eq!(canonical(value), "https://api.github.com", "for {value}");
    }
}

#[test]
fn a_wildcard_is_refused_before_the_parser_can_interpret_it() {
    // A request for every subdomain: not something a reviewer can evaluate, not something the
    // broker can match. Checked on the text so `*` never reaches the host parser.
    for value in [
        "https://*.github.com",
        "https://*",
        "https://api.*.com",
        "*://api.github.com",
    ] {
        assert_eq!(rejection(value), OriginRejection::Wildcard, "for {value}");
    }
}

#[test]
fn anything_beyond_scheme_host_and_port_is_refused() {
    // The trap this type exists for: a reviewer reads a path and approves what looks like one
    // endpoint, while the broker can only ever match the origin and has granted the whole host.
    assert_eq!(
        rejection("https://api.github.com/repos/acme"),
        OriginRejection::HasPath
    );
    assert_eq!(
        rejection("https://api.github.com/?a=b"),
        OriginRejection::HasQuery
    );
    assert_eq!(
        rejection("https://api.github.com/#section"),
        OriginRejection::HasFragment
    );
}

#[test]
fn a_credential_written_into_a_manifest_is_refused() {
    assert_eq!(
        rejection("https://user:token@api.github.com"),
        OriginRejection::Userinfo
    );
    assert_eq!(
        rejection("https://user@api.github.com"),
        OriginRejection::Userinfo
    );
}

#[test]
fn plaintext_http_is_refused_for_remote_hosts_and_allowed_for_loopback() {
    // Plaintext to a remote host hands every request and any bearer token to the network. A local
    // service often has no other option.
    assert_eq!(
        rejection("http://api.github.com"),
        OriginRejection::InsecureRemoteScheme
    );
    assert_eq!(
        rejection("http://192.168.1.10:8080"),
        OriginRejection::InsecureRemoteScheme
    );

    assert_eq!(canonical("http://localhost:3000"), "http://localhost:3000");
    assert_eq!(canonical("http://127.0.0.1:3000"), "http://127.0.0.1:3000");
    assert_eq!(canonical("http://[::1]:3000"), "http://[::1]:3000");
    // Case-insensitive, like every other host comparison.
    assert_eq!(canonical("http://LOCALHOST:3000"), "http://localhost:3000");
}

#[test]
fn only_http_and_https_are_supported_schemes() {
    for value in [
        "file:///etc/passwd",
        "ftp://example.com",
        "ws://localhost:3000",
        "custom://thing",
    ] {
        assert_eq!(
            rejection(value),
            OriginRejection::UnsupportedScheme,
            "for {value}"
        );
    }
}

#[test]
fn a_scheme_relative_or_bare_host_is_not_an_origin() {
    // An origin without a scheme is a guess about which scheme, and the guess would be http.
    for value in ["api.github.com", "//api.github.com", "/api"] {
        assert_eq!(
            rejection(value),
            OriginRejection::Unparseable,
            "for {value}"
        );
    }
}

#[test]
fn empty_and_oversized_origins_are_bounded() {
    assert_eq!(rejection(""), OriginRejection::Empty);

    let long_host = "a".repeat(MAX_ORIGIN_CHARACTERS);
    assert_eq!(
        rejection(&format!("https://{long_host}")),
        OriginRejection::TooLong
    );
}

#[test]
fn a_rejection_carries_a_bounded_origin_and_a_specific_reason_code() {
    let hostile = format!("https://{}/path", "a".repeat(1_000));
    let error = NetworkOrigin::parse(&hostile).expect_err("rejected");

    assert_eq!(error.code(), "invalid_network_origin");
    assert_eq!(error.reason_code(), "too_long");
    assert_eq!(error.origin.chars().count(), MAX_ORIGIN_CHARACTERS);
}

#[test]
fn every_rejection_reason_has_its_own_code() {
    let reasons = [
        OriginRejection::Empty,
        OriginRejection::TooLong,
        OriginRejection::Wildcard,
        OriginRejection::Unparseable,
        OriginRejection::UnsupportedScheme,
        OriginRejection::InsecureRemoteScheme,
        OriginRejection::Userinfo,
        OriginRejection::MissingHost,
        OriginRejection::HasPath,
        OriginRejection::HasQuery,
        OriginRejection::HasFragment,
    ];

    let mut codes: Vec<&str> = reasons.iter().map(|reason| reason.as_str()).collect();
    let total = codes.len();
    codes.sort_unstable();
    codes.dedup();
    assert_eq!(codes.len(), total, "each reason needs a distinct code");
}
