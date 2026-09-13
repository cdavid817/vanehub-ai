use crate::contexts::agent_runtime::application::AgentMemoryRef;
use crate::contexts::agent_runtime::domain::AgentMemoryReadContext;
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

/// Subjects retained before the least recently touched one is dropped.
///
/// This is also how a finished session's exclusions are reclaimed. There is deliberately no
/// explicit end-of-session hook: session lifecycle lives in the `sessions` context, and reaching
/// into this one from there would cross a boundary the architecture keeps closed. Nothing is
/// persisted, and a session that has ended is never consulted again, so the cap is sufficient.
const MAX_TRACKED_SUBJECTS: usize = 64;

/// Memories whose bodies have already been injected for one subject, keyed by immutable id and
/// holding the exact version shown: revision and content hash.
///
/// Version identity rather than a modification time: a corrected memory has a new revision and
/// hash and becomes eligible again, while an external edit that forgot to advance the revision
/// still changes the hash. Neither is something a timestamp can express reliably.
type SurfacedMemories = HashMap<String, (u64, String)>;

struct SurfacedStore {
    subjects: HashMap<String, SurfacedMemories>,
    /// Subject keys in touch order, oldest first. A plain Vec is enough at this size.
    order: Vec<String>,
}

fn store() -> &'static Mutex<SurfacedStore> {
    static STORE: OnceLock<Mutex<SurfacedStore>> = OnceLock::new();
    STORE.get_or_init(|| {
        Mutex::new(SurfacedStore {
            subjects: HashMap::new(),
            order: Vec::new(),
        })
    })
}

/// The partition one context's markers live in: the session plus the actual Agent/seat and the
/// frozen scope. Another seat in the same group session, or the same Agent under another
/// workspace or mode, is a different partition -- it inherits neither the suppression nor, more
/// importantly, anything that could read as authorization.
fn partition_key(context: &AgentMemoryReadContext) -> String {
    format!("{}\u{1e}{}", context.session_id, context.subject_key())
}

/// Candidates whose bodies this subject has not already been shown at this exact version.
///
/// Deduplication only: the refs arriving here have already passed the current eligibility
/// decision, and this never widens or narrows that. Filtering happens before the selector call
/// rather than after it, so the bounded selection budget is not spent on memories the caller is
/// about to discard.
pub(crate) fn unsurfaced_refs(
    context: &AgentMemoryReadContext,
    candidates: &[AgentMemoryRef],
) -> Vec<AgentMemoryRef> {
    let guard = store().lock();
    let Ok(guard) = guard else {
        // A poisoned lock means some earlier caller panicked. Degrading to "nothing was surfaced"
        // costs a repeat injection at worst, where propagating would fail the generation.
        return candidates.to_vec();
    };
    let Some(surfaced) = guard.subjects.get(&partition_key(context)) else {
        return candidates.to_vec();
    };
    candidates
        .iter()
        .filter(|memory| match surfaced.get(&memory.id) {
            Some((revision, hash)) => *revision != memory.revision || *hash != memory.content_hash,
            None => true,
        })
        .cloned()
        .collect()
}

/// Records that these memories' bodies reached the prompt for this subject, at these versions.
pub(crate) fn mark_surfaced(context: &AgentMemoryReadContext, memories: &[AgentMemoryRef]) {
    if memories.is_empty() {
        return;
    }
    let Ok(mut guard) = store().lock() else {
        return;
    };
    let key = partition_key(context);
    let entry = guard.subjects.entry(key.clone()).or_default();
    for memory in memories {
        entry.insert(
            memory.id.clone(),
            (memory.revision, memory.content_hash.clone()),
        );
    }
    touch(&mut guard, &key);
}

fn touch(guard: &mut SurfacedStore, key: &str) {
    guard.order.retain(|id| id != key);
    guard.order.push(key.to_string());
    while guard.order.len() > MAX_TRACKED_SUBJECTS {
        let evicted = guard.order.remove(0);
        guard.subjects.remove(&evicted);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contexts::agent_runtime::domain::AgentWorkspaceBinding;

    fn context(session: &str, agent: &str, seat: Option<&str>) -> AgentMemoryReadContext {
        AgentMemoryReadContext {
            agent_id: agent.to_string(),
            session_id: session.to_string(),
            generation_id: "generation".to_string(),
            seat_id: seat.map(str::to_string),
            workspace: AgentWorkspaceBinding::Absent,
            session_mode: "standard".to_string(),
            policy_revision: "policy".to_string(),
            read: true,
            global_allowed: true,
            workspace_allowed: None,
            maintenance_generation: 1,
            contract_version: 1,
            fingerprint: "fp".to_string(),
        }
    }

    fn memory(name: &str, revision: u64, hash: &str) -> AgentMemoryRef {
        AgentMemoryRef {
            id: format!("{name}.md"),
            revision,
            content_hash: hash.to_string(),
            authority_fingerprint: "authority".to_string(),
            name: name.to_string(),
            description: format!("About {name}"),
            memory_type: None,
            updated_at: None,
        }
    }

    fn names(memories: &[AgentMemoryRef]) -> Vec<&str> {
        memories.iter().map(|memory| memory.name.as_str()).collect()
    }

    #[test]
    fn a_memory_surfaced_earlier_in_the_session_is_not_offered_again() {
        let subject = context("session-not-offered-again", "onepiece", None);
        let pool = vec![memory("first", 1, "h1"), memory("second", 1, "h2")];

        mark_surfaced(&subject, &pool[..1]);

        assert_eq!(names(&unsurfaced_refs(&subject, &pool)), vec!["second"]);
    }

    #[test]
    fn a_new_revision_or_a_changed_hash_becomes_eligible_again() {
        // Its content is no longer the content the model was shown. Both an in-app correction
        // (new revision) and an external edit that kept the revision (new hash) count.
        let subject = context("session-corrected-again", "onepiece", None);
        let before = memory("npm-only", 1, "h1");
        mark_surfaced(&subject, std::slice::from_ref(&before));

        assert!(unsurfaced_refs(&subject, std::slice::from_ref(&before)).is_empty());
        assert_eq!(
            names(&unsurfaced_refs(&subject, &[memory("npm-only", 2, "h1")])),
            vec!["npm-only"]
        );
        assert_eq!(
            names(&unsurfaced_refs(&subject, &[memory("npm-only", 1, "h2")])),
            vec!["npm-only"]
        );
    }

    #[test]
    fn exclusions_do_not_leak_between_sessions() {
        let pool = vec![memory("shared", 1, "h")];
        mark_surfaced(&context("session-a", "onepiece", None), &pool);

        assert!(unsurfaced_refs(&context("session-a", "onepiece", None), &pool).is_empty());
        assert_eq!(
            names(&unsurfaced_refs(
                &context("session-b", "onepiece", None),
                &pool
            )),
            vec!["shared"]
        );
    }

    #[test]
    fn another_seat_in_the_same_session_does_not_inherit_the_suppression() {
        // MR-20: seat A being shown a record says nothing about seat B, which is a different
        // subject with its own eligibility decision and its own view of what it has seen.
        let pool = vec![memory("shared", 1, "h")];
        let seat_a = context("group-session", "onepiece", Some("seat-a"));
        let seat_b = context("group-session", "claude-code", Some("seat-b"));
        mark_surfaced(&seat_a, &pool);

        assert!(unsurfaced_refs(&seat_a, &pool).is_empty());
        assert_eq!(names(&unsurfaced_refs(&seat_b, &pool)), vec!["shared"]);
    }

    #[test]
    fn a_workspace_change_for_the_same_agent_is_a_different_partition() {
        let pool = vec![memory("shared", 1, "h")];
        let mut in_workspace = context("session", "onepiece", None);
        in_workspace.workspace = AgentWorkspaceBinding::Resolved("ws-1".to_string());
        in_workspace.workspace_allowed = Some("ws-1".to_string());
        mark_surfaced(&in_workspace, &pool);

        let mut elsewhere = in_workspace.clone();
        elsewhere.workspace = AgentWorkspaceBinding::Resolved("ws-2".to_string());
        elsewhere.workspace_allowed = Some("ws-2".to_string());
        assert_eq!(names(&unsurfaced_refs(&elsewhere, &pool)), vec!["shared"]);
    }

    #[test]
    fn the_store_reclaims_the_oldest_subjects_past_its_cap() {
        let pool = vec![memory("kept", 1, "h")];
        let oldest = context("session-evicted-0", "onepiece", None);
        for index in 0..=MAX_TRACKED_SUBJECTS {
            mark_surfaced(
                &context(&format!("session-evicted-{index}"), "onepiece", None),
                &pool,
            );
        }

        assert_eq!(names(&unsurfaced_refs(&oldest, &pool)), vec!["kept"]);
        let newest = context(
            &format!("session-evicted-{MAX_TRACKED_SUBJECTS}"),
            "onepiece",
            None,
        );
        assert!(unsurfaced_refs(&newest, &pool).is_empty());
    }
}
