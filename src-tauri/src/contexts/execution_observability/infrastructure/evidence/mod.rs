//! SQLite persistence for the evidence journal.
//!
//! Everything here is private to the context. The repository, the row shapes, the cursor encoding,
//! and the schema are reachable only through `ExecutionEvidenceApi`, so no consumer can issue a
//! query whose coverage nobody vouches for.
mod adapters;
mod cursor;
mod maintenance;
mod payload_row;
mod projection;
mod report_aggregate;
mod repository;
mod rows;
mod schema;
mod tokens;

pub(crate) use adapters::{
    DomainEvidenceRedactionValidator, RateLimitedEvidenceDiagnostics, SystemEvidenceClock,
    TauriEvidenceNoticePublisher, UuidEvidenceIdGenerator,
};
pub(crate) use repository::SqliteEvidenceRepository;
pub(crate) use schema::{apply_evidence_schema, repair_missing_evidence_schema};

#[cfg(test)]
mod report_aggregate_tests;
#[cfg(test)]
mod tests;
