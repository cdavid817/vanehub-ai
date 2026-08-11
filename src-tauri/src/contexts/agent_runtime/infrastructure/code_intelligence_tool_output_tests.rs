use super::code_intelligence_tool_output::{diagnostics_outcome, hover_outcome, locations_outcome};
use super::tools::MAX_TOOL_OUTPUT_BYTES;
use crate::contexts::agent_runtime::application::{
    AgentCodeDiagnostic, AgentCodeHover, AgentCodeIntelligenceMetadata,
    AgentCodeIntelligenceOutcome, AgentCodeIntelligenceStatus, AgentCodeRange,
};

#[test]
fn oversized_diagnostic_result_remains_valid_json_and_reports_truncation() {
    let diagnostics = (0..200)
        .map(|index| AgentCodeDiagnostic {
            file: format!("src/file_{index}.rs"),
            range: range(),
            severity: Some("error".to_owned()),
            message: "界".repeat(2_000),
            source: Some("fixture".to_owned()),
            code: Some("E0001".to_owned()),
        })
        .collect::<Vec<_>>();
    let outcome = diagnostics_outcome(ready(diagnostics));

    assert!(!outcome.is_error);
    assert!(outcome.output.len() <= MAX_TOOL_OUTPUT_BYTES);
    let value: serde_json::Value = serde_json::from_str(&outcome.output).expect("valid JSON");
    assert_eq!(value["metadata"]["truncated"], true);
    assert!(value["metadata"]["returned_count"].as_u64().unwrap_or(0) < 200);
}

#[test]
fn fail_soft_status_is_a_successful_visible_tool_outcome() {
    let outcome = locations_outcome(
        "definitions",
        degraded(AgentCodeIntelligenceStatus::Warming, "indexing"),
        20,
    );

    assert!(!outcome.is_error);
    let value: serde_json::Value = serde_json::from_str(&outcome.output).expect("valid JSON");
    assert_eq!(value["metadata"]["status"], "warming");
    assert_eq!(value["metadata"]["reason_code"], "indexing");
    assert_eq!(value["definitions"], serde_json::json!([]));
}

#[test]
fn hover_content_is_utf8_bounded_and_marks_metadata() {
    let outcome = hover_outcome(ready(Some(AgentCodeHover {
        signature: Some("fn example()".to_owned()),
        documentation: Some("😀".repeat(4_000)),
        range: Some(range()),
    })));

    let value: serde_json::Value = serde_json::from_str(&outcome.output).expect("valid JSON");
    assert_eq!(value["metadata"]["truncated"], true);
    assert!(value["hover"]["documentation"]
        .as_str()
        .is_some_and(|documentation| documentation.len() <= 4_096));
}

fn ready<T>(value: T) -> AgentCodeIntelligenceOutcome<T> {
    AgentCodeIntelligenceOutcome {
        metadata: metadata(AgentCodeIntelligenceStatus::Ready, None),
        value: Some(value),
    }
}

fn degraded<T>(
    status: AgentCodeIntelligenceStatus,
    reason: &str,
) -> AgentCodeIntelligenceOutcome<T> {
    AgentCodeIntelligenceOutcome {
        metadata: metadata(status, Some(reason)),
        value: None,
    }
}

fn metadata(
    status: AgentCodeIntelligenceStatus,
    reason: Option<&str>,
) -> AgentCodeIntelligenceMetadata {
    AgentCodeIntelligenceMetadata {
        status,
        server: Some("fixture".to_owned()),
        language: Some("rust".to_owned()),
        document_version: Some(1),
        stale: false,
        returned_count: 0,
        total: 200,
        truncated: false,
        filtered_count: 0,
        reason_code: reason.map(str::to_owned),
    }
}

fn range() -> AgentCodeRange {
    AgentCodeRange {
        start_line: 1,
        start_column: 1,
        end_line: 1,
        end_column: 2,
    }
}
