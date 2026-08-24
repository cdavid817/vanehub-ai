use uuid::Uuid;

use crate::contexts::personalization::application::MemoryIdGeneratorPort;
use crate::contexts::personalization::domain::MemoryId;

/// Allocates memory ids from random UUIDs.
///
/// Random rather than time-ordered on purpose: an id becomes a filename, and a sortable id would
/// leak creation order to anyone listing the directory. Ordering is what `updated_at` and the
/// projection's keyset index are for.
#[derive(Debug, Default, Clone, Copy)]
pub(crate) struct UuidMemoryIdGenerator;

impl MemoryIdGeneratorPort for UuidMemoryIdGenerator {
    fn generate(&self) -> MemoryId {
        // Infallible by construction rather than by `expect`: the domain owns the guarantee that a
        // rendered UUID satisfies the id rules, so this call site has no failure to justify.
        MemoryId::from_uuid(Uuid::new_v4())
    }
}
