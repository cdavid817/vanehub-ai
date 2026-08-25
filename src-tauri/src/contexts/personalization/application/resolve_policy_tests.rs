//! Scoped policy resolution and the instruction merge state machine.
//!
//! Fakes for the three ports, because every property here is about precedence and provenance rather
//! than about storage. The consistent-read contract — that a bundle reports `Absent` rather than
//! omitting a key — is asserted against the real SQLite repository where it lives.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use super::error::PersonalizationApplicationError;
use super::ports::{AgentCapabilityPort, MemoryHealthPort, PolicyRepository};
use super::resolve_policy::{PolicyResolutionService, ResolutionRequest};
use crate::contexts::personalization::domain::{
    AgentId, InstructionExclusionReason, InstructionField, InstructionMergeAction,
    InstructionMergeMode, MemoryRuntimeHealth, PatchPolicyResult, PersonalizationLayers,
    PersonalizationPolicyPatch, PersonalizationPolicyRecord, PersonalizationPolicyScope,
    PersonalizationRuntimeCapabilities, PersonalizationWarningCode, PolicyLayerState,
    PolicyResolutionBundle, PolicyToggle, SessionId, SessionPersonalizationMode, WorkspaceIdentity,
    WorkspaceKey, WorkspaceKind,
};

type Result<T> = std::result::Result<T, PersonalizationApplicationError>;

/// Stored rows, keyed by scope, with the reads counted so "one consistent read" is assertable.
#[derive(Default)]
struct FakePolicies {
    rows: Mutex<Vec<PersonalizationPolicyRecord>>,
    bundle_reads: AtomicUsize,
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
    health: Arc<FixedHealth>,
    service: PolicyResolutionService,
}

fn fixture() -> Fixture {
    let policies = Arc::new(FakePolicies::default());
    let agents = Arc::new(FakeAgents::default());
    let health = Arc::new(FixedHealth(Mutex::new(MemoryRuntimeHealth::Ready {
        generation: 1,
    })));
    agents.register("onepiece", full_capabilities());
    let service = PolicyResolutionService::new(policies.clone(), agents.clone(), health.clone());
    Fixture {
        policies,
        agents,
        health,
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
