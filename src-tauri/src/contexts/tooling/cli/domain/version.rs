//! The one place CLI versions are parsed, compared, and ordered.
//!
//! There is exactly one comparison path in the product. React never compares versions, no adapter
//! rolls its own parser, and directory names that carry versions are ordered through this type
//! rather than lexically -- lexical order puts `v10.0.0` before `v9.0.0`, which is how an NVM
//! layout ends up reporting the wrong active installation.
//!
//! A version that does not parse stays *opaque*: it can be compared for equality but has no order.
//! The backend refuses to call an unordered pair an upgrade or a downgrade rather than guessing.

use std::cmp::Ordering;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NormalizedCliVersion {
    /// Exactly what the source reported, preserved for display and for equality against a catalog
    /// entry. Never reconstructed from the parsed parts.
    raw: String,
    parsed: Option<ParsedVersion>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ParsedVersion {
    release: Vec<u64>,
    /// Empty means a stable release, which SemVer orders *above* any prerelease of the same
    /// release numbers.
    prerelease: Vec<PrereleaseSegment>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum PrereleaseSegment {
    Numeric(u64),
    Alphanumeric(String),
}

impl NormalizedCliVersion {
    pub(crate) fn parse(raw: impl Into<String>) -> Self {
        let raw = raw.into();
        let parsed = parse_version(raw.trim());
        Self { raw, parsed }
    }

    /// Extracts the version from `--version` output.
    ///
    /// CLIs disagree about the shape: bare `1.2.3`, `v1.2.3`, `claude-code 1.2.3`, or a banner
    /// line followed by detail. The first token that parses as an ordered version wins; failing
    /// that, the first non-empty line is kept whole as an opaque version rather than discarded --
    /// an unrecognised format still tells the user *something* ran and reported it.
    pub(crate) fn from_probe_output(output: &str) -> Option<Self> {
        let first_line = output
            .lines()
            .map(str::trim)
            .find(|line| !line.is_empty())?;
        let ordered = first_line
            .split_whitespace()
            .map(Self::parse)
            .find(Self::is_ordered);
        Some(ordered.unwrap_or_else(|| Self::parse(first_line)))
    }

    pub(crate) fn as_str(&self) -> &str {
        &self.raw
    }

    /// Whether this version participates in ordering at all. An opaque version still compares
    /// equal to an identical string, which is enough to answer "is the target already active".
    pub(crate) fn is_ordered(&self) -> bool {
        self.parsed.is_some()
    }

    pub(crate) fn is_stable(&self) -> bool {
        self.parsed
            .as_ref()
            .is_some_and(|parsed| parsed.prerelease.is_empty())
    }

    /// `None` when either side is opaque. Callers must treat `None` as "cannot tell", never as
    /// "equal" and never as "upgrade".
    pub(crate) fn compare(&self, other: &Self) -> Option<Ordering> {
        match (&self.parsed, &other.parsed) {
            (Some(left), Some(right)) => Some(compare_parsed(left, right)),
            // Two identical opaque strings are still the same version, which is what stops a
            // "reinstall the version you already have" action from being offered.
            _ if self.raw == other.raw => Some(Ordering::Equal),
            _ => None,
        }
    }

    /// Ordering for display and for picking the newest of several discovered installations.
    /// Opaque versions sort below every ordered one and among themselves by raw text, so the
    /// result is deterministic without implying a version relationship.
    pub(crate) fn display_order(&self, other: &Self) -> Ordering {
        match (&self.parsed, &other.parsed) {
            (Some(left), Some(right)) => compare_parsed(left, right),
            (Some(_), None) => Ordering::Greater,
            (None, Some(_)) => Ordering::Less,
            (None, None) => self.raw.cmp(&other.raw),
        }
    }
}

impl fmt::Display for NormalizedCliVersion {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.raw)
    }
}

fn compare_parsed(left: &ParsedVersion, right: &ParsedVersion) -> Ordering {
    let width = left.release.len().max(right.release.len());
    for index in 0..width {
        // `1.2` and `1.2.0` name the same release; the missing segments are zeros.
        let left_part = left.release.get(index).copied().unwrap_or(0);
        let right_part = right.release.get(index).copied().unwrap_or(0);
        match left_part.cmp(&right_part) {
            Ordering::Equal => continue,
            unequal => return unequal,
        }
    }
    compare_prerelease(&left.prerelease, &right.prerelease)
}

fn compare_prerelease(left: &[PrereleaseSegment], right: &[PrereleaseSegment]) -> Ordering {
    match (left.is_empty(), right.is_empty()) {
        (true, true) => return Ordering::Equal,
        // A release always outranks a prerelease of the same numbers: 1.2.3 > 1.2.3-rc.1.
        (true, false) => return Ordering::Greater,
        (false, true) => return Ordering::Less,
        (false, false) => {}
    }
    for index in 0..left.len().min(right.len()) {
        let ordering = match (&left[index], &right[index]) {
            (PrereleaseSegment::Numeric(a), PrereleaseSegment::Numeric(b)) => a.cmp(b),
            (PrereleaseSegment::Alphanumeric(a), PrereleaseSegment::Alphanumeric(b)) => a.cmp(b),
            // SemVer 2.0.0: numeric identifiers always rank lower than alphanumeric ones.
            (PrereleaseSegment::Numeric(_), PrereleaseSegment::Alphanumeric(_)) => Ordering::Less,
            (PrereleaseSegment::Alphanumeric(_), PrereleaseSegment::Numeric(_)) => {
                Ordering::Greater
            }
        };
        if ordering != Ordering::Equal {
            return ordering;
        }
    }
    // A larger set of prerelease fields outranks a smaller one when the shared prefix is equal.
    left.len().cmp(&right.len())
}

fn parse_version(value: &str) -> Option<ParsedVersion> {
    if value.is_empty() {
        return None;
    }
    let value = value.strip_prefix(['v', 'V']).unwrap_or(value);
    // Build metadata is explicitly ignored when determining precedence.
    let value = value.split('+').next()?;
    let (release, prerelease) = match value.split_once('-') {
        Some((release, prerelease)) => (release, Some(prerelease)),
        None => (value, None),
    };

    let mut release_parts = Vec::new();
    for part in release.split('.') {
        // Rejects an empty segment ("1..2"), a non-numeric one ("1.x"), and anything that would
        // overflow. Any of those makes the whole version opaque rather than half-understood.
        if part.is_empty() || !part.chars().all(|ch| ch.is_ascii_digit()) {
            return None;
        }
        release_parts.push(part.parse::<u64>().ok()?);
    }
    if release_parts.is_empty() {
        return None;
    }

    let prerelease_parts = match prerelease {
        None => Vec::new(),
        Some(prerelease) => {
            if prerelease.is_empty() {
                return None;
            }
            let mut parts = Vec::new();
            for part in prerelease.split('.') {
                if part.is_empty() {
                    return None;
                }
                if part.chars().all(|ch| ch.is_ascii_digit()) {
                    parts.push(PrereleaseSegment::Numeric(part.parse::<u64>().ok()?));
                } else if part
                    .chars()
                    .all(|ch| ch.is_ascii_alphanumeric() || ch == '-')
                {
                    parts.push(PrereleaseSegment::Alphanumeric(part.to_string()));
                } else {
                    return None;
                }
            }
            parts
        }
    };

    Some(ParsedVersion {
        release: release_parts,
        prerelease: prerelease_parts,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn version(raw: &str) -> NormalizedCliVersion {
        NormalizedCliVersion::parse(raw)
    }

    #[test]
    fn release_segments_compare_numerically_not_lexically() {
        // The bug lexical ordering causes: "10" sorts before "9" as text.
        assert_eq!(
            version("1.10.0").compare(&version("1.9.9")),
            Some(Ordering::Greater)
        );
        assert_eq!(
            version("v10.0.0").compare(&version("v9.0.0")),
            Some(Ordering::Greater)
        );
        assert_eq!(
            version("2.0.0").compare(&version("10.0.0")),
            Some(Ordering::Less)
        );
    }

    #[test]
    fn missing_release_segments_are_zeros() {
        assert_eq!(
            version("1.2").compare(&version("1.2.0")),
            Some(Ordering::Equal)
        );
        assert_eq!(
            version("1").compare(&version("1.0.0")),
            Some(Ordering::Equal)
        );
        assert_eq!(
            version("1.2").compare(&version("1.2.1")),
            Some(Ordering::Less)
        );
    }

    #[test]
    fn a_leading_v_and_build_metadata_do_not_change_precedence() {
        assert_eq!(
            version("v1.2.3").compare(&version("1.2.3")),
            Some(Ordering::Equal)
        );
        assert_eq!(
            version("1.2.3+build.5").compare(&version("1.2.3+build.9")),
            Some(Ordering::Equal)
        );
        // The raw text is preserved even though it is ignored for precedence.
        assert_eq!(version("v1.2.3").as_str(), "v1.2.3");
    }

    #[test]
    fn prereleases_rank_below_their_release_and_among_themselves() {
        assert_eq!(
            version("1.2.3-rc.1").compare(&version("1.2.3")),
            Some(Ordering::Less)
        );
        assert_eq!(
            version("1.0.0-alpha").compare(&version("1.0.0-alpha.1")),
            Some(Ordering::Less)
        );
        assert_eq!(
            version("1.0.0-alpha.1").compare(&version("1.0.0-beta")),
            Some(Ordering::Less)
        );
        assert_eq!(
            version("1.0.0-alpha.1").compare(&version("1.0.0-alpha.beta")),
            Some(Ordering::Less)
        );
        assert_eq!(
            version("1.0.0-rc.1").compare(&version("1.0.0-rc.1")),
            Some(Ordering::Equal)
        );
        assert!(!version("1.2.3-rc.1").is_stable());
        assert!(version("1.2.3").is_stable());
    }

    #[test]
    fn an_unparseable_version_is_opaque_and_never_implies_an_upgrade() {
        let nightly = version("nightly");
        assert!(!nightly.is_ordered());
        assert!(!nightly.is_stable());
        // Cannot tell -- and "cannot tell" must not be reported as an upgrade or a downgrade.
        assert_eq!(nightly.compare(&version("1.2.3")), None);
        assert_eq!(version("1.2.3").compare(&nightly), None);
        assert_eq!(version("1.x").compare(&version("1.2")), None);
        assert_eq!(version("1..2").compare(&version("1.0.2")), None);
        assert_eq!(version("1.2.3-").compare(&version("1.2.3")), None);
    }

    #[test]
    fn two_identical_opaque_versions_are_equal_so_no_redundant_action_is_offered() {
        // The target the user picked is textually the version already installed. Even with no
        // ordering available, offering "install" here would be a redundant machine mutation.
        assert_eq!(
            version("nightly").compare(&version("nightly")),
            Some(Ordering::Equal)
        );
        assert_eq!(
            version("2024-03-01").compare(&version("2024-03-01")),
            Some(Ordering::Equal)
        );
        assert_eq!(version("nightly").compare(&version("canary")), None);
    }

    #[test]
    fn display_order_is_total_and_puts_opaque_versions_last() {
        let mut versions = [
            version("nightly"),
            version("1.10.0"),
            version("1.9.9"),
            version("1.10.0-rc.1"),
            version("canary"),
        ];
        versions.sort_by(NormalizedCliVersion::display_order);

        assert_eq!(
            versions
                .iter()
                .map(NormalizedCliVersion::as_str)
                .collect::<Vec<_>>(),
            vec!["canary", "nightly", "1.9.9", "1.10.0-rc.1", "1.10.0"]
        );
    }

    #[test]
    fn probe_output_yields_the_version_whatever_shape_the_cli_prints_it_in() {
        let cases = [
            ("1.2.3", "1.2.3"),
            ("v1.2.3\n", "v1.2.3"),
            ("claude-code 1.2.3", "1.2.3"),
            ("codex-cli version 0.44.0 (build abc)", "0.44.0"),
            ("  1.2.3-rc.1  ", "1.2.3-rc.1"),
            // Blank leading lines are skipped rather than treated as the answer.
            ("\n\n2.0.0", "2.0.0"),
        ];
        for (output, expected) in cases {
            let version = NormalizedCliVersion::from_probe_output(output)
                .unwrap_or_else(|| panic!("parsed {output:?}"));
            assert_eq!(version.as_str(), expected, "output {output:?}");
            assert!(version.is_ordered(), "output {output:?}");
        }
    }

    #[test]
    fn unrecognised_probe_output_is_kept_opaque_rather_than_discarded() {
        // Something ran and answered; the answer is just not a version we can order. Dropping it
        // would render as "version unknown" when the user can plainly see output.
        let opaque = NormalizedCliVersion::from_probe_output("nightly build").expect("kept");
        assert_eq!(opaque.as_str(), "nightly build");
        assert!(!opaque.is_ordered());

        assert_eq!(NormalizedCliVersion::from_probe_output(""), None);
        assert_eq!(NormalizedCliVersion::from_probe_output("   \n  "), None);
    }

    #[test]
    fn surrounding_whitespace_does_not_make_a_version_opaque() {
        // Probe output routinely arrives with a trailing newline already stripped to a space.
        let padded = version(" 1.2.3 ");
        assert!(padded.is_ordered());
        assert_eq!(padded.compare(&version("1.2.3")), Some(Ordering::Equal));
    }
}
