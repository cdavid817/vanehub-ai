// The staleness half is consumed by task 5.2, which attaches the caveat to injected bodies.
// The age half is already used by the selection manifest.
#![allow(dead_code)]

use std::time::{Duration, SystemTime};

/// A memory younger than this carries no staleness caveat. A caveat on something written an hour
/// ago is noise, and noise here is expensive: it trains the model to skim past caveats generally,
/// including the ones that matter.
const STALENESS_THRESHOLD: Duration = Duration::from_secs(24 * 60 * 60);

pub(crate) const MEMORY_STALENESS_CAVEAT: &str =
    "This memory is a point-in-time observation, not live state. Claims about code behavior, file \
paths, or symbol names may be outdated -- verify against the current code before asserting them.";

/// Elapsed time in words.
///
/// Rendered rather than stamped because a raw timestamp requires date arithmetic to interpret, and
/// the interpretation is the part that has to happen for age to affect behavior at all.
pub(crate) fn render_memory_age(
    modified_at: Option<SystemTime>,
    now: SystemTime,
) -> Option<String> {
    let elapsed = elapsed_since(modified_at, now)?;
    let days = elapsed.as_secs() / (24 * 60 * 60);
    Some(match days {
        0 => "today".to_string(),
        1 => "yesterday".to_string(),
        2..=13 => format!("{days} days ago"),
        14..=59 => format!("{} weeks ago", days / 7),
        60..=364 => format!("{} months ago", days / 30),
        _ => format!("{} years ago", days / 365),
    })
}

/// The caveat, when the memory is old enough to warrant one.
pub(crate) fn memory_staleness_caveat(
    modified_at: Option<SystemTime>,
    now: SystemTime,
) -> Option<&'static str> {
    let elapsed = elapsed_since(modified_at, now)?;
    (elapsed >= STALENESS_THRESHOLD).then_some(MEMORY_STALENESS_CAVEAT)
}

/// `None` when the memory has no modification time, or when its time is in the future. A clock
/// skew or a copied file must not render as a negative age, and treating it as "unknown" is
/// honest where clamping to "today" would assert freshness this cannot know.
fn elapsed_since(modified_at: Option<SystemTime>, now: SystemTime) -> Option<Duration> {
    now.duration_since(modified_at?).ok()
}
