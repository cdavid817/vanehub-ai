//! What decides a span's kind, and what deliberately does not.
//!
//! The view used to classify by substring: a name containing "mcp" was an MCP call, one containing
//! "tool" was a tool. Both directions of that are wrong and neither is visible from the screen — a
//! model span named `chat.completion.tool_choice` rendered as a tool call, and an MCP span named
//! `list_resources` rendered as nothing. The tests below are mostly about names *not* being read.

use super::attributes::{SafeAttributeValue, SafeAttributes};
use super::span_kind::{classify_span_kind, ExecutionSpanKind};

fn attributes(entries: &[(&str, &str)]) -> SafeAttributes {
    SafeAttributes::try_from_entries(entries.iter().map(|(key, value)| {
        (
            (*key).to_string(),
            SafeAttributeValue::String((*value).to_string()),
        )
    }))
    .expect("attributes")
}

/// A producer that states its kind is believed.
///
/// It knows what it did; nothing downstream can know better. The whole reason the attribute exists
/// is that a producer with an unusual shape can say so rather than be inferred at.
#[test]
fn a_declared_kind_wins_over_every_inference() {
    let declared = attributes(&[
        ("vanehub.span.kind", "container"),
        // Would otherwise infer `model`.
        ("gen_ai.operation.name", "chat"),
    ]);

    assert_eq!(classify_span_kind(&declared), ExecutionSpanKind::Container);
}

/// A declared kind this build does not recognise falls through rather than becoming unknown.
///
/// A newer producer naming a kind this version has never heard of still carries the conventional
/// attributes, and those are still readable. Refusing to read them would lose information over a
/// vocabulary mismatch.
#[test]
fn an_unrecognised_declared_kind_falls_through_to_the_conventions() {
    let declared = attributes(&[
        ("vanehub.span.kind", "quantum-entanglement"),
        ("gen_ai.operation.name", "chat"),
    ]);

    assert_eq!(classify_span_kind(&declared), ExecutionSpanKind::Model);
}

#[test]
fn a_model_call_is_recognised_by_its_gen_ai_attributes() {
    assert_eq!(
        classify_span_kind(&attributes(&[("gen_ai.operation.name", "chat")])),
        ExecutionSpanKind::Model
    );
    assert_eq!(
        classify_span_kind(&attributes(&[("gen_ai.request.model", "claude-opus-5")])),
        ExecutionSpanKind::Model
    );
}

#[test]
fn a_tool_call_is_recognised_by_its_tool_name() {
    assert_eq!(
        classify_span_kind(&attributes(&[("gen_ai.tool.name", "read_file")])),
        ExecutionSpanKind::Tool
    );
}

/// `rpc.system` names a protocol, and only one of them is MCP.
#[test]
fn an_rpc_span_is_mcp_only_when_its_system_says_so() {
    assert_eq!(
        classify_span_kind(&attributes(&[("rpc.system", "mcp")])),
        ExecutionSpanKind::Mcp
    );
    // gRPC over the same attribute is network traffic. Treating every RPC as MCP would put
    // unrelated calls in a filter a reader uses to audit MCP servers.
    assert_eq!(
        classify_span_kind(&attributes(&[
            ("rpc.system", "grpc"),
            ("server.address", "example.test"),
        ])),
        ExecutionSpanKind::Network
    );
}

/// An MCP call surfaced as a tool is an MCP call.
///
/// It carries both attributes, so the answer is a decision rather than a coincidence: a reader
/// filtering for MCP traffic has to find it, and one filtering for tools sees it under the server
/// that ran it.
#[test]
fn an_mcp_tool_call_is_classified_as_mcp() {
    let both = attributes(&[("rpc.system", "mcp"), ("gen_ai.tool.name", "search")]);

    assert_eq!(classify_span_kind(&both), ExecutionSpanKind::Mcp);
}

#[test]
fn a_delegation_is_recognised_by_its_target() {
    assert_eq!(
        classify_span_kind(&attributes(&[("vanehub.delegation.target", "agent-2")])),
        ExecutionSpanKind::Delegation
    );
}

#[test]
fn a_process_and_a_file_are_recognised_by_their_own_conventions() {
    assert_eq!(
        classify_span_kind(&attributes(&[("process.command", "git")])),
        ExecutionSpanKind::Process
    );
    assert_eq!(
        classify_span_kind(&attributes(&[("file.path", "readme.md")])),
        ExecutionSpanKind::File
    );
}

/// A span carrying nothing is unknown, not a guess.
///
/// `unknown` tells a reader the producer did not say. A wrong kind tells them something false and
/// gives them no reason to doubt it.
#[test]
fn a_span_with_no_identifying_attribute_is_unknown() {
    assert_eq!(
        classify_span_kind(&SafeAttributes::default()),
        ExecutionSpanKind::Unknown
    );
}

/// The name is never read. This is the substring classification, restated as the thing that must
/// not happen.
#[test]
fn a_name_that_looks_like_a_kind_classifies_nothing() {
    // Every one of these would have been classified by the old substring rule, and every one of
    // them carries no attribute saying what it is.
    for misleading in [
        "mcp.list_resources",
        "tool.invoke",
        "gen_ai.chat",
        "delegate_to_agent",
    ] {
        let named_only = attributes(&[("vanehub.span.label", misleading)]);
        assert_eq!(
            classify_span_kind(&named_only),
            ExecutionSpanKind::Unknown,
            "{misleading} was classified from something that is not an assertion"
        );
    }
}

/// A non-text declared kind is ignored rather than coerced.
#[test]
fn a_declared_kind_that_is_not_text_is_ignored() {
    let numeric = SafeAttributes::try_from_entries([(
        "vanehub.span.kind".to_string(),
        SafeAttributeValue::Integer(3),
    )])
    .expect("attributes");

    assert_eq!(classify_span_kind(&numeric), ExecutionSpanKind::Unknown);
}

/// Every kind has a distinct stable token, because the wire value is what a client switches on.
#[test]
fn every_kind_has_its_own_stable_token() {
    let kinds = [
        ExecutionSpanKind::Model,
        ExecutionSpanKind::Tool,
        ExecutionSpanKind::Mcp,
        ExecutionSpanKind::Process,
        ExecutionSpanKind::Delegation,
        ExecutionSpanKind::File,
        ExecutionSpanKind::Network,
        ExecutionSpanKind::Container,
        ExecutionSpanKind::Unknown,
    ];

    let mut tokens: Vec<&str> = kinds.iter().map(|kind| kind.token()).collect();
    tokens.sort_unstable();
    tokens.dedup();
    assert_eq!(tokens.len(), kinds.len());
    // The default is `unknown`, so a kind nobody set never reads as a real classification.
    assert_eq!(ExecutionSpanKind::default(), ExecutionSpanKind::Unknown);
}
