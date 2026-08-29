use crate::contexts::personalization::application::SecretRedactionPort;

/// The platform's redaction rule, as this context sees it.
///
/// One adapter over the shared implementation rather than a second rule here. A preview that
/// redacted less than a log line would be exactly where a token escaped, and two rules drift the
/// moment either is improved.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct PlatformSecretRedaction;

impl SecretRedactionPort for PlatformSecretRedaction {
    fn redact(&self, text: &str) -> String {
        crate::platform::logging::redact_text(text)
    }
}
