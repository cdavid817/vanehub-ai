use std::fs;
use std::path::PathBuf;

use super::markdown_memory_repository::DERIVED_INDEX_FILE_NAME;
use crate::contexts::personalization::application::{
    DerivedIndexPort, PersonalizationApplicationError,
};
use crate::contexts::personalization::domain::{MemoryRecord, MemoryStatus};

type Result<T> = std::result::Result<T, PersonalizationApplicationError>;

/// The sibling an index rebuild writes before renaming it into place. Ends in `.tmp` so enumeration
/// classifies it transient rather than as a memory or a second index.
const TEMPORARY_INDEX_FILE_NAME: &str = "MEMORY.md.tmp";

/// `MEMORY.md`: a derived, bounded pointer list over active memories.
///
/// Always rebuilt from the authoritative records rather than edited in place. Incremental
/// maintenance is what let the old index drift from the directory — an index line pointing at a
/// deleted file, or a file with no line — and a full rebuild makes both states converge without a
/// special-case repair path.
#[derive(Clone)]
pub(crate) struct MarkdownDerivedIndex {
    root: PathBuf,
}

impl MarkdownDerivedIndex {
    pub(crate) fn new(root: PathBuf) -> Self {
        Self { root }
    }
}

impl DerivedIndexPort for MarkdownDerivedIndex {
    fn rebuild(&self, active: &[MemoryRecord]) -> Result<usize> {
        // Filtering here as well as at the call site is deliberate: this file is what a model reads
        // to learn what exists, so a candidate reaching it would be an unreviewed proposal
        // presented as an established fact.
        let mut included: Vec<&MemoryRecord> = active
            .iter()
            .filter(|record| matches!(record.status, MemoryStatus::Active))
            .collect();
        included.sort_by(|left, right| {
            right
                .updated_at
                .cmp(&left.updated_at)
                // Ties are routine — a migration writes a whole directory at one timestamp — and an
                // unstable order would reshuffle the file on every rebuild.
                .then_with(|| left.id.as_str().cmp(right.id.as_str()))
        });

        let mut lines = vec!["# Memory index".to_string(), String::new()];
        for record in &included {
            // Body content never appears here; the description is the hook that lets a reader
            // decide whether to open the memory itself.
            lines.push(format!(
                "- [{}]({}) — {}",
                record.name,
                record.file_name(),
                record.description
            ));
        }
        let rendered = format!("{}\n", lines.join("\n"));

        fs::create_dir_all(&self.root).map_err(|error| {
            PersonalizationApplicationError::Storage(format!(
                "memory directory is unavailable: {error}"
            ))
        })?;
        // Written to a sibling and renamed over the target rather than truncated in place. A crash
        // mid-write would otherwise leave a half-listed index that a model would read as the
        // complete set of what it knows, which is worse than an index one rebuild out of date.
        let destination = self.root.join(DERIVED_INDEX_FILE_NAME);
        let temporary = self.root.join(TEMPORARY_INDEX_FILE_NAME);
        fs::write(&temporary, rendered).map_err(|error| {
            PersonalizationApplicationError::Storage(format!(
                "memory index cannot be written: {error}"
            ))
        })?;
        fs::rename(&temporary, &destination).map_err(|error| {
            PersonalizationApplicationError::Storage(format!(
                "memory index cannot be replaced: {error}"
            ))
        })?;
        Ok(included.len())
    }
}
