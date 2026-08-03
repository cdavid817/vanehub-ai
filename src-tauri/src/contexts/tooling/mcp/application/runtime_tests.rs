use super::*;
use serde_json::json;

#[test]
fn runtime_error_code_and_display_ignore_upstream_diagnostic_text() {
    let secret = "Authorization: Bearer upstream-secret";
    let error = McpRuntimeError::with_diagnostic(McpFailureCode::Protocol, secret);

    assert_eq!(error.code().as_str(), "protocol");
    assert_eq!(error.to_string(), McpFailureCode::Protocol.safe_message());
    assert!(!error.to_string().contains(secret));
    assert_eq!(error.diagnostic(), Some(secret));
}

#[test]
fn default_limits_match_the_specified_contract() {
    let limits = McpLimits::DEFAULT;
    assert_eq!(limits.import_document_bytes, 1024 * 1024);
    assert_eq!(limits.import_server_entries, 128);
    assert_eq!(limits.configuration_collection_entries, 128);
    assert_eq!(limits.configuration_serialized_bytes, 256 * 1024);
    assert_eq!(limits.protocol_message_bytes, 2 * 1024 * 1024);
    assert_eq!(limits.tools_per_server, 128);
    assert_eq!(limits.catalog_serialized_bytes, 2 * 1024 * 1024);
    assert_eq!(limits.provider_tools, 256);
    assert_eq!(limits.tool_name_bytes, 256);
    assert_eq!(limits.tool_description_bytes, 8 * 1024);
    assert_eq!(limits.schema_bytes, 128 * 1024);
    assert_eq!(limits.json_depth, 32);
    assert_eq!(limits.tool_arguments_bytes, 256 * 1024);
    assert_eq!(limits.tool_result_bytes, 1024 * 1024);
    assert_eq!(limits.stderr_bytes, 64 * 1024);
}

#[test]
fn byte_and_count_limits_accept_boundary_and_reject_limit_plus_one() {
    let limits = McpLimits::DEFAULT;
    assert!(limits.validate_bytes("frame", 10, 10).is_ok());
    assert!(limits.validate_count("tools", 10, 10).is_ok());

    for error in [
        limits.validate_bytes("frame", 11, 10).expect_err("bytes"),
        limits.validate_count("tools", 11, 10).expect_err("count"),
    ] {
        assert_eq!(error.code(), McpFailureCode::LimitExceeded);
    }
}

#[test]
fn serialized_and_json_depth_limits_are_independently_enforced() {
    let limits = McpLimits::DEFAULT;
    let value = json!({ "outer": { "inner": true } });
    let exact_size = serde_json::to_vec(&value).expect("serialize").len();

    assert_eq!(
        limits
            .validate_serialized("value", &value, exact_size)
            .expect("exact size"),
        exact_size
    );
    assert_eq!(
        limits
            .validate_serialized("value", &value, exact_size - 1)
            .expect_err("oversized")
            .code(),
        McpFailureCode::LimitExceeded
    );
    assert!(limits.validate_json("value", &value, exact_size, 3).is_ok());
    assert_eq!(
        limits
            .validate_json("value", &value, exact_size, 2)
            .expect_err("too deep")
            .code(),
        McpFailureCode::LimitExceeded
    );
}

#[test]
fn execution_control_shares_deadline_and_cancellation() {
    let control = McpExecutionControl::with_timeout(Duration::from_secs(1));
    let clone = control.clone();
    assert!(control.remaining().is_ok());

    clone.cancellation().cancel();

    assert!(control.is_cancelled());
    assert_eq!(
        control.remaining().expect_err("cancelled").code(),
        McpFailureCode::Cancelled
    );
}

#[test]
fn expired_execution_control_returns_timeout() {
    let control = McpExecutionControl::with_deadline(Instant::now());
    assert_eq!(
        control.remaining().expect_err("expired").code(),
        McpFailureCode::Timeout
    );
}
