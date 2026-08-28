//! The numbers a waterfall needs that a span does not carry.
//!
//! Depth, horizontal offset, and which spans decided the total duration are all properties of a
//! span's *position among the others*, so no producer can attach them and no view should have to
//! derive them — a view that did would be recomputing them per render against whatever subset it
//! happened to have loaded.
//!
//! The rule every function here follows: **a value that cannot be derived is absent, not zero.**
//! A running span has no duration, and reporting its elapsed-so-far as one makes it look finished.
//! A span whose producer never recorded an attempt number has no attempt, and defaulting it to 1
//! asserts a retry history nobody observed. Absent is a fact; a filled-in default is a claim.

use super::super::domain::{ExecutionSpan, ExecutionSpanKind, ExecutionTimeline};
use std::collections::{BTreeMap, BTreeSet};

/// How deep the tree may be reported.
///
/// Not a display choice. A malformed parent chain — two spans naming each other, or a cycle
/// through three — would otherwise walk forever, and the walk happens while assembling a response.
/// Past this depth the reported depth stops climbing, which renders as a flat run rather than a
/// hang.
pub(crate) const MAX_SPAN_DEPTH: u16 = 64;

/// Producers that record a retry attempt set this. Absent means nobody counted.
const ATTEMPT_ATTRIBUTE: &str = "vanehub.attempt";
/// Set on a span whose work was handed to another agent.
const DELEGATION_ATTRIBUTE: &str = "vanehub.delegation.target";

/// What the waterfall knows about one span beyond what the span says.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct SpanWaterfallMetadata {
    /// Distance from a root span. Zero for a root.
    pub(crate) depth: u16,
    /// Milliseconds from the run's start to this span's start.
    ///
    /// Absent when either timestamp is unreadable. A bar drawn at offset zero because a timestamp
    /// failed to parse would place work at the beginning of the run that did not happen there.
    pub(crate) start_offset_ms: Option<u64>,
    /// Duration of a span that finished. Absent while it is still running.
    pub(crate) completed_duration_ms: Option<u64>,
    /// Which attempt this was, when a producer counted. Absent when nobody did.
    pub(crate) attempt: Option<u32>,
    /// Whether this span handed its work to another agent.
    pub(crate) delegated: bool,
    /// Whether this span is on the chain that determined the run's duration.
    ///
    /// Only ever true for a run whose spans have all finished. A critical path through work that
    /// is still running is a prediction, and this type reports observations.
    pub(crate) critical_path: bool,
}

/// Derives the waterfall metadata for every span in a timeline, keyed by span id.
pub(crate) fn derive_waterfall(
    timeline: &ExecutionTimeline,
) -> BTreeMap<String, SpanWaterfallMetadata> {
    let parents = parent_map(timeline);
    let run_start = parse_ms(&timeline.run.started_at);
    let critical = critical_path(timeline, &parents);

    timeline
        .spans
        .iter()
        .map(|span| {
            let span_id = span.context.span_id.as_str().to_string();
            let started = parse_ms(&span.started_at);
            (
                span_id.clone(),
                SpanWaterfallMetadata {
                    depth: depth_of(&span_id, &parents),
                    start_offset_ms: match (run_start, started) {
                        // A span that started before its run is a clock disagreement, not a
                        // negative offset. Reporting it as absent says "cannot place this" rather
                        // than drawing it off the left edge.
                        (Some(run), Some(span)) if span >= run => Some((span - run) as u64),
                        _ => None,
                    },
                    completed_duration_ms: completed_duration(span),
                    attempt: attempt_of(span),
                    delegated: span.attributes.entries().contains_key(DELEGATION_ATTRIBUTE)
                        || classify_span_kind_of(span) == ExecutionSpanKind::Delegation,
                    critical_path: critical.contains(&span_id),
                },
            )
        })
        .collect()
}

fn classify_span_kind_of(span: &ExecutionSpan) -> ExecutionSpanKind {
    super::super::domain::classify_span_kind(&span.attributes)
}

fn parent_map(timeline: &ExecutionTimeline) -> BTreeMap<String, String> {
    timeline
        .spans
        .iter()
        .filter_map(|span| {
            span.parent_span_id.as_ref().map(|parent| {
                (
                    span.context.span_id.as_str().to_string(),
                    parent.as_str().to_string(),
                )
            })
        })
        .collect()
}

/// Walks to the root, bounded.
///
/// The bound is what makes this safe against a parent chain that loops. A cycle is a producer bug
/// rather than an expected state, but it arrives as data and must not be able to hang a response.
fn depth_of(span_id: &str, parents: &BTreeMap<String, String>) -> u16 {
    let mut depth = 0u16;
    let mut current = span_id;
    let mut visited = BTreeSet::from([span_id.to_string()]);
    while let Some(parent) = parents.get(current) {
        if depth >= MAX_SPAN_DEPTH || !visited.insert(parent.clone()) {
            break;
        }
        depth += 1;
        current = parent;
    }
    depth
}

/// A duration only for a span that ended.
///
/// The absence is the point. A view showing elapsed-so-far as a duration makes a running span
/// indistinguishable from one that finished in exactly that time, and the two mean opposite things
/// about whether the work is done.
fn completed_duration(span: &ExecutionSpan) -> Option<u64> {
    let ended = span.ended_at.as_deref()?;
    let start = parse_ms(&span.started_at)?;
    let end = parse_ms(ended)?;
    (end >= start).then(|| (end - start) as u64)
}

fn attempt_of(span: &ExecutionSpan) -> Option<u32> {
    match span.attributes.entries().get(ATTEMPT_ATTRIBUTE)? {
        super::super::domain::SafeAttributeValue::Integer(value) => u32::try_from(*value).ok(),
        // A producer that wrote the attempt as text still counted it. Refusing to read it would
        // discard an observation over a formatting choice.
        super::super::domain::SafeAttributeValue::String(value) => value.parse().ok(),
        _ => None,
    }
}

/// The chain of finished spans that determined how long the run took.
///
/// Empty unless every span in the run has ended. A critical path is a statement about which work
/// the total duration depended on, and that cannot be known while some of the work is still
/// happening — the span that has not finished may yet become the longest.
fn critical_path(
    timeline: &ExecutionTimeline,
    parents: &BTreeMap<String, String>,
) -> BTreeSet<String> {
    if timeline.spans.iter().any(|span| span.ended_at.is_none()) {
        return BTreeSet::new();
    }
    // The latest-ending span is the one the run waited on; its ancestors are what it waited behind.
    let Some(latest) = timeline
        .spans
        .iter()
        .filter_map(|span| {
            parse_ms(span.ended_at.as_deref()?)
                .map(|ended| (ended, span.context.span_id.as_str().to_string()))
        })
        .max_by_key(|(ended, _)| *ended)
        .map(|(_, span_id)| span_id)
    else {
        return BTreeSet::new();
    };

    let mut path = BTreeSet::from([latest.clone()]);
    let mut current = latest;
    // Bounded by the same depth ceiling, for the same reason: a cycle here would loop.
    for _ in 0..MAX_SPAN_DEPTH {
        let Some(parent) = parents.get(&current) else {
            break;
        };
        if !path.insert(parent.clone()) {
            break;
        }
        current = parent.clone();
    }
    path
}

/// Milliseconds since the epoch, or nothing.
///
/// Nothing rather than zero: a timestamp that failed to parse is one this run cannot be placed by,
/// and zero would place it at the epoch — fifty years before every other span.
fn parse_ms(timestamp: &str) -> Option<i64> {
    chrono::DateTime::parse_from_rfc3339(timestamp)
        .ok()
        .map(|value| value.timestamp_millis())
}
