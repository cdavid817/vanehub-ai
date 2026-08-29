//! What the waterfall derives, and what it refuses to derive.
//!
//! Half of these tests assert an *absence*. That is the subject: every value here is a number a
//! view will draw a bar with, and a number that was defaulted rather than measured produces a bar
//! in a place nothing happened — which is indistinguishable from a bar in a place something did.

use super::super::domain::{
    CapturePolicy, ExecutionContext, ExecutionFidelity, ExecutionRun, ExecutionRunId,
    ExecutionSource, ExecutionSpan, ExecutionStatus, ExecutionTimeline, SafeAttributeValue,
    SafeAttributes, SpanId, TraceId,
};
use super::waterfall::{derive_waterfall, MAX_SPAN_DEPTH};

const RUN: &str = "0f9ba1a2-3c4d-4e5f-8a9b-0c1d2e3f4a5b";
const TRACE: &str = "4bf92f3577b34da6a3ce929d0e0e4736";

fn context(span_id: &str) -> ExecutionContext {
    ExecutionContext {
        run_id: ExecutionRunId::parse(RUN).expect("run id"),
        trace_id: TraceId::parse(TRACE).expect("trace id"),
        span_id: SpanId::parse(span_id).expect("span id"),
        capture_policy: CapturePolicy::MetadataOnly,
        sampling_per_million: 1_000_000,
        mcp_relay_enabled: false,
    }
}

/// Span ids are 16 hex characters and may not be all zero — that is the OpenTelemetry sentinel
/// for "no span", so the domain refuses it. Offsetting keeps index 0 a usable fixture id.
fn span_id(index: usize) -> String {
    format!("{:016x}", index + 0xa0)
}

struct SpanSpec {
    index: usize,
    parent: Option<usize>,
    started_at: &'static str,
    ended_at: Option<&'static str>,
    attributes: Vec<(&'static str, SafeAttributeValue)>,
}

fn spec(
    index: usize,
    parent: Option<usize>,
    started_at: &'static str,
    ended_at: Option<&'static str>,
) -> SpanSpec {
    SpanSpec {
        index,
        parent,
        started_at,
        ended_at,
        attributes: Vec::new(),
    }
}

fn timeline(run_started_at: &str, specs: Vec<SpanSpec>) -> ExecutionTimeline {
    ExecutionTimeline {
        run: ExecutionRun {
            context: context(&span_id(0)),
            source: ExecutionSource::Desktop,
            status: ExecutionStatus::Running,
            started_at: run_started_at.to_string(),
            ended_at: None,
            error_classification: None,
            session_id: None,
            user_message_id: None,
            assistant_message_id: None,
            operation_id: None,
            agent_id: None,
            provider_session_id: None,
            attributes: SafeAttributes::default(),
            links: Vec::new(),
        },
        spans: specs
            .into_iter()
            .map(|spec| ExecutionSpan {
                context: context(&span_id(spec.index)),
                parent_span_id: spec
                    .parent
                    .map(|parent| SpanId::parse(span_id(parent)).expect("parent id")),
                name: format!("span-{}", spec.index),
                status: ExecutionStatus::Running,
                fidelity: ExecutionFidelity::Native,
                started_at: spec.started_at.to_string(),
                ended_at: spec.ended_at.map(str::to_string),
                error_classification: None,
                attributes: SafeAttributes::try_from_entries(
                    spec.attributes
                        .into_iter()
                        .map(|(key, value)| (key.to_string(), value)),
                )
                .expect("attributes"),
                links: Vec::new(),
            })
            .collect(),
        events: Vec::new(),
    }
}

fn at(second: u32) -> &'static str {
    // Leaked so the fixtures can hold `&'static str` without every caller owning a String. The
    // count is fixed and tiny; this is a test.
    Box::leak(format!("2026-08-25T10:00:{second:02}Z").into_boxed_str())
}

/// Depth counts ancestors, and a root has none.
#[test]
fn depth_counts_the_ancestors_a_span_actually_has() {
    let derived = derive_waterfall(&timeline(
        at(0),
        vec![
            spec(1, None, at(0), None),
            spec(2, Some(1), at(1), None),
            spec(3, Some(2), at(2), None),
        ],
    ));

    assert_eq!(derived[&span_id(1)].depth, 0);
    assert_eq!(derived[&span_id(2)].depth, 1);
    assert_eq!(derived[&span_id(3)].depth, 2);
}

/// A parent chain that loops stops rather than walking forever.
///
/// A cycle is a producer bug, but it arrives as data — and the walk happens while assembling a
/// response, so an unbounded one would hang the command rather than return something wrong.
#[test]
fn a_cyclic_parent_chain_stops_instead_of_hanging() {
    let mut cyclic = timeline(
        at(0),
        vec![spec(1, Some(2), at(0), None), spec(2, Some(1), at(1), None)],
    );
    // Both spans name each other; neither is a root.
    cyclic.spans[0].parent_span_id = Some(SpanId::parse(span_id(2)).expect("id"));
    cyclic.spans[1].parent_span_id = Some(SpanId::parse(span_id(1)).expect("id"));

    let derived = derive_waterfall(&cyclic);

    assert!(derived[&span_id(1)].depth <= MAX_SPAN_DEPTH);
    assert!(derived[&span_id(2)].depth <= MAX_SPAN_DEPTH);
}

/// The offset is measured from the run's start, so bars line up against one timeline.
#[test]
fn the_start_offset_is_measured_from_the_run() {
    let derived = derive_waterfall(&timeline(
        at(0),
        vec![spec(1, None, at(0), None), spec(2, None, at(5), None)],
    ));

    assert_eq!(derived[&span_id(1)].start_offset_ms, Some(0));
    assert_eq!(derived[&span_id(2)].start_offset_ms, Some(5_000));
}

/// A span that started before its run has no offset rather than a negative one.
///
/// That is a clock disagreement, not work that happened before the run began. Reporting it as an
/// offset would draw the bar off the left edge of a chart whose origin is the run's start.
#[test]
fn a_span_that_predates_its_run_has_no_offset() {
    let derived = derive_waterfall(&timeline(at(10), vec![spec(1, None, at(3), None)]));

    assert_eq!(derived[&span_id(1)].start_offset_ms, None);
}

/// An unreadable timestamp yields no offset, not offset zero.
#[test]
fn an_unparseable_timestamp_yields_no_offset() {
    let derived = derive_waterfall(&timeline(
        at(0),
        vec![spec(1, None, "not a timestamp", None)],
    ));

    // Zero would place the span at the very start of the run — the one position that reads as a
    // definite claim about when it happened.
    assert_eq!(derived[&span_id(1)].start_offset_ms, None);
}

/// A running span has no duration.
///
/// Elapsed-so-far would make it indistinguishable from a span that finished in exactly that time,
/// and those two mean opposite things about whether the work is done.
#[test]
fn a_running_span_has_no_completed_duration() {
    let derived = derive_waterfall(&timeline(
        at(0),
        vec![
            spec(1, None, at(0), None),
            spec(2, None, at(0), Some(at(4))),
        ],
    ));

    assert_eq!(derived[&span_id(1)].completed_duration_ms, None);
    assert_eq!(derived[&span_id(2)].completed_duration_ms, Some(4_000));
}

/// A span that ended before it started has no duration either.
#[test]
fn a_span_that_ended_before_it_started_has_no_duration() {
    let derived = derive_waterfall(&timeline(at(0), vec![spec(1, None, at(9), Some(at(3)))]));

    assert_eq!(derived[&span_id(1)].completed_duration_ms, None);
}

/// An attempt is reported only when a producer counted one.
#[test]
fn an_attempt_is_reported_only_when_a_producer_counted_it() {
    let mut counted = spec(1, None, at(0), None);
    counted.attributes = vec![("vanehub.attempt", SafeAttributeValue::Integer(3))];
    let derived = derive_waterfall(&timeline(at(0), vec![counted, spec(2, None, at(0), None)]));

    assert_eq!(derived[&span_id(1)].attempt, Some(3));
    // Not `Some(1)`. Defaulting would assert a retry history nobody observed, and a reader
    // filtering for retries would find every span in the run.
    assert_eq!(derived[&span_id(2)].attempt, None);
}

/// An attempt written as text is still an attempt.
#[test]
fn an_attempt_recorded_as_text_is_still_read() {
    let mut textual = spec(1, None, at(0), None);
    textual.attributes = vec![(
        "vanehub.attempt",
        SafeAttributeValue::String("2".to_string()),
    )];

    let derived = derive_waterfall(&timeline(at(0), vec![textual]));

    // Discarding it over a formatting choice would lose an observation the producer made.
    assert_eq!(derived[&span_id(1)].attempt, Some(2));
}

/// Delegation is recognised from the attribute and from the classified kind.
#[test]
fn a_delegated_span_is_marked_from_what_it_carries() {
    let mut delegated = spec(1, None, at(0), None);
    delegated.attributes = vec![(
        "vanehub.delegation.target",
        SafeAttributeValue::String("agent-2".to_string()),
    )];

    let derived = derive_waterfall(&timeline(
        at(0),
        vec![delegated, spec(2, None, at(0), None)],
    ));

    assert!(derived[&span_id(1)].delegated);
    assert!(!derived[&span_id(2)].delegated);
}

/// The critical path is the chain the run's duration waited on.
#[test]
fn the_critical_path_is_the_chain_behind_the_latest_ending_span() {
    let derived = derive_waterfall(&timeline(
        at(0),
        vec![
            spec(1, None, at(0), Some(at(9))),
            spec(2, Some(1), at(1), Some(at(9))),
            // Finished early, so nothing waited on it.
            spec(3, Some(1), at(1), Some(at(2))),
        ],
    ));

    assert!(derived[&span_id(2)].critical_path, "the latest-ending span");
    assert!(derived[&span_id(1)].critical_path, "its parent");
    assert!(
        !derived[&span_id(3)].critical_path,
        "a sibling that finished early"
    );
}

/// No critical path is reported while any span is still running.
///
/// It is a statement about which work the total duration depended on, and that cannot be known
/// while some of the work is still happening — the unfinished span may yet become the longest.
#[test]
fn a_run_with_an_unfinished_span_reports_no_critical_path() {
    let derived = derive_waterfall(&timeline(
        at(0),
        vec![
            spec(1, None, at(0), Some(at(9))),
            spec(2, Some(1), at(1), None),
        ],
    ));

    assert!(!derived[&span_id(1)].critical_path);
    assert!(!derived[&span_id(2)].critical_path);
}

/// An empty timeline derives nothing and does not panic.
#[test]
fn an_empty_timeline_derives_nothing() {
    let derived = derive_waterfall(&timeline(at(0), Vec::new()));

    assert!(derived.is_empty());
}
