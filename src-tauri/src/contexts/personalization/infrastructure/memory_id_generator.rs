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
        // A v4 UUID is 36 characters of hex and hyphens, which `MemoryId` accepts by construction;
        // there is no input here that could fail its validation.
        MemoryId::parse(&Uuid::new_v4().to_string()).expect("a v4 UUID is always a valid memory id")
    }
}
