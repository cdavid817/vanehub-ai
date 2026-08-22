use thiserror::Error;

/// Why an evidence input was refused.
///
/// Every variant names the invariant rather than the field, because these errors reach a
/// rate-limited diagnostic and a coverage reason code, and a message that echoed the offending
/// value would defeat the point of refusing it.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub(crate) enum EvidenceDomainError {
    #[error("evidence {field} must contain 1 to {max} characters")]
    InvalidIdentifier { field: &'static str, max: usize },
    #[error(
        "evidence reason code must be 1 to {max} lowercase ascii letters, digits, or underscores"
    )]
    InvalidReasonCode { max: usize },
    #[error(
        "evidence label '{field}' must contain 1 to {max} characters and no control characters"
    )]
    InvalidLabel { field: &'static str, max: usize },
    #[error("evidence correlation requires a session id")]
    SessionRequired,
    #[error("a span id cannot be recorded without its trace id")]
    SpanWithoutTrace,
    #[error("a parent span id cannot be recorded without its trace id")]
    ParentSpanWithoutTrace,
    #[error("evidence kind '{kind}' does not accept this payload")]
    PayloadKindMismatch { kind: &'static str },
    #[error("evidence kind '{kind}' requires correlation field '{field}'")]
    MissingCorrelation {
        kind: &'static str,
        field: &'static str,
    },
    #[error("a coverage gap must report a dropped count greater than zero")]
    EmptyCoverageGap,
    #[error("safe evidence payload exceeds the {max} byte bound once serialized")]
    PayloadTooLarge { max: usize },
    #[error("a redacted command display must be at most {max} bytes and carry no line breaks")]
    InvalidRedactedDisplay { max: usize },
    #[error("evidence must not carry an absolute or user-rooted path")]
    AbsolutePathRejected,
    #[error("evidence must not carry a file path; only a normalized basename is allowed")]
    PathSeparatorRejected,
    #[error("evidence must not carry credential-shaped content")]
    CredentialShapedContentRejected,
    #[error("evidence redaction receipt accepts at most {max} rule ids")]
    TooManyRedactionRules { max: usize },
    #[error("evidence array field '{field}' accepts at most {max} entries")]
    TooManyEntries { field: &'static str, max: usize },
    #[error("evidence schema version {version} is not supported by this build")]
    UnsupportedSchemaVersion { version: u16 },
    #[error("evidence timestamp must be a bounded RFC 3339 value")]
    InvalidTimestamp,
}
