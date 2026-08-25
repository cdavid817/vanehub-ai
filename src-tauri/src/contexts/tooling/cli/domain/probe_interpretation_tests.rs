// Included through `#[path]` from probe_interpretation.rs.
//
// Fixture-driven. No provider is contacted, no login is performed, no credential store is read --
// these are string fixtures resembling what each documented command prints.
use super::*;

fn reading<'a>(exit_code: Option<i32>, stdout: &'a str, stderr: &'a str) -> CliProbeReading<'a> {
    CliProbeReading {
        exit_code,
        timed_out: false,
        cancelled: false,
        stdout,
        stderr,
    }
}

fn timed_out<'a>() -> CliProbeReading<'a> {
    CliProbeReading {
        exit_code: None,
        timed_out: true,
        cancelled: false,
        stdout: "",
        stderr: "",
    }
}

fn cancelled<'a>() -> CliProbeReading<'a> {
    CliProbeReading {
        exit_code: None,
        timed_out: false,
        cancelled: true,
        stdout: "",
        stderr: "",
    }
}

#[test]
fn an_undocumented_probe_is_always_unknown_whatever_it_printed() {
    // Gemini CLI and Antigravity CLI have no verified non-interactive probe. Even output that
    // looks conclusive must not be read, because nothing established what it means.
    let convincing = reading(Some(0), "logged in as someone@example.test", "");

    assert_eq!(
        interpret_authentication(CliAuthParser::Undocumented, convincing),
        CliAuthenticationStatus::Unknown
    );
    assert_eq!(
        interpret_doctor(CliDoctorParser::Undocumented, convincing),
        CliDoctorVerdict::Unknown
    );
}

#[test]
fn codex_reports_authenticated_only_on_a_documented_success_line() {
    let healthy = reading(Some(0), "Logged in as dev@example.test\n", "");
    assert_eq!(
        interpret_authentication(CliAuthParser::CodexLoginStatus, healthy),
        CliAuthenticationStatus::Authenticated
    );
}

#[test]
fn codex_exit_zero_with_nothing_recognisable_is_unknown_not_authenticated() {
    // The rule that matters: silence is never consent.
    let quiet = reading(Some(0), "\n", "");
    assert_eq!(
        interpret_authentication(CliAuthParser::CodexLoginStatus, quiet),
        CliAuthenticationStatus::Unknown
    );
}

#[test]
fn codex_reports_authentication_required() {
    for text in [
        "Not logged in.",
        "No credentials found.",
        "Please log in to continue",
    ] {
        let outcome = reading(Some(1), text, "");
        assert_eq!(
            interpret_authentication(CliAuthParser::CodexLoginStatus, outcome),
            CliAuthenticationStatus::Required,
            "{text}"
        );
    }
}

#[test]
fn an_expired_session_is_distinguished_from_never_logged_in() {
    // Expiry usually reports both phrases; the expired one is the more actionable.
    let expired = reading(Some(1), "Session expired. Not logged in.", "");
    assert_eq!(
        interpret_authentication(CliAuthParser::CodexLoginStatus, expired),
        CliAuthenticationStatus::Expired
    );

    let reauth = reading(Some(1), "Token is invalid; please reauthenticate.", "");
    assert_eq!(
        interpret_authentication(CliAuthParser::CodexLoginStatus, reauth),
        CliAuthenticationStatus::Expired
    );
}

#[test]
fn an_unsupported_subcommand_is_unknown_not_a_login_prompt() {
    // An older build without `login status` exits non-zero. Telling the user to log in to a tool
    // that has no login command is worse than admitting we cannot tell.
    for text in [
        "error: unrecognized subcommand 'login'",
        "unknown command: login",
        "no such command: login",
        "error: invalid choice: 'status'",
    ] {
        let outcome = reading(Some(2), "", text);
        assert_eq!(
            interpret_authentication(CliAuthParser::CodexLoginStatus, outcome),
            CliAuthenticationStatus::Unknown,
            "{text}"
        );
    }
}

#[test]
fn opencode_reduces_its_credential_table_to_a_count() {
    let listed = reading(
        Some(0),
        "Provider   Account\n---------  -------\nanthropic  dev@example.test\nopenai     ops@example.test\n",
        "",
    );
    assert_eq!(
        interpret_authentication(CliAuthParser::OpenCodeAuthList, listed),
        CliAuthenticationStatus::Authenticated
    );
    assert_eq!(credential_entry_count(listed.stdout), 2);
}

#[test]
fn opencode_with_an_empty_table_reports_authentication_required() {
    for stdout in [
        "Provider   Account\n---------  -------\n",
        "No credentials stored.\n",
        "\n",
    ] {
        let outcome = reading(Some(0), stdout, "");
        assert_eq!(
            interpret_authentication(CliAuthParser::OpenCodeAuthList, outcome),
            CliAuthenticationStatus::Required,
            "{stdout:?}"
        );
    }
}

#[test]
fn opencode_failing_to_list_is_unknown_rather_than_logged_out() {
    // A failed listing could be an unsupported command, a broken config, or a permissions problem.
    // None of those establishes that the user is logged out.
    let failed = reading(Some(1), "", "could not read auth store");
    assert_eq!(
        interpret_authentication(CliAuthParser::OpenCodeAuthList, failed),
        CliAuthenticationStatus::Unknown
    );
}

#[test]
fn a_timeout_or_cancellation_never_concludes_anything() {
    for outcome in [timed_out(), cancelled()] {
        for parser in [
            CliAuthParser::CodexLoginStatus,
            CliAuthParser::OpenCodeAuthList,
        ] {
            assert_eq!(
                interpret_authentication(parser, outcome),
                CliAuthenticationStatus::Unknown
            );
        }
        assert_eq!(
            interpret_doctor(CliDoctorParser::ClaudeCodeDoctor, outcome),
            CliDoctorVerdict::Unknown
        );
    }
}

#[test]
fn malformed_output_falls_through_to_unknown_rather_than_a_guess() {
    let noise = reading(Some(0), "\u{1}\u{2}garbled binary spew", "");
    assert_eq!(
        interpret_authentication(CliAuthParser::CodexLoginStatus, noise),
        CliAuthenticationStatus::Unknown
    );
}

#[test]
fn claude_doctor_maps_exit_status_to_a_verdict() {
    assert_eq!(
        interpret_doctor(
            CliDoctorParser::ClaudeCodeDoctor,
            reading(Some(0), "All checks passed", "")
        ),
        CliDoctorVerdict::Ok
    );
    assert_eq!(
        interpret_doctor(
            CliDoctorParser::ClaudeCodeDoctor,
            reading(Some(1), "1 check failed", "")
        ),
        CliDoctorVerdict::Problem
    );
}

#[test]
fn an_older_build_without_doctor_is_unknown_not_broken() {
    let unsupported = reading(Some(2), "", "error: unrecognized subcommand 'doctor'");
    assert_eq!(
        interpret_doctor(CliDoctorParser::ClaudeCodeDoctor, unsupported),
        CliDoctorVerdict::Unknown
    );
    assert!(!CliDoctorVerdict::Unknown.reports_problem());
    assert!(CliDoctorVerdict::Problem.reports_problem());
}

#[test]
fn no_parser_returns_any_fragment_of_the_output_it_read() {
    // The whole surface is an enum. A token that survived redaction upstream still cannot escape
    // through a state value, because there is no state value that carries text.
    let secret = "Authorization: Bearer sk-ant-not-redacted-by-accident";
    let outcome = reading(Some(0), secret, secret);

    let auth = interpret_authentication(CliAuthParser::CodexLoginStatus, outcome);
    let doctor = interpret_doctor(CliDoctorParser::ClaudeCodeDoctor, outcome);

    assert!(!auth.as_str().contains("sk-ant"));
    assert!(!doctor.as_str().contains("sk-ant"));
    // Every possible value is one of a fixed, short list of codes.
    assert!([
        "authenticated",
        "required",
        "expired",
        "unknown",
        "not-applicable"
    ]
    .contains(&auth.as_str()));
    assert!(["ok", "problem", "unknown"].contains(&doctor.as_str()));
}

#[test]
fn every_verdict_has_a_stable_wire_string() {
    assert_eq!(CliDoctorVerdict::Ok.as_str(), "ok");
    assert_eq!(CliDoctorVerdict::Problem.as_str(), "problem");
    assert_eq!(CliDoctorVerdict::Unknown.as_str(), "unknown");
}
