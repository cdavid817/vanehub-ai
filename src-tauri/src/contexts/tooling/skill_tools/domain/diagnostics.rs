/// Longest diagnostic summary that may be persisted for one tool revision.
pub(crate) const MAX_DIAGNOSTIC_SUMMARY_CHARACTERS: usize = 512;
/// Longest single detail fragment inside a summary.
pub(crate) const MAX_DIAGNOSTIC_DETAIL_CHARACTERS: usize = 120;
/// Entries retained per revision. Older entries are dropped rather than paged: this is an
/// operator hint, not an audit trail — unified logging owns the durable record.
pub(crate) const MAX_DIAGNOSTIC_ENTRIES: usize = 5;

const REDACTED: &str = "[redacted]";

/// Substrings that mark a value as sensitive regardless of where it came from.
///
/// Matched case-insensitively against the *detail* text, because a manifest author controls the
/// field names in their own schemas and a schema-declared sensitivity flag alone would leave
/// third-party payloads unredacted.
const SENSITIVE_MARKERS: &[&str] = &[
    "password",
    "passwd",
    "secret",
    "token",
    "api_key",
    "apikey",
    "api-key",
    "authorization",
    "bearer",
    "credential",
    "private_key",
    "session_key",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum SkillToolDiagnosticSeverity {
    Info,
    Warn,
    Error,
}

impl SkillToolDiagnosticSeverity {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Info => "info",
            Self::Warn => "warn",
            Self::Error => "error",
        }
    }
}

/// One bounded, already-redacted diagnostic line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SkillToolDiagnostic {
    pub(crate) severity: SkillToolDiagnosticSeverity,
    pub(crate) code: String,
    pub(crate) detail: String,
}

impl SkillToolDiagnostic {
    /// Builds a diagnostic whose detail is redacted and truncated at construction.
    ///
    /// Redaction happens here rather than at the persistence edge so no code path can hold an
    /// unredacted `SkillToolDiagnostic` and later write it somewhere the edge does not cover.
    pub(crate) fn new(
        severity: SkillToolDiagnosticSeverity,
        code: impl Into<String>,
        detail: &str,
    ) -> Self {
        Self {
            severity,
            code: code.into().chars().take(64).collect(),
            detail: redact_detail(detail),
        }
    }
}

/// The bounded diagnostic state persisted for one tool revision and shown in governance surfaces.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct SkillToolDiagnosticSummary {
    entries: Vec<SkillToolDiagnostic>,
}

impl SkillToolDiagnosticSummary {
    pub(crate) fn from_entries(entries: Vec<SkillToolDiagnostic>) -> Self {
        let mut summary = Self::default();
        for entry in entries {
            summary.push(entry);
        }
        summary
    }

    /// Keeps the most recent entries. Newest-wins because the latest failure is what an operator
    /// is looking at when they open the surface.
    pub(crate) fn push(&mut self, entry: SkillToolDiagnostic) {
        self.entries.push(entry);
        if self.entries.len() > MAX_DIAGNOSTIC_ENTRIES {
            let overflow = self.entries.len() - MAX_DIAGNOSTIC_ENTRIES;
            self.entries.drain(..overflow);
        }
    }

    pub(crate) fn entries(&self) -> &[SkillToolDiagnostic] {
        &self.entries
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub(crate) fn highest_severity(&self) -> Option<SkillToolDiagnosticSeverity> {
        self.entries.iter().map(|entry| entry.severity).max()
    }

    /// Flattens to the single bounded string persistence stores.
    pub(crate) fn to_storage_value(&self) -> String {
        let joined = self
            .entries
            .iter()
            .map(|entry| {
                format!(
                    "{}|{}|{}",
                    entry.severity.as_str(),
                    entry.code,
                    entry.detail
                )
            })
            .collect::<Vec<_>>()
            .join("\n");
        joined
            .chars()
            .take(MAX_DIAGNOSTIC_SUMMARY_CHARACTERS)
            .collect()
    }

    /// Rebuilds from persistence. Unparseable lines are dropped rather than failing the read:
    /// a corrupt hint must never make a Skill's tool inventory unreadable.
    pub(crate) fn from_storage_value(value: &str) -> Self {
        let entries = value
            .lines()
            .filter_map(|line| {
                let mut parts = line.splitn(3, '|');
                let severity = match parts.next()? {
                    "info" => SkillToolDiagnosticSeverity::Info,
                    "warn" => SkillToolDiagnosticSeverity::Warn,
                    "error" => SkillToolDiagnosticSeverity::Error,
                    _ => return None,
                };
                Some(SkillToolDiagnostic::new(
                    severity,
                    parts.next()?,
                    parts.next()?,
                ))
            })
            .collect();
        Self::from_entries(entries)
    }
}

/// Replaces a sensitive-looking detail entirely and bounds whatever survives.
///
/// Whole-value replacement rather than masking the matched token: a detail like
/// `authorization header rejected: Bearer abc123` has the secret next to the marker, not in it.
fn redact_detail(detail: &str) -> String {
    let lowered = detail.to_ascii_lowercase();
    if SENSITIVE_MARKERS
        .iter()
        .any(|marker| lowered.contains(marker))
    {
        return REDACTED.to_string();
    }
    detail
        .chars()
        .filter(|character| !character.is_control())
        .take(MAX_DIAGNOSTIC_DETAIL_CHARACTERS)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn error(detail: &str) -> SkillToolDiagnostic {
        SkillToolDiagnostic::new(SkillToolDiagnosticSeverity::Error, "limit-breach", detail)
    }

    #[test]
    fn sensitive_details_are_redacted_at_construction() {
        for detail in [
            "authorization header rejected: Bearer abc123",
            "API_KEY=sk-live-1234567890",
            "connection refused (password=hunter2)",
            "Private_Key could not be parsed",
        ] {
            assert_eq!(error(detail).detail, "[redacted]");
        }
        assert_eq!(
            error("module trapped at offset 42").detail,
            "module trapped at offset 42"
        );
    }

    #[test]
    fn details_and_codes_are_bounded_and_control_characters_are_stripped() {
        let long = error(&"x".repeat(1024));
        assert_eq!(long.detail.len(), MAX_DIAGNOSTIC_DETAIL_CHARACTERS);
        assert_eq!(
            SkillToolDiagnostic::new(
                SkillToolDiagnosticSeverity::Warn,
                "c".repeat(200),
                "line\nbreak\ttab"
            )
            .code
            .len(),
            64
        );
        assert_eq!(
            SkillToolDiagnostic::new(SkillToolDiagnosticSeverity::Warn, "x", "line\nbreak").detail,
            "linebreak"
        );
    }

    #[test]
    fn a_summary_keeps_only_the_most_recent_bounded_entries() {
        let mut summary = SkillToolDiagnosticSummary::default();
        assert!(summary.is_empty());
        for index in 0..(MAX_DIAGNOSTIC_ENTRIES + 3) {
            summary.push(error(&format!("failure {index}")));
        }
        assert_eq!(summary.entries().len(), MAX_DIAGNOSTIC_ENTRIES);
        assert_eq!(summary.entries()[0].detail, "failure 3");
        assert_eq!(
            summary.highest_severity(),
            Some(SkillToolDiagnosticSeverity::Error)
        );
        assert!(summary.to_storage_value().chars().count() <= MAX_DIAGNOSTIC_SUMMARY_CHARACTERS);
    }

    #[test]
    fn storage_round_trips_and_survives_corruption() {
        let summary = SkillToolDiagnosticSummary::from_entries(vec![
            SkillToolDiagnostic::new(SkillToolDiagnosticSeverity::Info, "discovered", "2 tools"),
            error("module trapped"),
        ]);
        let restored = SkillToolDiagnosticSummary::from_storage_value(&summary.to_storage_value());
        assert_eq!(restored, summary);

        let corrupt = SkillToolDiagnosticSummary::from_storage_value(
            "garbage\nnotalevel|code|detail\nerror|limit-breach|out of fuel",
        );
        assert_eq!(corrupt.entries().len(), 1);
        assert_eq!(corrupt.entries()[0].detail, "out of fuel");
    }

    #[test]
    fn storage_never_carries_a_sensitive_detail_back_out_of_persistence() {
        let smuggled = SkillToolDiagnosticSummary::from_storage_value(
            "error|limit-breach|authorization: Bearer abc123",
        );
        assert_eq!(smuggled.entries()[0].detail, "[redacted]");
    }
}
