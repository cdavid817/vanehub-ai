use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use chrono::{DateTime, TimeZone, Utc};

use super::error::PersonalizationApplicationError;
use super::manage_memory::MemoryApplicationService;
use super::models::{CreateMemoryInput, DeleteMemoryOutcome, ResetCounts, UpdateMemoryPatch};
use super::ports::{
    ClockPort, DerivedIndexPort, MemoryMaintenanceRepository, MemoryProjectionPort,
    MemoryRepository, RetrievalIndexPort,
};
use crate::contexts::personalization::domain::{
    MaintenancePhase, MemoryAudience, MemoryId, MemoryPage, MemoryProvenance, MemoryQuery,
    MemoryRecord, MemoryScope, MemoryScopeFilter, MemorySensitivity, MemorySource, MemoryStatus,
    MemoryType, OwnedEntryClassification, ReconcileMemoryOutcome, ResetConfirmationToken,
    ResetMemoryOutcome, ResetMemoryRequest, StorageEntry, RESET_CONFIRMATION_PHRASE,
};

type Result<T> = std::result::Result<T, PersonalizationApplicationError>;

fn now() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 8, 24, 9, 0, 0).unwrap()
}

fn memory_id(index: usize) -> MemoryId {
    MemoryId::parse(&format!("01K2MEM{index:019}")).expect("memory id")
}

fn record(index: usize, status: MemoryStatus) -> MemoryRecord {
    MemoryRecord {
        id: memory_id(index),
        name: format!("Memory {index}"),
        description: String::new(),
        memory_type: MemoryType::Project,
        content: format!("body {index}"),
        scope: MemoryScope::Global,
        audience: MemoryAudience::AllAgents,
        status,
        source: MemorySource::ExplicitUser,
        provenance: MemoryProvenance::default(),
        sensitivity: MemorySensitivity::Normal,
        revision: 1,
        created_at: now(),
        updated_at: now(),
        verified_at: None,
        last_used_at: None,
        use_count: 0,
    }
}

#[derive(Default)]
struct FakeFiles {
    records: Mutex<Vec<MemoryRecord>>,
    next_index: AtomicUsize,
}

impl MemoryRepository for FakeFiles {
    fn get(&self, id: &MemoryId) -> Result<Option<MemoryRecord>> {
        Ok(self
            .records
            .lock()
            .expect("records")
            .iter()
            .find(|record| &record.id == id)
            .cloned())
    }

    fn create(&self, input: CreateMemoryInput, at: DateTime<Utc>) -> Result<MemoryRecord> {
        let index = self.next_index.fetch_add(1, Ordering::SeqCst);
        let id = memory_id(index);
        self.create_with_id(&id, input, at, at)
    }

    fn create_with_id(
        &self,
        id: &MemoryId,
        input: CreateMemoryInput,
        created_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
    ) -> Result<MemoryRecord> {
        let mut created = record(0, input.status);
        created.id = id.clone();
        created.name = input.name;
        created.content = input.content;
        created.scope = input.scope;
        created.audience = input.audience;
        created.source = input.source;
        created.created_at = created_at;
        created.updated_at = updated_at;
        self.records.lock().expect("records").push(created.clone());
        Ok(created)
    }

    fn update(
        &self,
        id: &MemoryId,
        expected_revision: u64,
        patch: UpdateMemoryPatch,
        at: DateTime<Utc>,
    ) -> Result<MemoryRecord> {
        let mut records = self.records.lock().expect("records");
        let stored = records
            .iter_mut()
            .find(|record| &record.id == id)
            .ok_or(PersonalizationApplicationError::NotFound)?;
        if stored.revision != expected_revision {
            return Err(PersonalizationApplicationError::RevisionConflict(
                crate::contexts::personalization::domain::RevisionConflict {
                    expected: expected_revision,
                    current: stored.revision,
                },
            ));
        }
        if let Some(status) = patch.status {
            stored.status = status;
        }
        if let Some(content) = patch.content {
            stored.content = content;
        }
        stored.revision += 1;
        stored.updated_at = at;
        Ok(stored.clone())
    }

    fn delete(&self, id: &MemoryId, _expected: Option<u64>) -> Result<DeleteMemoryOutcome> {
        let mut records = self.records.lock().expect("records");
        let before = records.len();
        records.retain(|record| &record.id != id);
        Ok(DeleteMemoryOutcome {
            deleted_file: records.len() < before,
            ..DeleteMemoryOutcome::default()
        })
    }
}

impl MemoryMaintenanceRepository for FakeFiles {
    fn enumerate_owned_entries(&self) -> Result<Vec<StorageEntry>> {
        Ok(self
            .records
            .lock()
            .expect("records")
            .iter()
            .map(|record| StorageEntry {
                file_name: record.file_name(),
                classification: OwnedEntryClassification::ValidV2,
                memory_id: Some(record.id.clone()),
            })
            .collect())
    }

    fn count_for_reset(
        &self,
        _scope: &MemoryScopeFilter,
        _statuses: &[MemoryStatus],
    ) -> Result<ResetCounts> {
        Ok(ResetCounts {
            matched: self.records.lock().expect("records").len(),
            ..ResetCounts::default()
        })
    }

    fn reset(
        &self,
        _request: &ResetMemoryRequest,
        _at: DateTime<Utc>,
    ) -> Result<ResetMemoryOutcome> {
        let mut records = self.records.lock().expect("records");
        let removed = records.len();
        records.clear();
        Ok(ResetMemoryOutcome {
            matched: removed,
            deleted_files: removed,
            ..ResetMemoryOutcome::default()
        })
    }

    fn reconcile(&self, _at: DateTime<Utc>) -> Result<ReconcileMemoryOutcome> {
        Ok(ReconcileMemoryOutcome::default())
    }
}

#[derive(Default)]
struct FakeProjection {
    ids: Mutex<Vec<MemoryId>>,
    fail_upsert: AtomicBool,
}

impl MemoryProjectionPort for FakeProjection {
    fn upsert(&self, record: &MemoryRecord, _hash: &str) -> Result<()> {
        if self.fail_upsert.load(Ordering::SeqCst) {
            return Err(PersonalizationApplicationError::Storage("disk full".into()));
        }
        let mut ids = self.ids.lock().expect("ids");
        if !ids.contains(&record.id) {
            ids.push(record.id.clone());
        }
        Ok(())
    }

    fn remove(&self, id: &MemoryId) -> Result<bool> {
        let mut ids = self.ids.lock().expect("ids");
        let before = ids.len();
        ids.retain(|stored| stored != id);
        Ok(ids.len() < before)
    }

    fn list_page(&self, _query: &MemoryQuery) -> Result<MemoryPage> {
        Ok(MemoryPage::empty())
    }

    fn count_for_reset(
        &self,
        _scope: &MemoryScopeFilter,
        _statuses: &[MemoryStatus],
    ) -> Result<ResetCounts> {
        Ok(ResetCounts::default())
    }

    fn projected_ids(&self) -> Result<Vec<MemoryId>> {
        Ok(self.ids.lock().expect("ids").clone())
    }

    fn clear(&self) -> Result<usize> {
        let mut ids = self.ids.lock().expect("ids");
        let count = ids.len();
        ids.clear();
        Ok(count)
    }
}

#[derive(Default)]
struct FakeRetrievalIndex {
    ids: Mutex<Vec<MemoryId>>,
    fail_revoke: AtomicBool,
}

impl RetrievalIndexPort for FakeRetrievalIndex {
    fn upsert(&self, record: &MemoryRecord) -> Result<()> {
        let mut ids = self.ids.lock().expect("ids");
        if !ids.contains(&record.id) {
            ids.push(record.id.clone());
        }
        Ok(())
    }

    fn revoke(&self, id: &MemoryId) -> Result<()> {
        if self.fail_revoke.load(Ordering::SeqCst) {
            return Err(PersonalizationApplicationError::Storage(
                "index offline".into(),
            ));
        }
        self.ids.lock().expect("ids").retain(|stored| stored != id);
        Ok(())
    }

    fn revoke_all(&self, ids: &[MemoryId]) -> Result<usize> {
        for id in ids {
            self.revoke(id)?;
        }
        Ok(ids.len())
    }

    fn indexed_ids(&self) -> Result<Vec<MemoryId>> {
        Ok(self.ids.lock().expect("ids").clone())
    }
}

#[derive(Default)]
struct FakeDerivedIndex {
    last_rebuild: Mutex<Vec<MemoryId>>,
    rebuild_count: AtomicUsize,
}

impl DerivedIndexPort for FakeDerivedIndex {
    fn rebuild(&self, active: &[MemoryRecord]) -> Result<usize> {
        self.rebuild_count.fetch_add(1, Ordering::SeqCst);
        let included: Vec<MemoryId> = active
            .iter()
            .filter(|record| matches!(record.status, MemoryStatus::Active))
            .map(|record| record.id.clone())
            .collect();
        let count = included.len();
        *self.last_rebuild.lock().expect("rebuild") = included;
        Ok(count)
    }
}

struct FixedClock;

impl ClockPort for FixedClock {
    fn now(&self) -> DateTime<Utc> {
        now()
    }
}

struct Fixture {
    service: MemoryApplicationService,
    files: Arc<FakeFiles>,
    projection: Arc<FakeProjection>,
    retrieval: Arc<FakeRetrievalIndex>,
    derived: Arc<FakeDerivedIndex>,
}

fn fixture() -> Fixture {
    let files = Arc::new(FakeFiles::default());
    let projection = Arc::new(FakeProjection::default());
    let retrieval = Arc::new(FakeRetrievalIndex::default());
    let derived = Arc::new(FakeDerivedIndex::default());
    let service = MemoryApplicationService::new(
        files.clone(),
        files.clone(),
        projection.clone(),
        derived.clone(),
        retrieval.clone(),
        Arc::new(FixedClock),
    );
    Fixture {
        service,
        files,
        projection,
        retrieval,
        derived,
    }
}

fn input(name: &str, status: MemoryStatus) -> CreateMemoryInput {
    CreateMemoryInput {
        name: name.to_string(),
        description: String::new(),
        memory_type: MemoryType::Project,
        content: "body".to_string(),
        scope: MemoryScope::Global,
        audience: MemoryAudience::AllAgents,
        status,
        source: MemorySource::ExplicitUser,
        provenance: MemoryProvenance::default(),
        sensitivity: MemorySensitivity::Normal,
    }
}

fn reset_request() -> ResetMemoryRequest {
    ResetMemoryRequest {
        scope: MemoryScopeFilter::Any,
        statuses: Vec::new(),
        token: ResetConfirmationToken {
            value: "tok_01K2ABCDEF".to_string(),
            issued_at: now(),
            scope: MemoryScopeFilter::Any,
            statuses: Vec::new(),
        },
        typed_phrase: RESET_CONFIRMATION_PHRASE.to_string(),
    }
}

#[test]
fn creating_an_active_memory_publishes_it_to_every_derived_view() {
    let fixture = fixture();
    let created = fixture
        .service
        .create(input("Use npm", MemoryStatus::Active))
        .expect("create");

    assert!(!created.requires_repair());
    assert_eq!(
        fixture.projection.projected_ids().expect("ids"),
        vec![created.record.id.clone()]
    );
    assert_eq!(
        fixture.retrieval.indexed_ids().expect("ids"),
        vec![created.record.id.clone()]
    );
    assert_eq!(
        *fixture.derived.last_rebuild.lock().expect("rebuild"),
        vec![created.record.id]
    );
}

#[test]
fn a_candidate_reaches_the_projection_but_never_retrieval_or_the_index() {
    // A candidate in the retrieval index or in `MEMORY.md` is an unreviewed proposal presented to
    // the model as an established fact. It still needs a projection row so the review queue can
    // page over it.
    let fixture = fixture();
    let created = fixture
        .service
        .create(input("Proposed", MemoryStatus::Candidate))
        .expect("create");

    assert_eq!(
        fixture.projection.projected_ids().expect("ids"),
        vec![created.record.id]
    );
    assert!(fixture.retrieval.indexed_ids().expect("ids").is_empty());
    assert!(fixture
        .derived
        .last_rebuild
        .lock()
        .expect("rebuild")
        .is_empty());
}

#[test]
fn a_failed_projection_write_reports_repair_without_claiming_the_save_failed() {
    // The authoritative file is already written by this point. Returning an error would tell the
    // user their memory was lost, which is false, and would invite a retry that creates a second
    // copy.
    let fixture = fixture();
    fixture.projection.fail_upsert.store(true, Ordering::SeqCst);

    let created = fixture
        .service
        .create(input("Use npm", MemoryStatus::Active))
        .expect("the create itself must succeed");

    assert!(created.requires_repair());
    assert_eq!(
        created.failures[0].phase,
        MaintenancePhase::SqliteProjection
    );
    assert_eq!(
        fixture.files.records.lock().expect("records").len(),
        1,
        "the authoritative record is present"
    );
}

#[test]
fn archiving_a_memory_revokes_its_retrieval_entry_and_index_line() {
    let fixture = fixture();
    let created = fixture
        .service
        .create(input("Use npm", MemoryStatus::Active))
        .expect("create");
    assert_eq!(fixture.retrieval.indexed_ids().expect("ids").len(), 1);

    fixture
        .service
        .update(
            &created.record.id,
            1,
            UpdateMemoryPatch {
                status: Some(MemoryStatus::Archived),
                ..UpdateMemoryPatch::default()
            },
        )
        .expect("archive");

    assert!(
        fixture.retrieval.indexed_ids().expect("ids").is_empty(),
        "an archived memory must stop being recallable"
    );
    assert!(fixture
        .derived
        .last_rebuild
        .lock()
        .expect("rebuild")
        .is_empty());
    assert_eq!(
        fixture.projection.projected_ids().expect("ids").len(),
        1,
        "the record stays manageable after archiving"
    );
}

#[test]
fn deleting_removes_the_projection_row_the_retrieval_entry_and_the_index_line() {
    let fixture = fixture();
    let created = fixture
        .service
        .create(input("Use npm", MemoryStatus::Active))
        .expect("create");

    let outcome = fixture
        .service
        .delete(&created.record.id, Some(1))
        .expect("delete");
    assert!(outcome.deleted_file);
    assert!(outcome.deleted_projection_row);
    assert!(outcome.revoked_retrieval_entry);
    assert!(outcome.removed_index_line);
    assert!(!outcome.requires_repair());
    assert!(fixture.projection.projected_ids().expect("ids").is_empty());
    assert!(fixture.retrieval.indexed_ids().expect("ids").is_empty());
}

#[test]
fn a_failed_retrieval_revocation_sets_repair_required_rather_than_reporting_success() {
    // The authoritative record is gone, so the memory is already excluded by eligibility. What is
    // not yet true is that the derived index agrees, and reporting a clean delete would hide that.
    let fixture = fixture();
    let created = fixture
        .service
        .create(input("Use npm", MemoryStatus::Active))
        .expect("create");
    fixture.retrieval.fail_revoke.store(true, Ordering::SeqCst);

    let outcome = fixture
        .service
        .delete(&created.record.id, Some(1))
        .expect("delete");
    assert!(outcome.deleted_file);
    assert!(!outcome.revoked_retrieval_entry);
    assert!(outcome.requires_repair());
    assert_eq!(outcome.failures[0].phase, MaintenancePhase::RetrievalIndex);
}

#[test]
fn deleting_an_absent_memory_still_sweeps_an_orphaned_derived_entry() {
    // A projection row with no file behind it is exactly the state that keeps a deleted memory
    // recallable, so the delete path sweeps it even when there was no file to remove.
    let fixture = fixture();
    let orphan = memory_id(99);
    fixture
        .projection
        .upsert(&record(99, MemoryStatus::Active), "hash")
        .expect("seed orphan");
    fixture
        .retrieval
        .upsert(&record(99, MemoryStatus::Active))
        .expect("seed orphan");

    let outcome = fixture.service.delete(&orphan, None).expect("delete");
    assert!(!outcome.deleted_file);
    assert!(outcome.deleted_projection_row);
    assert!(outcome.revoked_retrieval_entry);
    assert!(fixture.projection.projected_ids().expect("ids").is_empty());
}

#[test]
fn a_reset_removes_derived_state_for_everything_it_deleted() {
    let fixture = fixture();
    for index in 0..3 {
        fixture
            .service
            .create(input(&format!("Memory {index}"), MemoryStatus::Active))
            .expect("create");
    }
    assert_eq!(fixture.projection.projected_ids().expect("ids").len(), 3);

    let outcome = fixture.service.reset(&reset_request()).expect("reset");
    assert_eq!(outcome.deleted_files, 3);
    assert_eq!(outcome.deleted_projection_rows, 3);
    assert_eq!(outcome.revoked_retrieval_entries, 3);
    assert!(!outcome.requires_repair());
    assert!(fixture.projection.projected_ids().expect("ids").is_empty());
    assert!(fixture.retrieval.indexed_ids().expect("ids").is_empty());
    assert!(fixture
        .derived
        .last_rebuild
        .lock()
        .expect("rebuild")
        .is_empty());
}

#[test]
fn reconciliation_revokes_an_orphaned_retrieval_entry_without_restoring_the_memory() {
    let fixture = fixture();
    fixture
        .retrieval
        .upsert(&record(42, MemoryStatus::Active))
        .expect("seed orphan");

    let outcome = fixture.service.reconcile().expect("reconcile");
    assert_eq!(outcome.revoked_orphan_retrieval_entries, 1);
    assert!(fixture.retrieval.indexed_ids().expect("ids").is_empty());
    assert!(
        fixture.files.records.lock().expect("records").is_empty(),
        "revoking an orphan must not resurrect a record"
    );
}

#[test]
fn reconciliation_rebuilds_the_projection_and_index_from_the_files() {
    let fixture = fixture();
    fixture
        .service
        .create(input("Kept", MemoryStatus::Active))
        .expect("create");
    fixture
        .projection
        .clear()
        .expect("simulate a lost projection");

    let outcome = fixture.service.reconcile().expect("reconcile");
    assert_eq!(outcome.rebuilt_projection_rows, 1);
    assert_eq!(outcome.rebuilt_index_lines, 1);
    assert_eq!(fixture.projection.projected_ids().expect("ids").len(), 1);
    assert!(!outcome.failures.iter().any(|failure| matches!(
        failure.phase,
        MaintenancePhase::SqliteProjection | MaintenancePhase::DerivedIndex
    )));
}

#[test]
fn a_list_page_comes_from_the_projection_not_the_files() {
    // Structural: the service's only list path is the projection, so no caller can reach an
    // N+1 read of every body to render a page.
    let fixture = fixture();
    fixture
        .service
        .create(input("Use npm", MemoryStatus::Active))
        .expect("create");
    let page = fixture.service.list(&MemoryQuery::default()).expect("list");
    assert!(
        page.items.is_empty(),
        "the fake projection answers, which is the point"
    );
}
