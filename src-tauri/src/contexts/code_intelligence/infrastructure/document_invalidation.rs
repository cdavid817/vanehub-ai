use std::collections::{HashSet, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

const MAX_PENDING_DOCUMENT_INVALIDATIONS: usize = 1_024;

#[derive(Default)]
struct InvalidationState {
    pending: VecDeque<(PathBuf, String)>,
    unique: HashSet<(PathBuf, String)>,
}

/// Non-blocking bridge from workspace mutations to the LSP actor side.
///
/// A semantic request drains its workspace's entries before preparing the document lease. Dropped
/// signals remain safe because the disk-authoritative lease path hashes the file before querying.
#[derive(Clone, Default)]
pub(crate) struct LspDocumentInvalidationQueue {
    state: Arc<Mutex<InvalidationState>>,
}

impl LspDocumentInvalidationQueue {
    pub(crate) fn publish(&self, workspace: &Path, relative_path: &str) {
        let Ok(mut state) = self.state.try_lock() else {
            return;
        };
        let mutation = (workspace.to_path_buf(), relative_path.replace('\\', "/"));
        if state.unique.contains(&mutation)
            || state.pending.len() >= MAX_PENDING_DOCUMENT_INVALIDATIONS
        {
            return;
        }
        state.unique.insert(mutation.clone());
        state.pending.push_back(mutation);
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) fn drain_workspace(&self, workspace: &Path) -> Vec<String> {
        let Ok(mut state) = self.state.lock() else {
            return Vec::new();
        };
        let mut drained = Vec::new();
        let mut retained = VecDeque::new();
        while let Some(mutation) = state.pending.pop_front() {
            state.unique.remove(&mutation);
            if mutation.0 == workspace {
                drained.push(mutation.1);
            } else {
                state.unique.insert(mutation.clone());
                retained.push_back(mutation);
            }
        }
        state.pending = retained;
        drained
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invalidations_are_normalized_coalesced_and_scoped() {
        let queue = LspDocumentInvalidationQueue::default();
        let left = Path::new("C:/workspace-a");
        let right = Path::new("C:/workspace-b");

        queue.publish(left, "src\\lib.rs");
        queue.publish(left, "src/lib.rs");
        queue.publish(right, "src/lib.rs");

        assert_eq!(queue.drain_workspace(left), vec!["src/lib.rs"]);
        assert_eq!(queue.drain_workspace(right), vec!["src/lib.rs"]);
    }
}
