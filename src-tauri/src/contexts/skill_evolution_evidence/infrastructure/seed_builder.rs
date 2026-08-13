use std::collections::{BTreeMap, BTreeSet};

use chrono::{DateTime, Duration, Utc};
use serde::Serialize;
use sha2::{Digest, Sha256};

pub(super) const SEED_BUILDER_V1: u16 = 1;
pub(crate) const MAX_SIGNALS_PER_SEED: usize = 100;
const GROUP_WINDOW_DAYS: i64 = 14;

#[derive(Debug, Clone)]
pub(super) struct SeedSignalRow {
    pub(super) signal_id: String,
    pub(super) workspace: Option<String>,
    pub(super) category: String,
    pub(super) polarity: String,
    pub(super) severity: String,
    pub(super) attribution: String,
    pub(super) occurred_at: String,
    pub(super) run_id: Option<String>,
    pub(super) attempt_id: Option<String>,
    pub(super) extractor_version: u16,
    pub(super) sanitizer_version: u16,
    pub(super) fingerprint_version: u16,
    pub(super) fingerprint: String,
    pub(super) discriminator: String,
    pub(super) safe_summary: String,
    pub(super) source_agent: Option<String>,
    pub(super) verified_targets: Vec<String>,
    pub(super) human_targets: Vec<String>,
}

#[derive(Debug, Clone)]
pub(super) struct SeedProjection {
    pub(super) seed_id: String,
    pub(super) grouping_hash: String,
    pub(super) workspace: Option<String>,
    pub(super) category: String,
    pub(super) readiness: &'static str,
    pub(super) readiness_reason: &'static str,
    pub(super) safe_summary: String,
    pub(super) first_occurred_at: String,
    pub(super) last_occurred_at: String,
    pub(super) independent_run_count: usize,
    pub(super) has_recovery: bool,
    pub(super) lineage_json: String,
    pub(super) signal_ids: Vec<String>,
    pub(super) lineage_signal_count: usize,
    pub(super) lineage_truncated_count: usize,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct LineageSummary {
    signal_ids: Vec<String>,
    signal_count: usize,
    truncated_signal_count: usize,
    extractor_versions: BTreeSet<u16>,
    sanitizer_versions: BTreeSet<u16>,
    fingerprint_versions: BTreeSet<u16>,
    categories: BTreeMap<String, usize>,
    polarities: BTreeMap<String, usize>,
    severities: BTreeMap<String, usize>,
    attributions: BTreeMap<String, usize>,
    verified_target_hints: BTreeSet<String>,
    human_only_target_hints: BTreeSet<String>,
    source_agents: BTreeMap<String, usize>,
}

pub(super) fn build_seeds(
    mut signals: Vec<SeedSignalRow>,
) -> Result<Vec<SeedProjection>, serde_json::Error> {
    attach_recoveries(&mut signals);
    signals.sort_by(|a, b| {
        base_key(a)
            .cmp(&base_key(b))
            .then(a.occurred_at.cmp(&b.occurred_at))
            .then(a.signal_id.cmp(&b.signal_id))
    });
    let mut clusters: Vec<Vec<SeedSignalRow>> = Vec::new();
    for signal in signals {
        let joins = clusters.last().is_some_and(|cluster| {
            base_key(&cluster[0]) == base_key(&signal)
                && within_window(&cluster[0].occurred_at, &signal.occurred_at)
        });
        if joins {
            if let Some(cluster) = clusters.last_mut() {
                cluster.push(signal);
            }
        } else {
            clusters.push(vec![signal]);
        }
    }
    clusters
        .into_iter()
        .map(project_seed)
        .collect::<Result<Vec<_>, _>>()
        .map(|seeds| {
            seeds
                .into_iter()
                .filter(|seed| seed.readiness != "collecting" && seed.readiness != "ineligible")
                .collect()
        })
}

fn attach_recoveries(signals: &mut [SeedSignalRow]) {
    let attempts: BTreeMap<String, (String, String, Vec<String>, Vec<String>)> = signals
        .iter()
        .filter(|signal| signal.polarity == "negative")
        .filter_map(|signal| {
            signal.attempt_id.as_ref().map(|attempt| {
                (
                    attempt.clone(),
                    (
                        signal.category.clone(),
                        signal.fingerprint.clone(),
                        signal.verified_targets.clone(),
                        signal.human_targets.clone(),
                    ),
                )
            })
        })
        .collect();
    for signal in signals
        .iter_mut()
        .filter(|signal| signal.category == "retry_recovery")
    {
        let predecessor = signal.discriminator.strip_prefix("retry:");
        if let Some((category, fingerprint, verified, human)) =
            predecessor.and_then(|id| attempts.get(id))
        {
            signal.category.clone_from(category);
            signal.fingerprint.clone_from(fingerprint);
            signal.verified_targets.clone_from(verified);
            signal.human_targets.clone_from(human);
        }
    }
}

fn base_key(signal: &SeedSignalRow) -> String {
    let mut cohort = signal.verified_targets.clone();
    cohort.extend(signal.human_targets.clone());
    cohort.sort();
    format!(
        "{}|{}|{}|{}|{}",
        signal.workspace.as_deref().unwrap_or("global"),
        signal.category,
        signal.fingerprint,
        signal.attribution,
        cohort.join(",")
    )
}

fn within_window(first: &str, current: &str) -> bool {
    let Ok(first) = DateTime::parse_from_rfc3339(first) else {
        return false;
    };
    let Ok(current) = DateTime::parse_from_rfc3339(current) else {
        return false;
    };
    current - first <= Duration::days(GROUP_WINDOW_DAYS)
}

fn project_seed(mut signals: Vec<SeedSignalRow>) -> Result<SeedProjection, serde_json::Error> {
    signals.sort_by(|a, b| {
        a.occurred_at
            .cmp(&b.occurred_at)
            .then(a.signal_id.cmp(&b.signal_id))
    });
    let first = signals
        .first()
        .map(|s| s.occurred_at.clone())
        .unwrap_or_default();
    let last = signals
        .last()
        .map(|s| s.occurred_at.clone())
        .unwrap_or_default();
    let runs: BTreeSet<_> = signals.iter().filter_map(|s| s.run_id.clone()).collect();
    let has_recovery = signals
        .iter()
        .any(|s| s.polarity == "positive" && s.discriminator.starts_with("retry:"));
    let corrected = signals.iter().any(|s| {
        s.category == "explicit_feedback"
            && s.polarity == "negative"
            && s.attribution == "verified"
            && s.safe_summary.starts_with("corrected:")
    });
    let negative_runs: BTreeSet<_> = signals
        .iter()
        .filter(|s| s.polarity == "negative")
        .filter_map(|s| s.run_id.clone())
        .collect();
    let (readiness, reason) = if corrected {
        ("ready", "verified_corrected_feedback")
    } else if (negative_runs.len() >= 2 || (has_recovery && !negative_runs.is_empty()))
        && signals
            .iter()
            .all(|signal| signal.attribution == "correlated")
    {
        ("human_review_only", "correlated_evidence_only")
    } else if negative_runs.len() >= 2 || (has_recovery && !negative_runs.is_empty()) {
        (
            "ready",
            if has_recovery {
                "verified_recovery_delta"
            } else {
                "independent_negative_runs"
            },
        )
    } else if signals.iter().all(|s| {
        s.attribution == "weak" || s.attribution == "unattributed" || s.polarity == "neutral"
    }) {
        ("ineligible", "weak_neutral_or_unattributed")
    } else {
        ("collecting", "insufficient_independent_evidence")
    };
    let grouping_hash = hash_text(&format!(
        "v{SEED_BUILDER_V1}|{}|{first}",
        base_key(&signals[0])
    ));
    let lineage = lineage(&signals);
    let category = signals[0].category.clone();
    let safe_summary = format!(
        "{category} pattern; signals={} runs={} recovery={has_recovery}",
        signals.len(),
        runs.len()
    );
    Ok(SeedProjection {
        seed_id: format!("seed-v{SEED_BUILDER_V1}-{}", &grouping_hash[..32]),
        grouping_hash,
        workspace: signals[0].workspace.clone(),
        category,
        readiness,
        readiness_reason: reason,
        safe_summary,
        first_occurred_at: first,
        last_occurred_at: last,
        independent_run_count: runs.len(),
        has_recovery,
        lineage_json: serde_json::to_string(&lineage)?,
        signal_ids: lineage.signal_ids,
        lineage_signal_count: signals.len(),
        lineage_truncated_count: signals.len().saturating_sub(MAX_SIGNALS_PER_SEED),
    })
}

fn lineage(signals: &[SeedSignalRow]) -> LineageSummary {
    let mut value = LineageSummary {
        signal_ids: Vec::new(),
        signal_count: signals.len(),
        truncated_signal_count: signals.len().saturating_sub(MAX_SIGNALS_PER_SEED),
        extractor_versions: BTreeSet::new(),
        sanitizer_versions: BTreeSet::new(),
        fingerprint_versions: BTreeSet::new(),
        categories: BTreeMap::new(),
        polarities: BTreeMap::new(),
        severities: BTreeMap::new(),
        attributions: BTreeMap::new(),
        verified_target_hints: BTreeSet::new(),
        human_only_target_hints: BTreeSet::new(),
        source_agents: BTreeMap::new(),
    };
    for signal in signals {
        if value.signal_ids.len() < MAX_SIGNALS_PER_SEED {
            value.signal_ids.push(signal.signal_id.clone());
        }
        value.extractor_versions.insert(signal.extractor_version);
        value.sanitizer_versions.insert(signal.sanitizer_version);
        value
            .fingerprint_versions
            .insert(signal.fingerprint_version);
        *value.categories.entry(signal.category.clone()).or_default() += 1;
        *value.polarities.entry(signal.polarity.clone()).or_default() += 1;
        *value.severities.entry(signal.severity.clone()).or_default() += 1;
        *value
            .attributions
            .entry(signal.attribution.clone())
            .or_default() += 1;
        value
            .verified_target_hints
            .extend(signal.verified_targets.iter().cloned());
        value
            .human_only_target_hints
            .extend(signal.human_targets.iter().cloned());
        if let Some(agent) = &signal.source_agent {
            *value.source_agents.entry(agent.clone()).or_default() += 1;
        }
    }
    value
}

fn hash_text(value: &str) -> String {
    Sha256::digest(value.as_bytes())
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}
