use super::*;
use crate::contexts::tooling::prompt_hooks::api::PromptHookApi;
use crate::contexts::tooling::prompt_hooks::domain::{
    ManagedCliAgentId, PromptHookBindings, PromptHookCategory, PromptHookDomainError, PromptHookId,
    PromptHookManifest, PromptHookSource, PromptHookStage,
};
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

#[derive(Default)]
struct RepositoryState {
    users: BTreeMap<PromptHookId, PromptHookRecord>,
    overrides: BTreeMap<PromptHookId, PromptHookOverride>,
    traces: Vec<PromptHookTrace>,
    trace_query_limits: Vec<usize>,
    calls: Vec<String>,
}

#[derive(Default)]
struct FakeRepository {
    state: Mutex<RepositoryState>,
    next_write_failure: Mutex<Option<String>>,
}

impl FakeRepository {
    fn insert_user(&self, record: PromptHookRecord) {
        self.state
            .lock()
            .expect("repository state")
            .users
            .insert(record.id().clone(), record);
    }

    fn insert_override(&self, override_record: PromptHookOverride) {
        self.state
            .lock()
            .expect("repository state")
            .overrides
            .insert(override_record.hook_id.clone(), override_record);
    }

    fn user(&self, hook_id: &PromptHookId) -> Option<PromptHookRecord> {
        self.state
            .lock()
            .expect("repository state")
            .users
            .get(hook_id)
            .cloned()
    }

    fn fail_next_write(&self, message: &str) {
        *self.next_write_failure.lock().expect("write failure") = Some(message.to_string());
    }

    fn check_write(&self) -> Result<(), PromptHookApplicationError> {
        match self
            .next_write_failure
            .lock()
            .expect("write failure")
            .take()
        {
            Some(message) => Err(PromptHookApplicationError::Repository(message)),
            None => Ok(()),
        }
    }
}

impl PromptHookRepository for FakeRepository {
    fn list_user_hooks(&self) -> Result<Vec<PromptHookRecord>, PromptHookApplicationError> {
        Ok(self
            .state
            .lock()
            .expect("repository state")
            .users
            .values()
            .cloned()
            .collect())
    }

    fn list_builtin_overrides(
        &self,
    ) -> Result<Vec<PromptHookOverride>, PromptHookApplicationError> {
        Ok(self
            .state
            .lock()
            .expect("repository state")
            .overrides
            .values()
            .cloned()
            .collect())
    }

    fn create_user_hook(
        &self,
        record: &PromptHookRecord,
    ) -> Result<(), PromptHookApplicationError> {
        self.check_write()?;
        let mut state = self.state.lock().expect("repository state");
        state.calls.push(format!("create:{}", record.id().as_str()));
        state.users.insert(record.id().clone(), record.clone());
        Ok(())
    }

    fn update_user_hook(
        &self,
        record: &PromptHookRecord,
    ) -> Result<(), PromptHookApplicationError> {
        self.check_write()?;
        let mut state = self.state.lock().expect("repository state");
        state.calls.push(format!("update:{}", record.id().as_str()));
        state.users.insert(record.id().clone(), record.clone());
        Ok(())
    }

    fn delete_user_hook(&self, hook_id: &PromptHookId) -> Result<(), PromptHookApplicationError> {
        self.check_write()?;
        let mut state = self.state.lock().expect("repository state");
        state.calls.push(format!("delete:{}", hook_id.as_str()));
        state.users.remove(hook_id);
        Ok(())
    }

    fn set_user_enabled(
        &self,
        hook_id: &PromptHookId,
        enabled: bool,
        updated_at: &str,
    ) -> Result<(), PromptHookApplicationError> {
        self.check_write()?;
        let mut state = self.state.lock().expect("repository state");
        state
            .calls
            .push(format!("enable:{}:{enabled}", hook_id.as_str()));
        let record = state.users.get_mut(hook_id).expect("user hook");
        record.enabled = enabled;
        record.updated_at = updated_at.to_string();
        Ok(())
    }

    fn set_user_bindings(
        &self,
        hook_id: &PromptHookId,
        bindings: &PromptHookBindings,
        updated_at: &str,
    ) -> Result<(), PromptHookApplicationError> {
        self.check_write()?;
        let mut state = self.state.lock().expect("repository state");
        state.calls.push(format!("bindings:{}", hook_id.as_str()));
        let record = state.users.get_mut(hook_id).expect("user hook");
        record.manifest = record.manifest.clone().with_bindings(bindings.clone());
        record.updated_at = updated_at.to_string();
        Ok(())
    }

    fn save_builtin_override(
        &self,
        override_record: &PromptHookOverride,
    ) -> Result<(), PromptHookApplicationError> {
        self.check_write()?;
        let mut state = self.state.lock().expect("repository state");
        state.calls.push(format!(
            "override:{}:{}",
            override_record.hook_id.as_str(),
            override_record.enabled
        ));
        state
            .overrides
            .insert(override_record.hook_id.clone(), override_record.clone());
        Ok(())
    }

    fn save_traces(
        &self,
        traces: &[PromptHookTrace],
        retained_limit: usize,
    ) -> Result<(), PromptHookApplicationError> {
        self.check_write()?;
        let mut state = self.state.lock().expect("repository state");
        state.calls.push(format!("traces:{}", traces.len()));
        state.traces.extend_from_slice(traces);
        let excess = state.traces.len().saturating_sub(retained_limit);
        if excess > 0 {
            state.traces.drain(0..excess);
        }
        Ok(())
    }

    fn list_traces(
        &self,
        limit: usize,
    ) -> Result<Vec<PromptHookTrace>, PromptHookApplicationError> {
        let mut state = self.state.lock().expect("repository state");
        state.trace_query_limits.push(limit);
        Ok(state.traces.iter().rev().take(limit).cloned().collect())
    }
}

struct FixedClock;

impl PromptHookClockPort for FixedClock {
    fn now(&self) -> String {
        "2026-07-18T12:00:00Z".to_string()
    }
}

#[derive(Default)]
struct SequenceTraceIds {
    next: Mutex<usize>,
}

impl PromptHookTraceIdPort for SequenceTraceIds {
    fn next_trace_id(&self) -> String {
        let mut next = self.next.lock().expect("trace id");
        *next += 1;
        format!("prompt-hook-trace-{next}")
    }
}

#[derive(Default)]
struct FakeLogging {
    events: Mutex<Vec<PromptHookLogEvent>>,
}

impl PromptHookLoggingPort for FakeLogging {
    fn record(&self, event: &PromptHookLogEvent) {
        self.events.lock().expect("log events").push(event.clone());
    }
}

struct Fixture {
    service: PromptHookApplicationService,
    repository: Arc<FakeRepository>,
    logging: Arc<FakeLogging>,
}

impl Fixture {
    fn new() -> Self {
        let repository = Arc::new(FakeRepository::default());
        let logging = Arc::new(FakeLogging::default());
        Self {
            service: PromptHookApplicationService::new(
                repository.clone(),
                Arc::new(FixedClock),
                Arc::new(SequenceTraceIds::default()),
                logging.clone(),
            ),
            repository,
            logging,
        }
    }
}

fn id(value: &str) -> PromptHookId {
    PromptHookId::parse(value).expect("Prompt Hook id")
}

fn bindings(values: &[&str]) -> PromptHookBindings {
    PromptHookBindings::new(
        &values
            .iter()
            .map(|value| (*value).to_string())
            .collect::<Vec<_>>(),
    )
    .expect("bindings")
}

fn governance() -> PromptHookGovernance {
    PromptHookGovernance {
        safety_tier: "editable".to_string(),
        transparency_tier: "visible-by-default".to_string(),
        governance_tier: "human-gated".to_string(),
    }
}

fn manifest(
    value: &str,
    order: i64,
    binding_values: &[&str],
    template: &str,
) -> PromptHookManifest {
    PromptHookManifest::new(
        value,
        format!("Name {value}"),
        PromptHookCategory::Dynamic,
        PromptHookStage::PerTurn,
        order,
        template,
        &binding_values
            .iter()
            .map(|value| (*value).to_string())
            .collect::<Vec<_>>(),
    )
    .expect("manifest")
}

fn user_record(value: &str, order: i64) -> PromptHookRecord {
    PromptHookRecord {
        manifest: manifest(value, order, &["codex-cli"], "User {{sampleInput}}"),
        description: "User hook".to_string(),
        version: 1,
        source: PromptHookSource::User,
        enabled: true,
        disableable: true,
        governance: governance(),
        created_at: "2026-07-17T00:00:00Z".to_string(),
        updated_at: "2026-07-17T00:00:00Z".to_string(),
    }
}

fn create_request(value: &str, order: i64) -> PromptHookCreateRequest {
    PromptHookCreateRequest {
        manifest: manifest(value, order, &["codex-cli"], "User {{sampleInput}}"),
        description: "  User description  ".to_string(),
        enabled: true,
        governance: governance(),
    }
}

#[test]
fn listing_merges_catalog_overrides_and_users_with_stable_stats_and_order() {
    let fixture = Fixture::new();
    fixture
        .repository
        .insert_user(user_record("user-focus", 450));
    fixture.repository.insert_override(PromptHookOverride {
        hook_id: id("dynamic-session-config"),
        enabled: false,
        bindings: bindings(&["codex-cli"]),
        updated_at: "2026-07-18T10:00:00Z".to_string(),
    });

    let result = fixture.service.list_hooks().expect("hooks");

    assert_eq!(result.stats.total, 8);
    assert_eq!(result.stats.builtin, 7);
    assert_eq!(result.stats.user, 1);
    assert_eq!(result.stats.enabled, 6);
    assert_eq!(result.hooks[2].id().as_str(), "user-focus");
    let dynamic = result
        .hooks
        .iter()
        .find(|hook| hook.id().as_str() == "dynamic-session-config")
        .expect("dynamic hook");
    assert!(!dynamic.enabled);
    assert_eq!(dynamic.manifest.bindings().to_strings(), ["codex-cli"]);
}

#[test]
fn create_persists_normalized_record_and_rejects_an_occupied_order_slot() {
    let fixture = Fixture::new();

    let created = fixture
        .service
        .create_hook(create_request("user-focus", 450))
        .expect("create");

    assert_eq!(created.description, "User description");
    assert_eq!(created.created_at, "2026-07-18T12:00:00Z");
    assert_eq!(fixture.repository.user(&id("user-focus")), Some(created));
    let error = fixture
        .service
        .create_hook(create_request("other-focus", 450))
        .expect_err("duplicate order");
    assert_eq!(
        error,
        PromptHookApplicationError::Domain(PromptHookDomainError::DuplicateOrder)
    );
    let events = fixture.logging.events.lock().expect("log events");
    assert_eq!(events[0].level, PromptHookLogLevel::Info);
    assert_eq!(events[1].level, PromptHookLogLevel::Error);
}

#[test]
fn update_enforces_identity_and_builtin_content_immutability_before_writing() {
    let fixture = Fixture::new();
    fixture
        .repository
        .insert_user(user_record("user-focus", 450));
    let identity_error = fixture
        .service
        .update_hook(PromptHookUpdateRequest {
            hook_id: id("user-focus"),
            manifest: manifest("other-focus", 451, &["codex-cli"], "updated"),
            description: "updated".to_string(),
            version: 2,
            enabled: true,
            governance: governance(),
        })
        .expect_err("immutable identity");
    assert_eq!(
        identity_error,
        PromptHookApplicationError::Domain(PromptHookDomainError::IdentityChanged)
    );

    let builtin_error = fixture
        .service
        .update_hook(PromptHookUpdateRequest {
            hook_id: id("dynamic-session-config"),
            manifest: manifest("dynamic-session-config", 401, &["codex-cli"], "updated"),
            description: "updated".to_string(),
            version: 2,
            enabled: true,
            governance: governance(),
        })
        .expect_err("immutable builtin");
    assert_eq!(
        builtin_error,
        PromptHookApplicationError::Domain(PromptHookDomainError::BuiltinContentImmutable)
    );
    assert!(fixture
        .repository
        .state
        .lock()
        .expect("repository state")
        .calls
        .is_empty());
}

#[test]
fn successful_update_preserves_creation_time_and_persists_the_validated_manifest() {
    let fixture = Fixture::new();
    fixture
        .repository
        .insert_user(user_record("user-focus", 450));

    let updated = fixture
        .service
        .update_hook(PromptHookUpdateRequest {
            hook_id: id("user-focus"),
            manifest: manifest("user-focus", 451, &["gemini-cli"], "Updated {{agentId}}"),
            description: "  Updated description  ".to_string(),
            version: 2,
            enabled: false,
            governance: governance(),
        })
        .expect("update");

    assert_eq!(updated.description, "Updated description");
    assert_eq!(updated.version, 2);
    assert_eq!(updated.created_at, "2026-07-17T00:00:00Z");
    assert_eq!(updated.updated_at, "2026-07-18T12:00:00Z");
    assert_eq!(updated.manifest.bindings().to_strings(), ["gemini-cli"]);
    assert_eq!(fixture.repository.user(&id("user-focus")), Some(updated));
    assert_eq!(
        fixture
            .repository
            .state
            .lock()
            .expect("repository state")
            .calls,
        ["update:user-focus"]
    );
}

#[test]
fn delete_enablement_and_bindings_route_by_source_and_preserve_builtin_policy() {
    let fixture = Fixture::new();
    fixture
        .repository
        .insert_user(user_record("user-focus", 450));

    let immutable = fixture
        .service
        .set_enabled(id("law-runtime-boundary"), false)
        .expect_err("immutable builtin");
    assert_eq!(
        immutable,
        PromptHookApplicationError::Domain(PromptHookDomainError::CannotBeDisabled)
    );
    let disabled = fixture
        .service
        .set_enabled(id("user-focus"), false)
        .expect("disable user");
    assert!(!disabled.enabled);
    let rebound = fixture
        .service
        .set_bindings(id("navigation-project-hints"), bindings(&["gemini-cli"]))
        .expect("bind builtin");
    assert_eq!(rebound.manifest.bindings().to_strings(), ["gemini-cli"]);
    let delete_builtin = fixture
        .service
        .delete_hook(id("static-response-format"))
        .expect_err("delete builtin");
    assert_eq!(
        delete_builtin,
        PromptHookApplicationError::Domain(PromptHookDomainError::BuiltinCannotBeDeleted)
    );
    fixture
        .service
        .delete_hook(id("user-focus"))
        .expect("delete user");
    assert!(fixture.repository.user(&id("user-focus")).is_none());
    let calls = &fixture
        .repository
        .state
        .lock()
        .expect("repository state")
        .calls;
    assert!(calls.iter().any(|call| call == "enable:user-focus:false"));
    assert!(calls
        .iter()
        .any(|call| call == "override:navigation-project-hints:true"));
}

#[test]
fn preview_renders_with_deterministic_trace_and_persists_no_raw_content() {
    let fixture = Fixture::new();

    let preview = fixture
        .service
        .preview_hook(PromptHookPreviewRequest {
            hook_id: id("dynamic-session-config"),
            agent_id: ManagedCliAgentId::CodexCli,
            sample_input: Some("secret request".to_string()),
        })
        .expect("preview");

    assert!(preview.rendered_content.contains("codex-cli"));
    assert_eq!(preview.trace[0].id, "prompt-hook-trace-1");
    assert_eq!(preview.trace[0].status, PromptHookTraceStatus::Fired);
    assert!(preview.trace[0].content_hash.is_some());
    assert_eq!(preview.trace[0].created_at, "2026-07-18T12:00:00Z");
    let event = fixture.logging.events.lock().expect("log events")[0].clone();
    assert_eq!(event.action, PromptHookLogAction::Preview);
    assert!(!event.message.contains("secret request"));
}

#[test]
fn assembly_skips_disabled_and_unbound_hooks_and_publishes_effective_prompt_api() {
    let fixture = Fixture::new();
    fixture
        .repository
        .insert_user(user_record("user-focus", 450));
    fixture.repository.insert_override(PromptHookOverride {
        hook_id: id("navigation-project-hints"),
        enabled: true,
        bindings: bindings(&["gemini-cli"]),
        updated_at: "2026-07-18T10:00:00Z".to_string(),
    });
    let api = PromptHookApi::new(fixture.service.clone());

    let result = api
        .effective_prompt("codex-cli", Some("session-1"), "ship secret")
        .expect("effective prompt");

    assert!(result.effective_prompt.contains("User ship secret"));
    assert!(result.effective_prompt.ends_with("ship secret"));
    assert_eq!(result.trace.len(), 8);
    assert!(result.trace.iter().any(|trace| {
        trace.hook_id.as_str() == "navigation-project-hints"
            && trace.status == PromptHookTraceStatus::Skipped
            && trace.reason.as_deref() == Some("unbound-cli")
    }));
    assert!(result.trace.iter().any(|trace| {
        trace.hook_id.as_str() == "callback-future-channel"
            && trace.status == PromptHookTraceStatus::Disabled
    }));
    assert_eq!(
        fixture
            .repository
            .state
            .lock()
            .expect("repository state")
            .traces
            .len(),
        8
    );
    let event = fixture.logging.events.lock().expect("log events")[0].clone();
    assert_eq!(event.action, PromptHookLogAction::Assemble);
    assert!(!event.message.contains("ship secret"));
}

#[test]
fn trace_queries_are_bounded_before_reaching_the_repository() {
    let fixture = Fixture::new();

    fixture.service.list_traces(-10).expect("lower bound");
    fixture.service.list_traces(500).expect("upper bound");

    assert_eq!(
        fixture
            .repository
            .state
            .lock()
            .expect("repository state")
            .trace_query_limits,
        [1, 100]
    );
}

#[test]
fn repository_failure_is_returned_and_logged_without_exposing_prompt_content() {
    let fixture = Fixture::new();
    fixture.repository.fail_next_write("database unavailable");

    let error = fixture
        .service
        .create_hook(create_request("user-focus", 450))
        .expect_err("repository failure");

    assert_eq!(
        error,
        PromptHookApplicationError::Repository("database unavailable".to_string())
    );
    let event = fixture.logging.events.lock().expect("log events")[0].clone();
    assert_eq!(event.level, PromptHookLogLevel::Error);
    assert_eq!(event.hook_id.as_deref(), Some("user-focus"));
    assert!(!event.message.contains("User {{sampleInput}}"));
}
