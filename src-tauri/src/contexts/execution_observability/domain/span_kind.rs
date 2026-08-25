//! What a span *is*, decided from its attributes rather than from its name.
//!
//! The Traces tab used to classify by substring — a name containing "mcp" was an MCP call, one
//! containing "tool" was a tool. That is wrong in both directions and neither direction is visible:
//! a model span named `chat.completion.tool_choice` was shown as a tool call, and an MCP span named
//! `list_resources` was shown as nothing at all. Worse, the classification lived in the view, so it
//! could disagree with anything else that reasoned about the same span.
//!
//! Attributes are the right input because they are what the producer actually asserted. A span
//! carrying `gen_ai.operation.name` came from something that knows it made a model call; a span
//! whose name happens to contain "gen" did not. When no attribute says what a span is, the answer
//! is `Unknown` — not a guess dressed as a fact.

use super::attributes::{SafeAttributeValue, SafeAttributes};

/// The shapes of work a span can represent.
///
/// Deliberately small. Each variant exists because the UI does something different for it — a
/// distinct legend entry, a distinct filter, a distinct detail section — and a kind nobody renders
/// differently would be a distinction the reader has to learn for nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub(crate) enum ExecutionSpanKind {
    /// A model call: a generation, an embedding, a completion.
    Model,
    /// A tool invocation the agent runtime executed.
    Tool,
    /// A call to an MCP server.
    Mcp,
    /// A shell or subprocess execution.
    Process,
    /// Work handed to another agent.
    Delegation,
    /// A file read or write.
    File,
    /// An outbound HTTP or RPC call that is none of the above.
    Network,
    /// A span that owns other spans and does no work of its own.
    Container,
    /// Nothing the span carries says what it is.
    ///
    /// The default, and the honest answer far more often than a guess would be. A reader seeing
    /// `unknown` learns that the producer did not say; a reader seeing a wrong kind learns
    /// something false and has no reason to doubt it.
    #[default]
    Unknown,
}

impl ExecutionSpanKind {
    pub(crate) fn token(self) -> &'static str {
        match self {
            Self::Model => "model",
            Self::Tool => "tool",
            Self::Mcp => "mcp",
            Self::Process => "process",
            Self::Delegation => "delegation",
            Self::File => "file",
            Self::Network => "network",
            Self::Container => "container",
            Self::Unknown => "unknown",
        }
    }
}

/// The attribute VaneHub producers set when they know their own kind.
///
/// Checked first and trusted absolutely: a producer that states its kind is more authoritative
/// than any convention this function could infer from, and the whole point of having it is that a
/// producer with an unusual shape can say so rather than being misclassified.
const VANEHUB_KIND: &str = "vanehub.span.kind";

/// Attributes that identify a kind by their presence, in priority order.
///
/// Order matters where a span legitimately carries more than one. An MCP tool call has both
/// `rpc.system=mcp` and `gen_ai.tool.name`: it is an MCP call that happens to be surfaced as a
/// tool, and a reader filtering for MCP traffic needs to find it. So MCP wins over tool, and the
/// order here is that decision written down rather than an accident of matching.
const KIND_BY_ATTRIBUTE: &[(&str, ExecutionSpanKind)] = &[
    ("rpc.system", ExecutionSpanKind::Mcp),
    ("vanehub.delegation.target", ExecutionSpanKind::Delegation),
    ("gen_ai.tool.name", ExecutionSpanKind::Tool),
    ("gen_ai.operation.name", ExecutionSpanKind::Model),
    ("gen_ai.request.model", ExecutionSpanKind::Model),
    ("process.command", ExecutionSpanKind::Process),
    ("process.executable.name", ExecutionSpanKind::Process),
    ("file.path", ExecutionSpanKind::File),
    ("http.request.method", ExecutionSpanKind::Network),
    ("server.address", ExecutionSpanKind::Network),
];

/// Classifies one span from what it carries.
///
/// Never reads the span name. A name is a label a human chose and can contain any word for any
/// reason; an attribute is an assertion a producer made. Deriving a kind from the first would put
/// the classification at the mercy of whoever last renamed something.
pub(crate) fn classify_span_kind(attributes: &SafeAttributes) -> ExecutionSpanKind {
    if let Some(declared) = attributes.entries().get(VANEHUB_KIND).and_then(safe_text) {
        if let Some(kind) = parse_kind(declared) {
            return kind;
        }
    }
    for (attribute, kind) in KIND_BY_ATTRIBUTE {
        // `rpc.system` names a protocol, and only one of them is MCP. A gRPC call carrying the
        // same attribute is network traffic, not an MCP invocation.
        if *attribute == "rpc.system" {
            if attributes.entries().get(*attribute).and_then(safe_text) == Some("mcp") {
                return ExecutionSpanKind::Mcp;
            }
            continue;
        }
        if attributes.entries().contains_key(*attribute) {
            return *kind;
        }
    }
    ExecutionSpanKind::Unknown
}

fn parse_kind(value: &str) -> Option<ExecutionSpanKind> {
    match value {
        "model" => Some(ExecutionSpanKind::Model),
        "tool" => Some(ExecutionSpanKind::Tool),
        "mcp" => Some(ExecutionSpanKind::Mcp),
        "process" => Some(ExecutionSpanKind::Process),
        "delegation" => Some(ExecutionSpanKind::Delegation),
        "file" => Some(ExecutionSpanKind::File),
        "network" => Some(ExecutionSpanKind::Network),
        "container" => Some(ExecutionSpanKind::Container),
        // An unrecognised declared kind falls through to inference rather than becoming `Unknown`
        // outright: a newer producer naming a kind this build does not have still carries the
        // conventional attributes, and those are still readable.
        _ => None,
    }
}

fn safe_text(value: &SafeAttributeValue) -> Option<&str> {
    match value {
        SafeAttributeValue::String(text) => Some(text.as_str()),
        _ => None,
    }
}
