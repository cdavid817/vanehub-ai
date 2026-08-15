use crate::contexts::agent_runtime::application::AgentMemory;
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use std::time::SystemTime;

/// Sessions retained before the least recently touched one is dropped.
///
/// This is also how a finished session's exclusions are reclaimed. There is deliberately no
/// explicit end-of-session hook: session lifecycle lives in the `sessions` context, and reaching
/// into this one from there would cross a boundary the architecture keeps closed. Nothing is
/// persisted, and a session that has ended is never consulted again, so the cap is sufficient.
const MAX_TRACKED_SESSIONS: usize = 64;

/// Memories whose bodies have already been injected in a session, with the modification time they
/// had when that happened.
///
/// Keyed on modification time rather than path alone so a memory the model has since corrected
/// becomes eligible again: its content is no longer the content the model was shown.
type SurfacedMemories = HashMap<String, SystemTime>;

struct SurfacedStore {
    sessions: HashMap<String, SurfacedMemories>,
    /// Session ids in touch order, oldest first. A plain Vec is enough at this size.
    order: Vec<String>,
}

fn store() -> &'static Mutex<SurfacedStore> {
    static STORE: OnceLock<Mutex<SurfacedStore>> = OnceLock::new();
    STORE.get_or_init(|| {
        Mutex::new(SurfacedStore {
            sessions: HashMap::new(),
            order: Vec::new(),
        })
    })
}

/// Candidates whose bodies this session has not already been shown.
///
/// Filtering happens before the selector call, not after it: filtering afterwards would spend the
/// bounded selection budget on memories the caller is about to discard.
pub(crate) fn unsurfaced_candidates(
    session_id: &str,
    candidates: &[AgentMemory],
) -> Vec<AgentMemory> {
    let guard = store().lock();
    let Ok(guard) = guard else {
        // A poisoned lock means some earlier caller panicked. Degrading to "nothing was surfaced"
        // costs a repeat injection at worst, where propagating would fail the generation.
        return candidates.to_vec();
    };
    let Some(surfaced) = guard.sessions.get(session_id) else {
        return candidates.to_vec();
    };
    candidates
        .iter()
        .filter(|memory| match surfaced.get(&memory.id) {
            Some(shown_at) => memory.modified_at != Some(*shown_at),
            None => true,
        })
        .cloned()
        .collect()
}

/// Records that these memories' bodies reached the prompt for this session.
pub(crate) fn mark_surfaced(session_id: &str, memories: &[AgentMemory]) {
    if memories.is_empty() {
        return;
    }
    let Ok(mut guard) = store().lock() else {
        return;
    };
    let entry = guard.sessions.entry(session_id.to_string()).or_default();
    for memory in memories {
        // A memory with no modification time cannot be re-eligibility-checked, so it is not
        // tracked at all rather than being excluded forever on a timestamp we never had.
        if let Some(modified_at) = memory.modified_at {
            entry.insert(memory.id.clone(), modified_at);
        }
    }
    touch(&mut guard, session_id);
}

fn touch(guard: &mut SurfacedStore, session_id: &str) {
    guard.order.retain(|id| id != session_id);
    guard.order.push(session_id.to_string());
    while guard.order.len() > MAX_TRACKED_SESSIONS {
        let evicted = guard.order.remove(0);
        guard.sessions.remove(&evicted);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contexts::agent_runtime::application::MemorySource;
    use std::time::Duration;

    fn memory(name: &str, modified_at: Option<SystemTime>) -> AgentMemory {
        AgentMemory {
            id: format!("{name}.md"),
            agent_id: "onepiece".to_string(),
            folder: None,
            name: name.to_string(),
            description: format!("About {name}"),
            memory_type: None,
            content: "Body.".to_string(),
            source: MemorySource::Explicit,
            created_at: "2026-01-01T00:00:00Z".to_string(),
            modified_at,
        }
    }

    fn at(seconds: u64) -> Option<SystemTime> {
        Some(SystemTime::UNIX_EPOCH + Duration::from_secs(seconds))
    }

    fn names(memories: &[AgentMemory]) -> Vec<&str> {
        memories.iter().map(|memory| memory.name.as_str()).collect()
    }

    #[test]
    fn a_memory_surfaced_earlier_in_the_session_is_not_offered_again() {
        // The bound is small, so re-offering something the model has already been shown spends the
        // budget on nothing.
        let session = "session-not-offered-again";
        let pool = vec![memory("first", at(10)), memory("second", at(20))];

        mark_surfaced(session, &pool[..1]);

        assert_eq!(
            names(&unsurfaced_candidates(session, &pool)),
            vec!["second"]
        );
    }

    #[test]
    fn a_corrected_memory_becomes_eligible_again() {
        // Its content is no longer the content the model was shown, so excluding it would hide the
        // correction for the rest of the session — the opposite of what correcting it was for.
        let session = "session-corrected-again";
        let before = memory("npm-only", at(10));
        mark_surfaced(session, std::slice::from_ref(&before));
        let after = memory("npm-only", at(99));

        assert!(unsurfaced_candidates(session, std::slice::from_ref(&before)).is_empty());
        assert_eq!(
            names(&unsurfaced_candidates(
                session,
                std::slice::from_ref(&after)
            )),
            vec!["npm-only"]
        );
    }

    #[test]
    fn exclusions_do_not_leak_between_sessions() {
        // A new session must see everything again; carrying exclusions across sessions would make
        // old memories progressively harder to surface, which is the failure this change removes.
        let pool = vec![memory("shared", at(10))];
        mark_surfaced("session-a", &pool);

        assert!(unsurfaced_candidates("session-a", &pool).is_empty());
        assert_eq!(
            names(&unsurfaced_candidates("session-b", &pool)),
            vec!["shared"]
        );
    }

    #[test]
    fn a_memory_without_a_modification_time_is_never_excluded() {
        // Excluding it would be permanent: with no timestamp there is no later value that could
        // make it eligible again.
        let session = "session-no-timestamp";
        let pool = vec![memory("undated", None)];

        mark_surfaced(session, &pool);

        assert_eq!(
            names(&unsurfaced_candidates(session, &pool)),
            vec!["undated"]
        );
    }

    #[test]
    fn the_store_reclaims_the_oldest_sessions_past_its_cap() {
        let pool = vec![memory("kept", at(10))];
        let oldest = "session-evicted-0";
        for index in 0..=MAX_TRACKED_SESSIONS {
            mark_surfaced(&format!("session-evicted-{index}"), &pool);
        }

        // The oldest session's exclusions are gone, so it would see the memory again — which is
        // exactly the behavior a finished session should decay into.
        assert_eq!(names(&unsurfaced_candidates(oldest, &pool)), vec!["kept"]);
        let newest = format!("session-evicted-{MAX_TRACKED_SESSIONS}");
        assert!(unsurfaced_candidates(&newest, &pool).is_empty());
    }
}
