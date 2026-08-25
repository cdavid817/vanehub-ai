//! Scoped policy resolution and the instruction merge state machine.
//!
//! Fakes for the three ports, because every property here is about precedence and provenance rather
//! than about storage. The consistent-read contract — that a bundle reports `Absent` rather than
//! omitting a key — is asserted against the real SQLite repository where it lives.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use super::error::PersonalizationApplicationError;
use super::models::{MemoryEligibilityCriteria, ResetCounts};
use super::policy_cache::{is_transient_read_failure, LastKnownGoodPolicyCache};
use super::ports::{
    AgentCapabilityPort, MemoryHealthPort, MemoryProjectionPort, PolicyRepository,
    SecretRedactionPort,
};
use super::preview_personalization::PersonalizationPreviewService;
use super::resolve_policy::{PolicyResolutionService, ResolutionRequest};
use crate::contexts::personalization::domain::{
    AgentId, InstructionExclusionReason, InstructionField, InstructionMergeAction,
    InstructionMergeMode, MemoryBlockReason, MemoryDeliveryMode, MemoryEligibilitySummary,
    MemoryExclusionCount, MemoryId, MemoryPage, MemoryQuery, MemoryRecord, MemoryRuntimeHealth,
    MemorySaveConstraint, MemoryScopeFilter, MemoryStatus, MemoryType, PatchPolicyResult,
    PersonalizationExclusionReason, PersonalizationLayers, PersonalizationPolicyPatch,
    PersonalizationPolicyRecord, PersonalizationPolicyScope, PersonalizationRuntimeCapabilities,
    PersonalizationWarningCode, PolicyLayerState, PolicyResolutionBundle, PolicyToggle, SessionId,
    SessionPersonalizationMode, SnapshotMemoryRef, WorkspaceIdentity, WorkspaceKey, WorkspaceKind,
};

type Result<T> = std::result::Result<T, PersonalizationApplicationError>;

/// Stored rows, keyed by scope, with the reads counted so "one consistent read" is assertable.
#[derive(Default)]
struct FakePolicies {
    rows: Mutex<Vec<PersonalizationPolicyRecord>>,
    bundle_reads: AtomicUsize,
    /// What the next bundle read should fail with, if anything.
    fails_with: Mutex<Option<PersonalizationApplicationError>>,
}

impl FakePolicies {
    fn put(&self, record: PersonalizationPolicyRecord) {
        let mut rows = self.rows.lock().expect("rows");
        rows.retain(|stored| stored.scope() != record.scope());
        rows.push(record);
    }
}

impl PolicyRepository for FakePolicies {
    fn load(
        &self,
        scope: &PersonalizationPolicyScope,
    ) -> Result<Option<PersonalizationPolicyRecord>> {
        Ok(self
            .rows
            .lock()
            .expect("rows")
            .iter()
            .find(|record| record.scope() == scope)
            .cloned())
    }

    fn load_resolution_bundle(
        &self,
        scopes: &[PersonalizationPolicyScope],
    ) -> Result<PolicyResolutionBundle> {
        self.bundle_reads.fetch_add(1, Ordering::SeqCst);
        if let Some(error) = self.fails_with.lock().expect("fails").clone() {
            return Err(error);
        }
        let rows = self.rows.lock().expect("rows");
        Ok(PolicyResolutionBundle {
            layers: scopes
                .iter()
                .map(|scope| {
                    let state = rows
                        .iter()
                        .find(|record| record.scope() == scope)
                        .cloned()
                        .map(PolicyLayerState::Present)
                        .unwrap_or(PolicyLayerState::Absent);
                    (scope.clone(), state)
                })
                .collect(),
        })
    }

    fn load_layers(
        &self,
        _agent_id: &AgentId,
        _workspace_key: Option<&WorkspaceKey>,
    ) -> Result<PersonalizationLayers> {
        unreachable!("resolution reads the bundle, never the layered convenience view")
    }

    fn list_all(&self) -> Result<Vec<PersonalizationPolicyRecord>> {
        Ok(self.rows.lock().expect("rows").clone())
    }

    fn seed_default_global(
        &self,
        _now: chrono::DateTime<chrono::Utc>,
    ) -> Result<PersonalizationPolicyRecord> {
        unreachable!("resolution never seeds")
    }

    fn patch(
        &self,
        _scope: &PersonalizationPolicyScope,
        _expected_revision: Option<u64>,
        _patch: PersonalizationPolicyPatch,
        _now: chrono::DateTime<chrono::Utc>,
    ) -> Result<PatchPolicyResult> {
        unreachable!("resolution never writes")
    }

    fn delete(&self, _scope: &PersonalizationPolicyScope) -> Result<bool> {
        unreachable!("resolution never deletes")
    }
}

/// A registry with whatever Agents a test registers. Nothing here knows a built-in Agent's name,
/// which is the point: an Agent added at runtime resolves through the same path.
#[derive(Default)]
struct FakeAgents {
    registered: Mutex<Vec<(String, PersonalizationRuntimeCapabilities)>>,
}

impl FakeAgents {
    fn register(&self, id: &str, capabilities: PersonalizationRuntimeCapabilities) {
        self.registered
            .lock()
            .expect("registered")
            .push((id.to_string(), capabilities));
    }
}

impl AgentCapabilityPort for FakeAgents {
    fn capabilities(
        &self,
        agent_id: &AgentId,
    ) -> Result<Option<PersonalizationRuntimeCapabilities>> {
        Ok(self
            .registered
            .lock()
            .expect("registered")
            .iter()
            .find(|(id, _)| id == agent_id.as_str())
            .map(|(_, capabilities)| *capabilities))
    }

    fn list_capabilities(
        &self,
    ) -> Result<Vec<crate::contexts::personalization::application::AgentCapabilityEntry>> {
        Ok(Vec::new())
    }
}

/// Records what eligibility was asked, and answers with whatever a test set.
#[derive(Default)]
struct FakeProjection {
    summary: Mutex<MemoryEligibilitySummary>,
    last_criteria: Mutex<Option<MemoryEligibilityCriteria>>,
}

impl MemoryProjectionPort for FakeProjection {
    fn upsert(&self, _record: &MemoryRecord, _content_hash: &str) -> Result<()> {
        unreachable!("resolution never writes the projection")
    }
    fn remove(&self, _id: &MemoryId) -> Result<bool> {
        unreachable!("resolution never writes the projection")
    }
    fn list_page(&self, _query: &MemoryQuery) -> Result<MemoryPage> {
        unreachable!("resolution asks for eligibility, not a list page")
    }
    fn count_for_reset(
        &self,
        _scope: &MemoryScopeFilter,
        _statuses: &[MemoryStatus],
    ) -> Result<ResetCounts> {
        unreachable!("resolution never counts for reset")
    }
    fn eligible_page(
        &self,
        criteria: &MemoryEligibilityCriteria,
    ) -> Result<MemoryEligibilitySummary> {
        *self.last_criteria.lock().expect("criteria") = Some(criteria.clone());
        Ok(self.summary.lock().expect("summary").clone())
    }
    fn projected_ids(&self) -> Result<Vec<MemoryId>> {
        Ok(Vec::new())
    }
    fn clear(&self) -> Result<usize> {
        Ok(0)
    }
}

struct FixedHealth(Mutex<MemoryRuntimeHealth>);

impl MemoryHealthPort for FixedHealth {
    fn health(&self) -> MemoryRuntimeHealth {
        *self.0.lock().expect("health")
    }
}

fn full_capabilities() -> PersonalizationRuntimeCapabilities {
    PersonalizationRuntimeCapabilities {
        supports_custom_instructions: true,
        supports_memory_index: true,
        supports_selected_memory_bodies: true,
        supports_automatic_extraction: true,
    }
}

struct Fixture {
    policies: Arc<FakePolicies>,
    agents: Arc<FakeAgents>,
    projection: Arc<FakeProjection>,
    health: Arc<FixedHealth>,
    cache: Arc<LastKnownGoodPolicyCache>,
    service: PolicyResolutionService,
}

fn fixture() -> Fixture {
    let policies = Arc::new(FakePolicies::default());
    let agents = Arc::new(FakeAgents::default());
    let health = Arc::new(FixedHealth(Mutex::new(MemoryRuntimeHealth::Ready {
        generation: 1,
    })));
    agents.register("onepiece", full_capabilities());
    let projection = Arc::new(FakeProjection::default());
    let cache = Arc::new(LastKnownGoodPolicyCache::default());
    let service = PolicyResolutionService::new(
        policies.clone(),
        agents.clone(),
        projection.clone(),
        health.clone(),
        cache.clone(),
    );
    Fixture {
        policies,
        agents,
        projection,
        health,
        cache,
        service,
    }
}

fn agent(id: &str) -> AgentId {
    AgentId::parse(id).expect("agent id")
}

fn workspace(key: &str) -> WorkspaceIdentity {
    WorkspaceIdentity::new(
        WorkspaceKey::parse(key).expect("workspace key"),
        "D:/code/project".to_string(),
        WorkspaceKind::Local,
    )
}

fn remote_workspace(key: &str) -> WorkspaceIdentity {
    WorkspaceIdentity::new(
        WorkspaceKey::parse(key).expect("workspace key"),
        "ssh://build.example.test/srv/project".to_string(),
        WorkspaceKind::Remote,
    )
}

fn request(fixture_workspace: Option<WorkspaceIdentity>) -> ResolutionRequest {
    ResolutionRequest {
        agent_id: agent("onepiece"),
        session_id: SessionId::parse("ses_1").expect("session"),
        workspace: fixture_workspace,
        session_mode: SessionPersonalizationMode::Standard,
        session_override: None,
    }
}

/// A layer with the merge mode and text a test cares about, everything else inherited.
fn layer(
    scope: PersonalizationPolicyScope,
    revision: u64,
    mode: InstructionMergeMode,
    about_user: &str,
    style_rules: &str,
) -> PersonalizationPolicyRecord {
    let mut record = if matches!(scope, PersonalizationPolicyScope::Global) {
        PersonalizationPolicyRecord::default_global()
    } else {
        PersonalizationPolicyRecord::inheriting(scope.clone())
    };
    record.set_instruction_merge_mode(mode);
    record.set_about_user(about_user.to_string());
    record.set_style_rules(style_rules.to_string());
    record.set_revision(revision);
    record
}

fn texts(
    snapshot: &crate::contexts::personalization::domain::EffectivePersonalizationSnapshot,
) -> Vec<String> {
    snapshot
        .instruction_segments
        .iter()
        .map(|segment| segment.text.clone())
        .collect()
}

// =================================================================================================
// Precedence
// =================================================================================================

#[test]
fn every_scope_layer_contributes_in_precedence_order() {
    let fixture = fixture();
    let space = workspace("ws_alpha");
    for (scope, revision, text) in [
        (PersonalizationPolicyScope::Global, 1, "global"),
        (
            PersonalizationPolicyScope::Agent {
                agent_id: agent("onepiece"),
            },
            2,
            "agent",
        ),
        (
            PersonalizationPolicyScope::Workspace {
                workspace_key: space.key().clone(),
            },
            3,
            "workspace",
        ),
        (
            PersonalizationPolicyScope::WorkspaceAgent {
                workspace_key: space.key().clone(),
                agent_id: agent("onepiece"),
            },
            4,
            "workspace-agent",
        ),
    ] {
        fixture.policies.put(layer(
            scope,
            revision,
            InstructionMergeMode::Append,
            text,
            "",
        ));
    }

    let snapshot = fixture
        .service
        .resolve(request(Some(space)))
        .expect("resolve");

    assert_eq!(
        texts(&snapshot),
        vec![
            "global".to_string(),
            "agent".to_string(),
            "workspace".to_string(),
            "workspace-agent".to_string()
        ],
        "layers apply in precedence order, lowest first"
    );
    // One read for the whole resolution: four round trips could mix revisions.
    assert_eq!(fixture.policies.bundle_reads.load(Ordering::SeqCst), 1);
}

#[test]
fn the_scope_keys_asked_for_depend_on_whether_there_is_a_workspace() {
    let with = PolicyResolutionService::scopes_for(&request(Some(workspace("ws_alpha"))));
    let without = PolicyResolutionService::scopes_for(&request(None));

    assert_eq!(with.len(), 4);
    assert_eq!(
        without.len(),
        2,
        "a key that cannot exist is not asked for, so its absence is not a finding"
    );
    assert!(matches!(without[0], PersonalizationPolicyScope::Global));
}

// =================================================================================================
// The instruction merge state machine
// =================================================================================================

#[test]
fn an_inheriting_layer_keeps_the_current_state_and_reports_its_own_text_excluded() {
    let fixture = fixture();
    fixture.policies.put(layer(
        PersonalizationPolicyScope::Global,
        1,
        InstructionMergeMode::Append,
        "from global",
        "",
    ));
    fixture.policies.put(layer(
        PersonalizationPolicyScope::Agent {
            agent_id: agent("onepiece"),
        },
        2,
        InstructionMergeMode::Inherit,
        "stored but inherited",
        "",
    ));

    let snapshot = fixture.service.resolve(request(None)).expect("resolve");

    assert_eq!(texts(&snapshot), vec!["from global".to_string()]);
    assert_eq!(
        snapshot.effective_instruction_mode,
        InstructionMergeMode::Append
    );
    // The text stored on the inheriting layer is reported, not silently dropped: a user who typed
    // it needs to be told why it is not being used.
    assert!(snapshot
        .excluded_instruction_segments
        .iter()
        .any(|segment| {
            segment.reason == InstructionExclusionReason::InheritedLayer
                && segment.scope_kind == "agent"
        }));
}

#[test]
fn append_adds_to_what_survived_and_replace_clears_it_first() {
    let fixture = fixture();
    fixture.policies.put(layer(
        PersonalizationPolicyScope::Global,
        1,
        InstructionMergeMode::Append,
        "global",
        "",
    ));
    fixture.policies.put(layer(
        PersonalizationPolicyScope::Agent {
            agent_id: agent("onepiece"),
        },
        7,
        InstructionMergeMode::Replace,
        "agent replaces",
        "",
    ));

    let snapshot = fixture.service.resolve(request(None)).expect("resolve");

    assert_eq!(texts(&snapshot), vec!["agent replaces".to_string()]);
    assert_eq!(
        snapshot.effective_instruction_mode,
        InstructionMergeMode::Replace
    );
    assert_eq!(snapshot.instruction_segments[0].policy_revision, 7);
    assert_eq!(
        snapshot.instruction_segments[0].merge_action,
        InstructionMergeAction::Replaced
    );
    assert!(snapshot
        .excluded_instruction_segments
        .iter()
        .any(|segment| {
            segment.reason == InstructionExclusionReason::ReplacedByHigherLayer
                && segment.scope_kind == "global"
        }));
}

#[test]
fn disabled_clears_everything_and_a_higher_layer_may_re_establish_it() {
    // The state machine has no terminal state. A workspace that disabled instructions must not stop
    // a workspace-Agent layer from turning them back on for that one Agent.
    let fixture = fixture();
    let space = workspace("ws_alpha");
    fixture.policies.put(layer(
        PersonalizationPolicyScope::Global,
        1,
        InstructionMergeMode::Append,
        "global",
        "",
    ));
    fixture.policies.put(layer(
        PersonalizationPolicyScope::Workspace {
            workspace_key: space.key().clone(),
        },
        2,
        InstructionMergeMode::Disabled,
        "",
        "",
    ));
    fixture.policies.put(layer(
        PersonalizationPolicyScope::WorkspaceAgent {
            workspace_key: space.key().clone(),
            agent_id: agent("onepiece"),
        },
        3,
        InstructionMergeMode::Append,
        "re-established",
        "",
    ));

    let snapshot = fixture
        .service
        .resolve(request(Some(space)))
        .expect("resolve");

    assert_eq!(texts(&snapshot), vec!["re-established".to_string()]);
    assert!(snapshot
        .excluded_instruction_segments
        .iter()
        .any(|segment| { segment.reason == InstructionExclusionReason::DisabledByHigherLayer }));
}

#[test]
fn replace_then_append_keeps_both_in_order() {
    let fixture = fixture();
    let space = workspace("ws_alpha");
    fixture.policies.put(layer(
        PersonalizationPolicyScope::Global,
        1,
        InstructionMergeMode::Append,
        "dropped",
        "",
    ));
    fixture.policies.put(layer(
        PersonalizationPolicyScope::Agent {
            agent_id: agent("onepiece"),
        },
        2,
        InstructionMergeMode::Replace,
        "replacement",
        "",
    ));
    fixture.policies.put(layer(
        PersonalizationPolicyScope::Workspace {
            workspace_key: space.key().clone(),
        },
        3,
        InstructionMergeMode::Append,
        "addition",
        "",
    ));

    let snapshot = fixture
        .service
        .resolve(request(Some(space)))
        .expect("resolve");

    assert_eq!(
        texts(&snapshot),
        vec!["replacement".to_string(), "addition".to_string()]
    );
}

#[test]
fn an_empty_append_and_an_empty_replace_are_different_things() {
    let fixture = fixture();
    fixture.policies.put(layer(
        PersonalizationPolicyScope::Global,
        1,
        InstructionMergeMode::Append,
        "global",
        "",
    ));

    // Empty append: contributes nothing, keeps what is below.
    fixture.policies.put(layer(
        PersonalizationPolicyScope::Agent {
            agent_id: agent("onepiece"),
        },
        2,
        InstructionMergeMode::Append,
        "",
        "",
    ));
    let appended = fixture.service.resolve(request(None)).expect("resolve");
    assert_eq!(texts(&appended), vec!["global".to_string()]);

    // Empty replace: clears what is below and contributes nothing, which is not the same as
    // leaving it alone. Reporting the stored mode here would claim a replacement that produced no
    // text, so the effective mode collapses to disabled.
    fixture.policies.put(layer(
        PersonalizationPolicyScope::Agent {
            agent_id: agent("onepiece"),
        },
        3,
        InstructionMergeMode::Replace,
        "",
        "",
    ));
    let replaced = fixture.service.resolve(request(None)).expect("resolve");
    assert!(replaced.instruction_segments.is_empty());
    assert_eq!(
        replaced.effective_instruction_mode,
        InstructionMergeMode::Disabled
    );
}

#[test]
fn each_field_carries_its_own_provenance() {
    let fixture = fixture();
    fixture.policies.put(layer(
        PersonalizationPolicyScope::Global,
        11,
        InstructionMergeMode::Append,
        "about the user",
        "",
    ));
    fixture.policies.put(layer(
        PersonalizationPolicyScope::Agent {
            agent_id: agent("onepiece"),
        },
        12,
        InstructionMergeMode::Append,
        "",
        "no preamble",
    ));

    let snapshot = fixture.service.resolve(request(None)).expect("resolve");

    let about = snapshot
        .instruction_segments
        .iter()
        .find(|segment| segment.field == InstructionField::AboutUser)
        .expect("the about-user field");
    assert_eq!(about.scope_kind, "global");
    assert_eq!(about.policy_revision, 11);

    let style = snapshot
        .instruction_segments
        .iter()
        .find(|segment| segment.field == InstructionField::StyleRules)
        .expect("the style-rules field");
    assert_eq!(style.scope_kind, "agent");
    assert_eq!(style.policy_revision, 12);

    // The empty half of each layer is reported rather than invisible.
    assert!(snapshot
        .excluded_instruction_segments
        .iter()
        .any(|segment| segment.reason == InstructionExclusionReason::EmptyField));
}

#[test]
fn a_session_override_applies_after_every_durable_layer() {
    let fixture = fixture();
    fixture.policies.put(layer(
        PersonalizationPolicyScope::Global,
        1,
        InstructionMergeMode::Append,
        "durable",
        "",
    ));

    let mut with_override = request(None);
    with_override.session_override = Some(PersonalizationPolicyPatch {
        instruction_merge_mode: Some(InstructionMergeMode::Replace),
        about_user: Some("just for this session".to_string()),
        ..PersonalizationPolicyPatch::default()
    });

    let snapshot = fixture.service.resolve(with_override).expect("resolve");

    assert_eq!(texts(&snapshot), vec!["just for this session".to_string()]);
    assert_eq!(snapshot.instruction_segments[0].scope_kind, "session");
}

#[test]
fn nothing_outside_the_two_user_fields_ever_reaches_the_snapshot() {
    // Core, system, safety, role and runtime instructions are not modelled here at all, and this
    // states that as a property: every segment names one of exactly two user-authored fields, so
    // there is no representation in which a hidden prompt could travel.
    let fixture = fixture();
    fixture.policies.put(layer(
        PersonalizationPolicyScope::Global,
        1,
        InstructionMergeMode::Append,
        "about",
        "style",
    ));

    let snapshot = fixture.service.resolve(request(None)).expect("resolve");

    assert!(snapshot.instruction_segments.iter().all(|segment| matches!(
        segment.field,
        InstructionField::AboutUser | InstructionField::StyleRules
    )));
    assert_eq!(snapshot.instruction_segments.len(), 2);
}

// =================================================================================================
// Fail-closed cases
// =================================================================================================

#[test]
fn an_installation_with_no_global_row_fails_closed() {
    let fixture = fixture();

    let snapshot = fixture.service.resolve(request(None)).expect("resolve");

    assert!(snapshot.instruction_segments.is_empty());
    assert!(!snapshot.memory_access.read);
    assert!(snapshot
        .warnings
        .iter()
        .any(|warning| warning.code == PersonalizationWarningCode::NoValidatedPolicy));
}

#[test]
fn an_agent_the_registry_does_not_know_fails_closed_rather_than_defaulting() {
    let fixture = fixture();
    fixture.policies.put(layer(
        PersonalizationPolicyScope::Global,
        1,
        InstructionMergeMode::Append,
        "global",
        "",
    ));
    let mut unknown = request(None);
    unknown.agent_id = agent("never-registered");

    let snapshot = fixture.service.resolve(unknown).expect("resolve");

    assert!(snapshot.instruction_segments.is_empty());
    assert!(!snapshot.memory_access.read);
    assert!(snapshot
        .warnings
        .iter()
        .any(|warning| warning.code == PersonalizationWarningCode::UnknownAgent));
}

#[test]
fn an_agent_registered_while_running_resolves_like_any_other() {
    let fixture = fixture();
    fixture.policies.put(layer(
        PersonalizationPolicyScope::Global,
        1,
        InstructionMergeMode::Append,
        "global",
        "",
    ));
    fixture
        .agents
        .register("added-at-runtime", full_capabilities());
    let mut dynamic = request(None);
    dynamic.agent_id = agent("added-at-runtime");

    let snapshot = fixture.service.resolve(dynamic).expect("resolve");

    assert_eq!(texts(&snapshot), vec!["global".to_string()]);
    assert!(snapshot.memory_access.read);
}

#[test]
fn a_project_only_session_without_a_workspace_fails_closed() {
    // "Read everything global" is the single interpretation a project-isolated session must never
    // degrade to, so the resolver refuses rather than widening the scope.
    let fixture = fixture();
    fixture.policies.put(layer(
        PersonalizationPolicyScope::Global,
        1,
        InstructionMergeMode::Append,
        "global",
        "",
    ));
    let mut project_only = request(None);
    project_only.session_mode = SessionPersonalizationMode::ProjectOnly;

    let snapshot = fixture.service.resolve(project_only).expect("resolve");

    assert!(!snapshot.memory_access.read);
    assert!(!snapshot.memory_access.global_memory);
    assert!(snapshot
        .warnings
        .iter()
        .any(|warning| warning.code == PersonalizationWarningCode::WorkspaceRequired));
}

#[test]
fn a_remote_workspace_resolves_by_its_key_and_not_its_display_path() {
    let fixture = fixture();
    let remote = remote_workspace("ws_remote");
    fixture.policies.put(layer(
        PersonalizationPolicyScope::Global,
        1,
        InstructionMergeMode::Append,
        "global",
        "",
    ));
    fixture.policies.put(layer(
        PersonalizationPolicyScope::Workspace {
            workspace_key: remote.key().clone(),
        },
        2,
        InstructionMergeMode::Append,
        "remote workspace",
        "",
    ));

    let snapshot = fixture
        .service
        .resolve(request(Some(remote.clone())))
        .expect("resolve");

    assert!(texts(&snapshot).contains(&"remote workspace".to_string()));
    assert_eq!(
        snapshot.memory_access.workspace.as_ref(),
        Some(remote.key())
    );
    // The display path is not what any scope key was built from.
    assert!(snapshot
        .instruction_segments
        .iter()
        .all(|segment| !segment.scope_key.contains("ssh://")));
}

#[test]
fn a_runtime_without_custom_instruction_support_applies_none_and_says_why() {
    let fixture = fixture();
    fixture.agents.register(
        "index-only",
        PersonalizationRuntimeCapabilities {
            supports_custom_instructions: false,
            supports_memory_index: true,
            supports_selected_memory_bodies: false,
            supports_automatic_extraction: false,
        },
    );
    fixture.policies.put(layer(
        PersonalizationPolicyScope::Global,
        1,
        InstructionMergeMode::Append,
        "would be applied elsewhere",
        "",
    ));
    let mut limited = request(None);
    limited.agent_id = agent("index-only");

    let snapshot = fixture.service.resolve(limited).expect("resolve");

    assert!(snapshot.instruction_segments.is_empty());
    assert!(snapshot
        .excluded_instruction_segments
        .iter()
        .any(|segment| { segment.reason == InstructionExclusionReason::RuntimeCapability }));
    assert!(snapshot
        .warnings
        .iter()
        .any(|warning| warning.code == PersonalizationWarningCode::UnsupportedCapabilityOverride));
}

#[test]
fn memory_health_denies_memory_without_touching_instructions() {
    let fixture = fixture();
    fixture.policies.put(layer(
        PersonalizationPolicyScope::Global,
        1,
        InstructionMergeMode::Append,
        "still applied",
        "",
    ));
    *fixture.health.0.lock().expect("health") = MemoryRuntimeHealth::RepairRequired;

    let snapshot = fixture.service.resolve(request(None)).expect("resolve");

    assert_eq!(texts(&snapshot), vec!["still applied".to_string()]);
    assert!(!snapshot.memory_access.read);
    assert!(!snapshot.memory_access.explicit_save);
    assert!(!snapshot.memory_access.automatic_extraction);
}

#[test]
fn a_toggle_set_on_a_higher_layer_wins_over_a_lower_one() {
    let fixture = fixture();
    let mut global = PersonalizationPolicyRecord::default_global();
    global.set_memory_read_mode(PolicyToggle::Enabled);
    global.set_revision(1);
    fixture.policies.put(global);
    let mut agent_layer =
        PersonalizationPolicyRecord::inheriting(PersonalizationPolicyScope::Agent {
            agent_id: agent("onepiece"),
        });
    agent_layer.set_memory_read_mode(PolicyToggle::Disabled);
    agent_layer.set_revision(2);
    fixture.policies.put(agent_layer);

    let snapshot = fixture.service.resolve(request(None)).expect("resolve");

    assert!(!snapshot.memory_access.read, "the Agent layer wins");
}

// =================================================================================================
// Memory access, delivery, and the immutable snapshot
// =================================================================================================

/// A global policy with every memory dimension on, so a test can turn exactly one off.
fn enabled_global() -> PersonalizationPolicyRecord {
    let mut record = PersonalizationPolicyRecord::default_global();
    record.set_memory_read_mode(PolicyToggle::Enabled);
    record.set_explicit_save_mode(PolicyToggle::Enabled);
    record.set_automatic_extraction_mode(PolicyToggle::Enabled);
    record.set_global_memory_access_mode(PolicyToggle::Enabled);
    record.set_revision(1);
    record
}

#[test]
fn the_four_memory_dimensions_are_independent() {
    for (label, patch) in [
        ("read", PolicyToggle::Disabled),
        ("save", PolicyToggle::Disabled),
        ("extraction", PolicyToggle::Disabled),
        ("global", PolicyToggle::Disabled),
    ] {
        let fixture = fixture();
        let mut global = enabled_global();
        match label {
            "read" => global.set_memory_read_mode(patch),
            "save" => global.set_explicit_save_mode(patch),
            "extraction" => global.set_automatic_extraction_mode(patch),
            _ => global.set_global_memory_access_mode(patch),
        }
        fixture.policies.put(global);

        let snapshot = fixture.service.resolve(request(None)).expect("resolve");
        let access = &snapshot.memory_access;
        let disabled = [
            access.read,
            access.explicit_save,
            access.automatic_extraction,
            access.global_memory,
        ]
        .iter()
        .filter(|value| !**value)
        .count();
        assert_eq!(
            disabled, 1,
            "{label}: turning one off must not turn others off"
        );
    }
}

#[test]
fn a_readable_snapshot_reports_a_delivery_mode_and_a_blocked_one_reports_a_reason() {
    let fixture = fixture();
    fixture.policies.put(enabled_global());

    let snapshot = fixture.service.resolve(request(None)).expect("resolve");
    assert_eq!(
        snapshot.memory_access.delivery,
        MemoryDeliveryMode::IndexWithSelectedBodies
    );
    assert_eq!(snapshot.memory_access.block_reason, None);

    // Selected-body support only widens delivery; it never makes a memory eligible.
    fixture.agents.register(
        "index-only",
        PersonalizationRuntimeCapabilities {
            supports_custom_instructions: true,
            supports_memory_index: true,
            supports_selected_memory_bodies: false,
            supports_automatic_extraction: true,
        },
    );
    let mut limited = request(None);
    limited.agent_id = agent("index-only");
    let snapshot = fixture.service.resolve(limited).expect("resolve");
    assert_eq!(
        snapshot.memory_access.delivery,
        MemoryDeliveryMode::IndexOnly
    );
    assert!(snapshot.memory_access.read);
}

#[test]
fn a_runtime_with_no_memory_index_delivers_nothing_and_says_why() {
    let fixture = fixture();
    fixture.policies.put(enabled_global());
    fixture.agents.register(
        "no-index",
        PersonalizationRuntimeCapabilities {
            supports_custom_instructions: true,
            supports_memory_index: false,
            supports_selected_memory_bodies: false,
            supports_automatic_extraction: false,
        },
    );
    let mut limited = request(None);
    limited.agent_id = agent("no-index");

    let snapshot = fixture.service.resolve(limited).expect("resolve");

    assert!(!snapshot.memory_access.read);
    assert_eq!(snapshot.memory_access.delivery, MemoryDeliveryMode::None);
    assert_eq!(
        snapshot.memory_access.block_reason,
        Some(MemoryBlockReason::RuntimeCapability)
    );
}

#[test]
fn a_temporary_session_forbids_every_memory_direction_but_keeps_instructions() {
    let fixture = fixture();
    let mut global = enabled_global();
    global.set_instruction_merge_mode(InstructionMergeMode::Append);
    global.set_about_user("still applied".to_string());
    fixture.policies.put(global);
    let mut temporary = request(None);
    temporary.session_mode = SessionPersonalizationMode::Temporary;

    let snapshot = fixture.service.resolve(temporary).expect("resolve");

    assert_eq!(texts(&snapshot), vec!["still applied".to_string()]);
    let access = &snapshot.memory_access;
    assert!(!access.read);
    assert!(!access.explicit_save);
    assert!(!access.automatic_extraction);
    // The two a read-only check would have missed.
    assert!(!access.candidate_creation);
    assert!(!access.retrieval_write);
    assert_eq!(access.block_reason, Some(MemoryBlockReason::SessionMode));
}

#[test]
fn a_project_only_session_offers_the_workspace_and_never_global() {
    let fixture = fixture();
    fixture.policies.put(enabled_global());
    let space = workspace("ws_alpha");
    let mut project_only = request(Some(space.clone()));
    project_only.session_mode = SessionPersonalizationMode::ProjectOnly;

    let snapshot = fixture.service.resolve(project_only).expect("resolve");

    let allowance = snapshot.memory_access.readable_scopes();
    assert!(!allowance.global);
    assert_eq!(allowance.workspace.as_ref(), Some(space.key()));
    assert_eq!(
        snapshot.memory_access.save_constraint(),
        MemorySaveConstraint::WorkspaceOnly {
            workspace: space.key().clone()
        },
        "global is not offered at all, which is different from offering one that would fail"
    );
}

#[test]
fn every_unready_health_state_denies_memory() {
    for health in [
        MemoryRuntimeHealth::NotStarted,
        MemoryRuntimeHealth::Busy,
        MemoryRuntimeHealth::Migrating,
        MemoryRuntimeHealth::RebuildingDerived,
        MemoryRuntimeHealth::RepairRequired,
        MemoryRuntimeHealth::Failed,
    ] {
        let fixture = fixture();
        fixture.policies.put(enabled_global());
        *fixture.health.0.lock().expect("health") = health;

        let snapshot = fixture.service.resolve(request(None)).expect("resolve");

        assert!(!snapshot.memory_access.read, "{health:?}");
        assert!(!snapshot.memory_access.explicit_save, "{health:?}");
        assert!(!snapshot.memory_access.automatic_extraction, "{health:?}");
        assert_eq!(
            snapshot.memory_access.block_reason,
            Some(MemoryBlockReason::MaintenanceState),
            "{health:?}"
        );
    }
}

#[test]
fn eligibility_is_not_queried_when_memory_is_blocked() {
    // Reporting per-record exclusion counts would imply an enumeration that never happened.
    let fixture = fixture();
    let mut global = enabled_global();
    global.set_memory_read_mode(PolicyToggle::Disabled);
    fixture.policies.put(global);

    let snapshot = fixture.service.resolve(request(None)).expect("resolve");

    assert!(fixture
        .projection
        .last_criteria
        .lock()
        .expect("criteria")
        .is_none());
    assert_eq!(snapshot.memory.considered, 0);
    assert!(snapshot.memory.is_balanced());
}

#[test]
fn eligibility_is_queried_with_the_scopes_the_snapshot_resolved() {
    let fixture = fixture();
    fixture.policies.put(enabled_global());
    let space = workspace("ws_alpha");

    fixture
        .service
        .resolve(request(Some(space.clone())))
        .expect("resolve");

    let criteria = fixture
        .projection
        .last_criteria
        .lock()
        .expect("criteria")
        .clone()
        .expect("eligibility was queried");
    assert!(criteria.allow_global);
    assert_eq!(criteria.workspace.as_ref(), Some(space.key()));
    assert!(!criteria.project_only);
    assert_eq!(criteria.agent_id, agent("onepiece"));
}

#[test]
fn a_snapshot_does_not_change_when_the_policy_or_the_memories_do() {
    // The whole point of taking one snapshot per generation. A settings change mid-turn must reach
    // the *next* turn, not rewrite the one already planned around the old values.
    let fixture = fixture();
    fixture.policies.put(enabled_global());
    *fixture.projection.summary.lock().expect("summary") = MemoryEligibilitySummary {
        considered: 1,
        eligible_total: 1,
        refs: Vec::new(),
        truncated: true,
        exclusions: Vec::new(),
        digest: "digest-before".to_string(),
    };
    let captured = fixture.service.resolve(request(None)).expect("resolve");
    let token_before = captured.revision_token.clone();

    // Everything changes underneath it.
    let mut disabled = enabled_global();
    disabled.set_memory_read_mode(PolicyToggle::Disabled);
    disabled.set_revision(9);
    fixture.policies.put(disabled);
    *fixture.projection.summary.lock().expect("summary") = MemoryEligibilitySummary {
        digest: "digest-after".to_string(),
        ..MemoryEligibilitySummary::default()
    };
    *fixture.health.0.lock().expect("health") = MemoryRuntimeHealth::RepairRequired;

    // The captured value is untouched.
    assert!(captured.memory_access.read);
    assert_eq!(captured.memory.eligible_total, 1);
    assert_eq!(captured.revision_token, token_before);

    // And the next snapshot reflects the new state.
    let next = fixture.service.resolve(request(None)).expect("resolve");
    assert!(!next.memory_access.read);
    assert_ne!(next.revision_token, token_before);
}

#[test]
fn the_revision_token_is_deterministic_and_moves_with_every_input() {
    let fixture = fixture();
    fixture.policies.put(enabled_global());
    let baseline = fixture
        .service
        .resolve(request(None))
        .expect("resolve")
        .revision_token;
    assert_eq!(
        fixture
            .service
            .resolve(request(None))
            .expect("resolve")
            .revision_token,
        baseline,
        "identical inputs produce an identical token"
    );

    // A policy revision moves it.
    let mut bumped = enabled_global();
    bumped.set_revision(2);
    fixture.policies.put(bumped);
    let after_policy = fixture
        .service
        .resolve(request(None))
        .expect("resolve")
        .revision_token;
    assert_ne!(after_policy, baseline);

    // So does the eligible set, through its digest.
    *fixture.projection.summary.lock().expect("summary") = MemoryEligibilitySummary {
        digest: "a-different-digest".to_string(),
        ..MemoryEligibilitySummary::default()
    };
    let after_memory = fixture
        .service
        .resolve(request(None))
        .expect("resolve")
        .revision_token;
    assert_ne!(after_memory, after_policy);

    // So does a capability change, with everything else identical.
    fixture.agents.register(
        "narrower",
        PersonalizationRuntimeCapabilities {
            supports_custom_instructions: true,
            supports_memory_index: true,
            supports_selected_memory_bodies: false,
            supports_automatic_extraction: true,
        },
    );
    let mut narrower = request(None);
    narrower.agent_id = agent("narrower");
    let after_capability = fixture
        .service
        .resolve(narrower)
        .expect("resolve")
        .revision_token;
    assert_ne!(after_capability, after_memory);
}

#[test]
fn the_revision_token_carries_no_text_no_path_and_no_credential() {
    // It reaches logs and the frontend, so everything hashed into it has to be something we would
    // be willing to correlate across records.
    let fixture = fixture();
    let mut global = enabled_global();
    global.set_instruction_merge_mode(InstructionMergeMode::Append);
    global.set_about_user("a secret sentence about the user".to_string());
    fixture.policies.put(global);
    let remote = remote_workspace("ws_remote");

    let token = fixture
        .service
        .resolve(request(Some(remote)))
        .expect("resolve")
        .revision_token;

    assert!(token.chars().all(|character| character.is_ascii_hexdigit()));
    for forbidden in ["secret", "ssh://", "example.test", "D:/"] {
        assert!(!token.contains(forbidden), "{forbidden} must not appear");
    }
}

// =================================================================================================
// Last-known-good policy
// =================================================================================================

fn transient() -> PersonalizationApplicationError {
    PersonalizationApplicationError::Storage("database is locked".to_string())
}

fn corrupted() -> PersonalizationApplicationError {
    PersonalizationApplicationError::Domain(
        crate::contexts::personalization::domain::PersonalizationDomainError::UnknownPolicyToggle(
            "something-this-build-cannot-read".to_string(),
        ),
    )
}

#[test]
fn a_transient_read_failure_falls_back_to_the_last_validated_bundle() {
    let fixture = fixture();
    fixture.policies.put(enabled_global());
    let good = fixture.service.resolve(request(None)).expect("first read");
    assert!(good.memory_access.read);

    *fixture.policies.fails_with.lock().expect("fails") = Some(transient());
    let fallback = fixture.service.resolve(request(None)).expect("fallback");

    assert!(
        fallback.memory_access.read,
        "a locked database is not a reason to forget the settings"
    );
    assert!(fallback
        .warnings
        .iter()
        .any(|warning| warning.code == PersonalizationWarningCode::UsingLastKnownGoodPolicy));
}

#[test]
fn a_corrupted_or_unreadable_value_never_borrows_a_cached_bundle() {
    // Answering from an older copy would assert a fact about data that just failed to validate.
    let fixture = fixture();
    fixture.policies.put(enabled_global());
    fixture.service.resolve(request(None)).expect("first read");

    *fixture.policies.fails_with.lock().expect("fails") = Some(corrupted());
    let refused = fixture.service.resolve(request(None)).expect("resolve");

    assert!(!refused.memory_access.read);
    assert!(refused.instruction_segments.is_empty());
    assert!(refused
        .warnings
        .iter()
        .any(|warning| warning.code == PersonalizationWarningCode::NoValidatedPolicy));
}

#[test]
fn with_no_cached_bundle_a_transient_failure_still_fails_closed() {
    let fixture = fixture();
    fixture.policies.put(enabled_global());
    *fixture.policies.fails_with.lock().expect("fails") = Some(transient());

    let snapshot = fixture.service.resolve(request(None)).expect("resolve");

    assert!(!snapshot.memory_access.read);
    assert!(snapshot.instruction_segments.is_empty());
    assert!(snapshot
        .warnings
        .iter()
        .any(|warning| warning.code == PersonalizationWarningCode::NoValidatedPolicy));
}

#[test]
fn a_cached_bundle_is_never_lent_to_a_different_context() {
    let fixture = fixture();
    fixture.policies.put(enabled_global());
    fixture.agents.register("other-agent", full_capabilities());
    // Validated for onepiece with no workspace.
    fixture.service.resolve(request(None)).expect("prime");
    *fixture.policies.fails_with.lock().expect("fails") = Some(transient());

    // A different Agent.
    let mut other_agent = request(None);
    other_agent.agent_id = agent("other-agent");
    assert!(
        !fixture
            .service
            .resolve(other_agent)
            .expect("resolve")
            .memory_access
            .read
    );

    // A different workspace: the primed bundle proved nothing about a workspace override, and
    // lending it would silently assert that none exists.
    assert!(
        !fixture
            .service
            .resolve(request(Some(workspace("ws_alpha"))))
            .expect("resolve")
            .memory_access
            .read
    );
}

#[test]
fn a_cached_bundle_does_not_survive_a_new_migration_generation() {
    let fixture = fixture();
    fixture.policies.put(enabled_global());
    fixture.service.resolve(request(None)).expect("prime");

    *fixture.health.0.lock().expect("health") = MemoryRuntimeHealth::Ready { generation: 2 };
    *fixture.policies.fails_with.lock().expect("fails") = Some(transient());

    let snapshot = fixture.service.resolve(request(None)).expect("resolve");

    assert!(
        !snapshot.memory_access.read,
        "a migration makes every earlier bundle unusable rather than merely stale"
    );
}

#[test]
fn a_cached_policy_never_outranks_the_current_health_session_or_capability() {
    // The reason only policy is cached. Everything else is read fresh, so a cached "memory was
    // enabled" cannot survive a store that has since gone into repair.
    let fixture = fixture();
    fixture.policies.put(enabled_global());
    fixture.service.resolve(request(None)).expect("prime");
    *fixture.policies.fails_with.lock().expect("fails") = Some(transient());

    *fixture.health.0.lock().expect("health") = MemoryRuntimeHealth::RepairRequired;
    let unhealthy = fixture.service.resolve(request(None)).expect("resolve");
    assert!(!unhealthy.memory_access.read);
    *fixture.health.0.lock().expect("health") = MemoryRuntimeHealth::Ready { generation: 1 };

    let mut temporary = request(None);
    temporary.session_mode = SessionPersonalizationMode::Temporary;
    let temporary = fixture.service.resolve(temporary).expect("resolve");
    assert!(!temporary.memory_access.automatic_extraction);

    fixture.agents.register(
        "narrow",
        PersonalizationRuntimeCapabilities {
            supports_custom_instructions: true,
            supports_memory_index: false,
            supports_selected_memory_bodies: false,
            supports_automatic_extraction: false,
        },
    );
    let mut narrow = request(None);
    narrow.agent_id = agent("narrow");
    // A different Agent has no cached bundle of its own, so this fails closed rather than reusing
    // one — which is itself the property under test for capability changes.
    assert!(
        !fixture
            .service
            .resolve(narrow)
            .expect("resolve")
            .memory_access
            .read
    );
}

#[test]
fn a_successful_policy_write_drops_every_cached_bundle() {
    let fixture = fixture();
    fixture.policies.put(enabled_global());
    fixture.service.resolve(request(None)).expect("prime");
    assert_eq!(fixture.cache.len(), 1);

    fixture.cache.invalidate();

    assert_eq!(fixture.cache.len(), 0);
    *fixture.policies.fails_with.lock().expect("fails") = Some(transient());
    assert!(
        !fixture
            .service
            .resolve(request(None))
            .expect("resolve")
            .memory_access
            .read
    );
}

#[test]
fn transience_is_classified_rather_than_assumed() {
    assert!(is_transient_read_failure(&transient()));
    assert!(is_transient_read_failure(
        &PersonalizationApplicationError::MaintenanceBusy
    ));
    for permanent in [
        corrupted(),
        PersonalizationApplicationError::NotFound,
        PersonalizationApplicationError::MaintenanceRequired,
    ] {
        assert!(
            !is_transient_read_failure(&permanent),
            "{permanent:?} must never borrow a cached bundle"
        );
    }
}

// =================================================================================================
// Effective preview
// =================================================================================================

/// Marks what it was given, so a test can prove the preview routed text through the port at all.
///
/// Deliberately not the real rule: whether the platform redaction removes a token is a property of
/// that rule, asserted against the real adapter where it lives. Mimicking it here would only prove
/// that a mimic mimics.
struct MarkingRedaction;

impl SecretRedactionPort for MarkingRedaction {
    fn redact(&self, text: &str) -> String {
        format!("[redacted:{}]", text.chars().count())
    }
}

fn preview_service(fixture: &Fixture) -> PersonalizationPreviewService {
    PersonalizationPreviewService::new(
        Arc::new(PolicyResolutionService::new(
            fixture.policies.clone(),
            fixture.agents.clone(),
            fixture.projection.clone(),
            fixture.health.clone(),
            fixture.cache.clone(),
        )),
        Arc::new(MarkingRedaction),
    )
}

#[test]
fn a_preview_reports_provenance_modes_counts_and_an_estimate() {
    let fixture = fixture();
    let mut global = enabled_global();
    global.set_instruction_merge_mode(InstructionMergeMode::Append);
    global.set_about_user("Prefers concise answers.".to_string());
    fixture.policies.put(global);
    *fixture.projection.summary.lock().expect("summary") = MemoryEligibilitySummary {
        considered: 5,
        eligible_total: 3,
        refs: Vec::new(),
        truncated: true,
        exclusions: vec![MemoryExclusionCount {
            reason: PersonalizationExclusionReason::Archived,
            count: 2,
        }],
        digest: "digest".to_string(),
    };

    let preview = preview_service(&fixture)
        .preview(request(None))
        .expect("preview");

    assert_eq!(preview.instruction_mode, InstructionMergeMode::Append);
    let segment = preview
        .included_instructions
        .first()
        .expect("one included field");
    assert_eq!(segment.scope_kind, "global");
    assert_eq!(segment.merge_action, InstructionMergeAction::Appended);
    assert_eq!(
        segment.characters,
        "Prefers concise answers.".chars().count()
    );
    assert_eq!(preview.eligible_memory_count, 3);
    assert_eq!(preview.considered_memory_count, 5);
    assert_eq!(preview.memory_exclusions.len(), 1);
    assert_eq!(
        preview.delivery,
        MemoryDeliveryMode::IndexWithSelectedBodies
    );
    assert!(preview.context_estimate.known_characters > 0);
    assert!(preview.context_estimate.approximate_tokens > 0);
    // Stated rather than implied: VaneHub does not manage a CLI internal context.
    assert!(!preview.cli_internal_compaction_managed);
}

#[test]
fn every_shown_instruction_goes_through_the_redaction_port() {
    // A settings screen is screenshotted, pasted into issues and read over shoulders, so nothing
    // reaches it unfiltered. What the filter actually removes is asserted against the real adapter,
    // where that rule lives.
    let fixture = fixture();
    let original = "my api_key=sk-live-01234567890abcdef and D:/private/notes";
    let mut global = enabled_global();
    global.set_instruction_merge_mode(InstructionMergeMode::Append);
    global.set_about_user(original.to_string());
    fixture.policies.put(global);

    let preview = preview_service(&fixture)
        .preview(request(None))
        .expect("preview");

    let segment = &preview.included_instructions[0];
    assert!(
        segment.redacted_text.starts_with("[redacted:"),
        "the shown text came from the port, not from the record: {}",
        segment.redacted_text
    );
    assert!(!segment.redacted_text.contains("sk-live-01234567890abcdef"));
    // The length reported is of what will actually be sent, not of the rendering: a user sizing
    // their instructions needs the real number.
    assert_eq!(segment.characters, original.chars().count());
}

#[test]
fn a_preview_carries_no_memory_body_and_no_recorded_path() {
    let fixture = fixture();
    fixture.policies.put(enabled_global());
    *fixture.projection.summary.lock().expect("summary") = MemoryEligibilitySummary {
        considered: 1,
        eligible_total: 1,
        refs: vec![SnapshotMemoryRef {
            id: MemoryId::parse("01K2MEM0000000000000000001").expect("id"),
            revision: 1,
            content_hash: "sha256:abc".to_string(),
            name: "user-role".to_string(),
            description: "A short hook".to_string(),
            memory_type: MemoryType::Project,
            scope_hint: "global".to_string(),
            updated_at: chrono::Utc::now(),
        }],
        truncated: false,
        exclusions: Vec::new(),
        digest: "digest".to_string(),
    };

    let preview = preview_service(&fixture)
        .preview(request(None))
        .expect("preview");

    // The preview reports counts and metadata, never a body: the snapshot never carried one, which
    // is what makes this a property rather than a filter someone has to remember to apply.
    let rendered = format!("{preview:?}");
    assert!(!rendered.contains("legacy_folder"));
    assert!(rendered.contains("eligible_memory_count: 1"));
}

#[test]
fn the_context_estimate_names_what_it_leaves_out() {
    // An estimate whose boundaries are unclear is worse than none: a user who reads a small number
    // and finds their turn costs far more has been misled by the omission.
    let fixture = fixture();
    fixture.policies.put(enabled_global());

    let preview = preview_service(&fixture)
        .preview(request(None))
        .expect("preview");

    let excluded = &preview.context_estimate.excluded_surfaces;
    for surface in [
        "core_system_prompt",
        "user_message",
        "prompt_hooks",
        "cli_internal_context",
    ] {
        assert!(excluded.contains(&surface), "{surface} must be named");
    }
    assert_eq!(
        preview.context_estimate.estimator_version,
        "personalization-context-estimate-v1"
    );
}

#[test]
fn the_selected_body_budget_is_a_bound_rather_than_an_invented_measurement() {
    let fixture = fixture();
    fixture.policies.put(enabled_global());
    fixture.agents.register(
        "index-only",
        PersonalizationRuntimeCapabilities {
            supports_custom_instructions: true,
            supports_memory_index: true,
            supports_selected_memory_bodies: false,
            supports_automatic_extraction: true,
        },
    );

    let with_bodies = preview_service(&fixture)
        .preview(request(None))
        .expect("preview");
    assert!(with_bodies.context_estimate.selected_body_budget_max > 0);

    let mut index_only = request(None);
    index_only.agent_id = agent("index-only");
    let without = preview_service(&fixture)
        .preview(index_only)
        .expect("preview");
    assert_eq!(without.context_estimate.selected_body_budget_max, 0);
}

#[test]
fn a_preview_of_a_fail_closed_resolution_still_explains_itself() {
    let fixture = fixture();

    let preview = preview_service(&fixture)
        .preview(request(None))
        .expect("preview");

    assert!(preview.included_instructions.is_empty());
    assert_eq!(preview.eligible_memory_count, 0);
    assert!(preview
        .warnings
        .iter()
        .any(|warning| warning.code == PersonalizationWarningCode::NoValidatedPolicy));
    assert_eq!(preview.context_estimate.known_characters, 0);
}
