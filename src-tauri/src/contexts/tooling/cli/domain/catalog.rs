//! Versions a *specific source* offers for a *specific tool*.
//!
//! There is no global `latestVersion`. A catalog is stamped with the tool and source it came from,
//! and the update state of an installation may only be computed from the catalog of the source
//! that owns it. Asking a mismatched catalog yields `CatalogUnavailable`, never an answer.
//!
//! That mismatch is the defect this file removes: detection queried the npm registry whenever a
//! tool had an npm package, so a WinGet-installed CLI was told it was out of date by npm.

use chrono::{DateTime, Utc};

use super::ids::{CliSourceId, CliToolId};
use super::status::CliUpdateStatus;
use super::version::NormalizedCliVersion;

/// Why a catalog holds no usable versions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CliCatalogUnavailableReason {
    /// The source is not installed or not reachable on this host.
    SourceUnavailable,
    /// The source ran but its output could not be parsed. Localized or reformatted package-manager
    /// output lands here, and the correct response is to report nothing rather than to guess.
    UnparseableOutput,
    /// The source declined the request -- offline, rate limited, authentication.
    QueryFailed,
    /// This source publishes no catalog at all, e.g. a detect-only installation.
    NotApplicable,
}

impl CliCatalogUnavailableReason {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::SourceUnavailable => "source-unavailable",
            Self::UnparseableOutput => "unparseable-output",
            Self::QueryFailed => "query-failed",
            Self::NotApplicable => "not-applicable",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CliCatalogStatus {
    Available,
    Unavailable(CliCatalogUnavailableReason),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CliVersionCatalog {
    pub(crate) agent_id: CliToolId,
    pub(crate) source_id: CliSourceId,
    /// `None` when the source has no channel concept. `Some("stable")` is the default where it
    /// does; two channels of one source are two catalogs.
    pub(crate) channel: Option<String>,
    /// Newest first, as the source ordered them where it has an opinion.
    pub(crate) versions: Vec<NormalizedCliVersion>,
    pub(crate) latest: Option<NormalizedCliVersion>,
    pub(crate) fetched_at: DateTime<Utc>,
    pub(crate) expires_at: DateTime<Utc>,
    pub(crate) status: CliCatalogStatus,
}

impl CliVersionCatalog {
    pub(crate) fn unavailable(
        agent_id: CliToolId,
        source_id: CliSourceId,
        channel: Option<String>,
        reason: CliCatalogUnavailableReason,
        fetched_at: DateTime<Utc>,
        expires_at: DateTime<Utc>,
    ) -> Self {
        Self {
            agent_id,
            source_id,
            channel,
            versions: Vec::new(),
            latest: None,
            fetched_at,
            expires_at,
            status: CliCatalogStatus::Unavailable(reason),
        }
    }

    pub(crate) fn is_available(&self) -> bool {
        self.status == CliCatalogStatus::Available
    }

    pub(crate) fn is_expired(&self, now: DateTime<Utc>) -> bool {
        now >= self.expires_at
    }

    /// Whether this catalog is the right one to answer questions about `(agent_id, source_id,
    /// channel)`. Channel `None` in the request matches any channel; a specific request must match
    /// exactly.
    pub(crate) fn describes(
        &self,
        agent_id: &CliToolId,
        source_id: &CliSourceId,
        channel: Option<&str>,
    ) -> bool {
        if &self.agent_id != agent_id || &self.source_id != source_id {
            return false;
        }
        match channel {
            None => true,
            Some(requested) => self.channel.as_deref() == Some(requested),
        }
    }

    /// Whether the source offers this exact version. Equality works even for opaque versions,
    /// which is what an exact-target request needs.
    pub(crate) fn offers(&self, version: &NormalizedCliVersion) -> bool {
        self.is_available() && self.versions.contains(version)
    }

    /// Update state of `active` against *this* catalog. Callers must reach this through
    /// `update_status_for_source` so a mismatched catalog cannot be used by accident.
    fn update_status(&self, active: Option<&NormalizedCliVersion>) -> CliUpdateStatus {
        if let CliCatalogStatus::Unavailable(reason) = self.status {
            return match reason {
                CliCatalogUnavailableReason::NotApplicable => CliUpdateStatus::NotApplicable,
                _ => CliUpdateStatus::CatalogUnavailable,
            };
        }
        let (Some(active), Some(latest)) = (active, self.latest.as_ref()) else {
            // Either nothing is installed or the source published no latest. Neither is a
            // statement about whether an update exists.
            return CliUpdateStatus::Unknown;
        };
        match active.compare(latest) {
            Some(std::cmp::Ordering::Equal) => CliUpdateStatus::UpToDate,
            Some(std::cmp::Ordering::Less) => CliUpdateStatus::Available,
            Some(std::cmp::Ordering::Greater) => CliUpdateStatus::Ahead,
            // One side is opaque. Refusing to guess is the point.
            None => CliUpdateStatus::Unknown,
        }
    }
}

/// The only supported way to compute update state.
///
/// A catalog that does not describe this exact tool and source is not consulted -- it yields
/// `CatalogUnavailable` rather than an answer borrowed from another source.
pub(crate) fn update_status_for_source(
    catalog: Option<&CliVersionCatalog>,
    agent_id: &CliToolId,
    source_id: &CliSourceId,
    channel: Option<&str>,
    active: Option<&NormalizedCliVersion>,
) -> CliUpdateStatus {
    match catalog {
        Some(catalog) if catalog.describes(agent_id, source_id, channel) => {
            catalog.update_status(active)
        }
        // Present but for a different source, or absent entirely. Both mean the same thing here:
        // nothing authoritative is available for the source that owns this installation.
        Some(_) | None => CliUpdateStatus::CatalogUnavailable,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn timestamp(seconds: i64) -> DateTime<Utc> {
        DateTime::from_timestamp(seconds, 0).expect("timestamp")
    }

    fn tool() -> CliToolId {
        CliToolId::new("claude-code").expect("tool id")
    }

    fn npm() -> CliSourceId {
        CliSourceId::new("npm").expect("source id")
    }

    fn winget() -> CliSourceId {
        CliSourceId::new("winget").expect("source id")
    }

    fn catalog(source: CliSourceId, versions: &[&str], latest: Option<&str>) -> CliVersionCatalog {
        CliVersionCatalog {
            agent_id: tool(),
            source_id: source,
            channel: Some("stable".to_string()),
            versions: versions
                .iter()
                .map(|version| NormalizedCliVersion::parse(*version))
                .collect(),
            latest: latest.map(NormalizedCliVersion::parse),
            fetched_at: timestamp(1_000),
            expires_at: timestamp(1_900),
            status: CliCatalogStatus::Available,
        }
    }

    #[test]
    fn a_catalog_answers_only_for_the_tool_and_source_it_came_from() {
        let npm_catalog = catalog(npm(), &["2.0.0", "1.0.0"], Some("2.0.0"));

        assert!(npm_catalog.describes(&tool(), &npm(), Some("stable")));
        // Channel-agnostic request matches.
        assert!(npm_catalog.describes(&tool(), &npm(), None));
        assert!(!npm_catalog.describes(&tool(), &winget(), Some("stable")));
        assert!(!npm_catalog.describes(&tool(), &npm(), Some("nightly")));
        assert!(!npm_catalog.describes(
            &CliToolId::new("codex-cli").expect("tool"),
            &npm(),
            Some("stable")
        ));
    }

    #[test]
    fn an_npm_catalog_never_decides_the_update_state_of_a_winget_install() {
        // The defect: npm says 2.0.0 is latest, the machine has a 1.0.0 WinGet install, and the
        // old model reported an available upgrade -- through npm, for a package npm does not own.
        let npm_catalog = catalog(npm(), &["2.0.0", "1.0.0"], Some("2.0.0"));
        let active = NormalizedCliVersion::parse("1.0.0");

        let borrowed = update_status_for_source(
            Some(&npm_catalog),
            &tool(),
            &winget(),
            Some("stable"),
            Some(&active),
        );
        assert_eq!(borrowed, CliUpdateStatus::CatalogUnavailable);

        // Asked correctly, the same catalog does answer.
        let owned = update_status_for_source(
            Some(&npm_catalog),
            &tool(),
            &npm(),
            Some("stable"),
            Some(&active),
        );
        assert_eq!(owned, CliUpdateStatus::Available);
    }

    #[test]
    fn a_detect_only_source_reports_not_applicable_rather_than_a_borrowed_answer() {
        let none = CliVersionCatalog::unavailable(
            tool(),
            CliSourceId::new("manual").expect("source"),
            None,
            CliCatalogUnavailableReason::NotApplicable,
            timestamp(1_000),
            timestamp(1_900),
        );
        let active = NormalizedCliVersion::parse("1.0.0");

        assert_eq!(
            update_status_for_source(
                Some(&none),
                &tool(),
                &CliSourceId::new("manual").expect("source"),
                None,
                Some(&active)
            ),
            CliUpdateStatus::NotApplicable
        );
    }

    #[test]
    fn an_unreadable_catalog_is_distinguished_from_one_that_does_not_exist() {
        for reason in [
            CliCatalogUnavailableReason::SourceUnavailable,
            CliCatalogUnavailableReason::UnparseableOutput,
            CliCatalogUnavailableReason::QueryFailed,
        ] {
            let broken = CliVersionCatalog::unavailable(
                tool(),
                npm(),
                Some("stable".to_string()),
                reason,
                timestamp(1_000),
                timestamp(1_900),
            );
            let active = NormalizedCliVersion::parse("1.0.0");
            // Retrying may help, so this is not `NotApplicable`.
            assert_eq!(
                update_status_for_source(
                    Some(&broken),
                    &tool(),
                    &npm(),
                    Some("stable"),
                    Some(&active)
                ),
                CliUpdateStatus::CatalogUnavailable,
                "{}",
                reason.as_str()
            );
            assert!(!broken.is_available());
            assert!(!broken.offers(&active));
        }
    }

    #[test]
    fn the_update_states_follow_the_comparison_and_never_guess() {
        let npm_catalog = catalog(npm(), &["2.0.0", "1.0.0"], Some("2.0.0"));

        let cases = [
            ("1.0.0", CliUpdateStatus::Available),
            ("2.0.0", CliUpdateStatus::UpToDate),
            // Newer than the catalog: common on a prerelease build, not an update prompt.
            ("3.0.0", CliUpdateStatus::Ahead),
            // Opaque: comparison yields nothing, so neither does the update state.
            ("nightly", CliUpdateStatus::Unknown),
        ];
        for (active, expected) in cases {
            let version = NormalizedCliVersion::parse(active);
            assert_eq!(
                update_status_for_source(
                    Some(&npm_catalog),
                    &tool(),
                    &npm(),
                    Some("stable"),
                    Some(&version)
                ),
                expected,
                "active {active}"
            );
        }
    }

    #[test]
    fn nothing_installed_or_no_published_latest_is_unknown_not_up_to_date() {
        let npm_catalog = catalog(npm(), &["2.0.0"], Some("2.0.0"));
        assert_eq!(
            update_status_for_source(Some(&npm_catalog), &tool(), &npm(), Some("stable"), None),
            CliUpdateStatus::Unknown
        );

        let no_latest = catalog(npm(), &["2.0.0"], None);
        let active = NormalizedCliVersion::parse("2.0.0");
        assert_eq!(
            update_status_for_source(
                Some(&no_latest),
                &tool(),
                &npm(),
                Some("stable"),
                Some(&active)
            ),
            CliUpdateStatus::Unknown
        );
    }

    #[test]
    fn a_missing_catalog_is_unavailable_rather_than_up_to_date() {
        let active = NormalizedCliVersion::parse("1.0.0");
        assert_eq!(
            update_status_for_source(None, &tool(), &npm(), Some("stable"), Some(&active)),
            CliUpdateStatus::CatalogUnavailable
        );
    }

    #[test]
    fn offers_answers_exact_target_requests_including_opaque_versions() {
        let npm_catalog = catalog(npm(), &["2.0.0", "1.0.0", "nightly"], Some("2.0.0"));

        assert!(npm_catalog.offers(&NormalizedCliVersion::parse("1.0.0")));
        // A leading `v` is the same version; equality goes through the parser, not the raw text.
        assert!(npm_catalog.offers(&NormalizedCliVersion::parse("1.0.0")));
        assert!(npm_catalog.offers(&NormalizedCliVersion::parse("nightly")));
        assert!(!npm_catalog.offers(&NormalizedCliVersion::parse("9.9.9")));
    }

    #[test]
    fn expiry_is_evaluated_against_a_supplied_clock() {
        let npm_catalog = catalog(npm(), &["1.0.0"], Some("1.0.0"));
        assert!(!npm_catalog.is_expired(timestamp(1_899)));
        assert!(npm_catalog.is_expired(timestamp(1_900)));
        assert!(npm_catalog.is_expired(timestamp(9_999)));
        assert_eq!(npm_catalog.fetched_at, timestamp(1_000));
    }

    #[test]
    fn unavailable_reasons_have_stable_wire_strings() {
        assert_eq!(
            CliCatalogUnavailableReason::SourceUnavailable.as_str(),
            "source-unavailable"
        );
        assert_eq!(
            CliCatalogUnavailableReason::UnparseableOutput.as_str(),
            "unparseable-output"
        );
        assert_eq!(
            CliCatalogUnavailableReason::QueryFailed.as_str(),
            "query-failed"
        );
        assert_eq!(
            CliCatalogUnavailableReason::NotApplicable.as_str(),
            "not-applicable"
        );
    }
}
