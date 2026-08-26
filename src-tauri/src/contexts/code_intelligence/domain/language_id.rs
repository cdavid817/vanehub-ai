//! Validated identifier for a supported language.
//!
//! Once the storage layer stopped constraining which language ids may exist, this type is what
//! keeps a malformed one from becoming a primary key, a localization key suffix, or a map key
//! that two call sites disagree about.

use std::fmt;

/// Long enough for any real language id, short enough that a malformed value cannot become an
/// unbounded key in a map or a log line.
const MAX_LENGTH: usize = 64;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LanguageIdError {
    Empty,
    TooLong { length: usize },
    UnsupportedCharacter,
}

impl fmt::Display for LanguageIdError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => formatter.write_str("language id must not be empty"),
            Self::TooLong { length } => write!(
                formatter,
                "language id is {length} characters; the maximum is {MAX_LENGTH}"
            ),
            Self::UnsupportedCharacter => formatter.write_str(
                "language id must contain only lowercase ASCII letters, digits, and underscores",
            ),
        }
    }
}

/// Stricter than a general identifier check on purpose. A language id is concatenated into the
/// `lspSettings.language.<id>` localization key and stored as a primary key, so restricting it to
/// `[a-z0-9_]` removes casing ambiguity and any question of what a separator or a control
/// character would do to either consumer.
fn validate(value: &str) -> Result<(), LanguageIdError> {
    if value.is_empty() {
        return Err(LanguageIdError::Empty);
    }
    if value.len() > MAX_LENGTH {
        return Err(LanguageIdError::TooLong {
            length: value.len(),
        });
    }
    if !value
        .bytes()
        .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_')
    {
        return Err(LanguageIdError::UnsupportedCharacter);
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct LspLanguageId(String);

impl LspLanguageId {
    /// For anything that came from outside: a wire DTO field, a stored row, a tool argument.
    pub(crate) fn new(value: impl Into<String>) -> Result<Self, LanguageIdError> {
        let value = value.into();
        validate(&value)?;
        Ok(Self(value))
    }

    /// For a literal this repository wrote itself, which is the registry and nothing else.
    ///
    /// Validation exists to reject external input; a literal in the registry table is not that.
    /// `expect`ing here would put a panic in a release binary to guard against a typo the
    /// registry-completeness test already catches, so the check is a debug assertion that fires
    /// in every test run and costs a user nothing.
    pub(crate) fn trusted(value: &'static str) -> Self {
        debug_assert!(
            validate(value).is_ok(),
            "language id declared in the registry is invalid"
        );
        Self(value.to_string())
    }

    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for LspLanguageId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl PartialEq<str> for LspLanguageId {
    fn eq(&self, other: &str) -> bool {
        self.0 == other
    }
}
