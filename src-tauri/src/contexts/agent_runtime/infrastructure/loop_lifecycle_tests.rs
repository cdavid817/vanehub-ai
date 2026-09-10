//! Acceptance SC-16: a complete strict Loop on the real native stack. Persistence is the
//! SQLite repository, the boundary is the handle-relative platform over a real Git worktree,
//! verification is the in-process `patch-whitespace` check, and acceptance seals real contents.
//! Only the two model outputs are fixtures: the Worker fixture writes through the mediated
//! guard exactly as the file tools do, and the Verifier fixture proves it cannot mutate.

#![cfg(unix)]

use super::loop_generation_completions::InMemoryLoopRoleGenerationCompletions;
use super::loop_repository::SqliteLoopRepository;
use super::loop_scope_platform::{CatalogLoopCliCapability, NativeLoopScopePlatform};
use super::loop_verification_process::StructuredLoopVerificationProcess;
use crate::contexts::agent_runtime::application::{
    AgentClockPort, AgentOperation, AgentRegistryRepository, AgentRuntimeApplicationError,
    AgentTaskPort, ApiAgentGateway, ApiProviderConfig, LoopAcceptanceApplicationPorts,
    LoopAcceptanceApplicationService, LoopApplicationPorts, LoopApplicationService,
    LoopAssessmentService, LoopBackgroundPort, LoopBindingStatus, LoopControlEnvelope,
    LoopGenerationControlPort, LoopGitStatePort, LoopGitStateView, LoopGuardRole, LoopLog,
    LoopLoggingPort, LoopOperationContext, LoopOperationObserver,
    LoopOrchestratorApplicationService, LoopOrchestratorPorts, LoopProgressApplicationService,
    LoopProjectPort, LoopRepository, LoopRoleGenerationCompletionPort, LoopRoleGenerationOutcome,
    LoopRoleGenerationTerminal, LoopRoleSessionPort, LoopRoleSessionRequest, LoopScopePlatformPort,
    LoopVerificationApplicationPorts, LoopVerificationApplicationService,
    LoopVerificationCancellation, LoopVerificationEvidenceFact, LoopVerificationEvidencePort,
    LoopVerifierApplicationPorts, LoopVerifierApplicationService, LoopVerifierContextPort,
    LoopVerifierGenerationPort, LoopWorkerApplicationPorts, LoopWorkerApplicationService,
    LoopWorkerGenerationPort, PreparedLoopWorktree, RegisterApiAgentInput,
    RequestLoopAcceptanceRequest, SaveLoopDefinitionRequest, UpdateApiAgentInput,
};
use crate::contexts::agent_runtime::domain::{
    AgentAvailability, AgentDefinition, AgentDefinitionInput, AvailabilityAssessment,
    InteractionMode, LaunchMetadata, LoopLimits, LoopRequestedMode, LoopRunStatus,
    LoopSideEffectChannel, LoopTerminalReason, LoopVerificationCommand, LoopVerificationKind,
    NATIVE_CHECK_PATCH_WHITESPACE,
};
use crate::platform::database::NativeDatabase;
use crate::test_support::TempDirectory;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, Mutex};

const API_AGENT: &str = "loop-lifecycle-api";
const CLI_AGENT: &str = "claude-code";
const WORKER_CONTENT: &str = "export const answer = 42;\n";

fn git(root: &Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(args)
        .env("GIT_AUTHOR_NAME", "Loop Test")
        .env("GIT_AUTHOR_EMAIL", "loop@test.invalid")
        .env("GIT_COMMITTER_NAME", "Loop Test")
        .env("GIT_COMMITTER_EMAIL", "loop@test.invalid")
        .output()
        .expect("git available");
    assert!(
        output.status.success(),
        "git {args:?} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).trim().to_string()
}

fn sentinel(label: &str) -> String {
    format!(
        "SENTINEL-{label}-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos()
    )
}

/// One role session the fixture created, with the backend-issued scope reference it received.
struct RoleSession {
    run_id: String,
    iteration_id: String,
    role: LoopGuardRole,
}

struct LifecycleWorld {
    database: NativeDatabase,
    repository: Arc<SqliteLoopRepository>,
    platform: Arc<NativeLoopScopePlatform>,
    completions: Arc<InMemoryLoopRoleGenerationCompletions>,
    project: PathBuf,
    worktrees: PathBuf,
    sessions: Mutex<Vec<(String, RoleSession)>>,
    operations: Mutex<u32>,
    logs: Mutex<Vec<LoopLog>>,
    refusals: Mutex<Vec<String>>,
    outside_sentinel: String,
    protected_sentinel: String,
}

impl LifecycleWorld {
    fn guard(
        &self,
        run_id: &str,
        role: LoopGuardRole,
    ) -> Arc<dyn crate::contexts::agent_runtime::application::LoopScopeGuard> {
        let binding = self
            .repository
            .find_run_scope(run_id)
            .expect("scope record")
            .expect("run")
            .binding
            .expect("binding persisted before any role effect");
        self.platform.guard(&binding, role).expect("guard")
    }

    fn session(&self, session_id: &str) -> (String, String, LoopGuardRole) {
        let sessions = self.sessions.lock().expect("sessions");
        let (_, session) = sessions
            .iter()
            .find(|(id, _)| id == session_id)
            .expect("known role session");
        (
            session.run_id.clone(),
            session.iteration_id.clone(),
            session.role,
        )
    }

    fn observer(self: &Arc<Self>) -> LoopOperationObserver {
        LoopOperationObserver::new(self.clone(), self.clone(), self.clone())
    }

    fn assessment(self: &Arc<Self>) -> LoopAssessmentService {
        LoopAssessmentService::new(
            self.clone(),
            self.clone(),
            Arc::new(CatalogLoopCliCapability),
            self.platform.clone(),
            self.clone(),
        )
    }

    fn admission(self: &Arc<Self>) -> LoopApplicationService {
        LoopApplicationService::new(LoopApplicationPorts {
            loops: self.repository.clone(),
            registry: self.clone(),
            api_agents: self.clone(),
            projects: self.clone(),
            observer: self.observer(),
            clock: self.clone(),
            assessment: self.assessment(),
            app_epoch: "lifecycle-epoch".to_string(),
        })
    }

    fn orchestrator(self: &Arc<Self>) -> LoopOrchestratorApplicationService {
        let observer = self.observer();
        LoopOrchestratorApplicationService::new(LoopOrchestratorPorts {
            loops: self.repository.clone(),
            iterations: self.repository.clone(),
            projects: self.clone(),
            verifier_context: self.clone(),
            completions: self.completions.clone(),
            generations: self.clone(),
            worker: LoopWorkerApplicationService::new(LoopWorkerApplicationPorts {
                iterations: self.repository.clone(),
                registry: self.clone(),
                roles: self.clone(),
                git: self.clone(),
                generations: self.clone(),
                clock: self.clone(),
            }),
            verification: LoopVerificationApplicationService::new(
                LoopVerificationApplicationPorts {
                    iterations: self.repository.clone(),
                    processes: Arc::new(StructuredLoopVerificationProcess::default()),
                    observer: observer.clone(),
                    clock: self.clone(),
                    evidence: self.clone(),
                    scope_platform: self.platform.clone(),
                },
            ),
            verifier: LoopVerifierApplicationService::new(LoopVerifierApplicationPorts {
                iterations: self.repository.clone(),
                registry: self.clone(),
                roles: self.clone(),
                context: self.clone(),
                generations: self.clone(),
            }),
            progress: LoopProgressApplicationService::new(self.repository.clone()),
            observer,
            clock: self.clone(),
            scope_platform: self.platform.clone(),
            assessment: self.assessment(),
        })
    }

    fn acceptance(self: &Arc<Self>) -> LoopAcceptanceApplicationService {
        LoopAcceptanceApplicationService::new(LoopAcceptanceApplicationPorts {
            loops: self.repository.clone(),
            iterations: self.repository.clone(),
            generations: self.clone(),
            scope_platform: self.platform.clone(),
            background: Arc::new(InlineBackground),
            observer: self.observer(),
            clock: self.clone(),
        })
    }
}

struct InlineBackground;

impl LoopBackgroundPort for InlineBackground {
    fn spawn(
        &self,
        _: &str,
        task: Box<dyn FnOnce() + Send + 'static>,
    ) -> Result<(), AgentRuntimeApplicationError> {
        task();
        Ok(())
    }
}

fn agent(id: &str, cli: bool) -> AgentDefinition {
    let launch = if cli {
        LaunchMetadata::new(
            "cli".to_string(),
            Some("claude".to_string()),
            None,
            Some("claude".to_string()),
        )
    } else {
        LaunchMetadata::new("api".to_string(), None, None, None)
    }
    .expect("launch");
    AgentDefinition::new(AgentDefinitionInput {
        id: id.to_string(),
        display_name: id.to_string(),
        provider: "test".to_string(),
        managed_sdk_dependency_id: None,
        launch,
        supported_interaction_modes: vec![if cli {
            InteractionMode::Cli
        } else {
            InteractionMode::Api
        }],
        availability: AvailabilityAssessment::new(AgentAvailability::Available, None),
        capability_tags: Vec::new(),
    })
    .expect("agent")
}

impl AgentRegistryRepository for LifecycleWorld {
    fn list(&self) -> Result<Vec<AgentDefinition>, AgentRuntimeApplicationError> {
        Ok(vec![agent(API_AGENT, false), agent(CLI_AGENT, true)])
    }
    fn find(
        &self,
        agent_id: &str,
    ) -> Result<Option<AgentDefinition>, AgentRuntimeApplicationError> {
        Ok(match agent_id {
            API_AGENT => Some(agent(API_AGENT, false)),
            CLI_AGENT => Some(agent(CLI_AGENT, true)),
            _ => None,
        })
    }
}

impl ApiAgentGateway for LifecycleWorld {
    fn register(
        &self,
        _: &str,
        _: &RegisterApiAgentInput,
    ) -> Result<AgentDefinition, AgentRuntimeApplicationError> {
        unreachable!()
    }
    fn provider_config(
        &self,
        _: &str,
    ) -> Result<Option<ApiProviderConfig>, AgentRuntimeApplicationError> {
        Ok(Some(ApiProviderConfig {
            source_provider_id: None,
            model_id: "fixture-model".to_string(),
            interface_format: "anthropic".to_string(),
            base_url: None,
            auto_approve_tools: true,
        }))
    }
    fn update(
        &self,
        _: &str,
        _: &UpdateApiAgentInput,
    ) -> Result<AgentDefinition, AgentRuntimeApplicationError> {
        unreachable!()
    }
    fn delete(&self, _: &str) -> Result<(), AgentRuntimeApplicationError> {
        unreachable!()
    }
}

impl LoopProjectPort for LifecycleWorld {
    fn validate_local_git_project(
        &self,
        project_path: &str,
    ) -> Result<String, AgentRuntimeApplicationError> {
        assert_eq!(Path::new(project_path), self.project.as_path());
        Ok(project_path.to_string())
    }
    fn prepare_loop_worktree(
        &self,
        project_path: &str,
        name: &str,
        base_branch: &str,
    ) -> Result<PreparedLoopWorktree, AgentRuntimeApplicationError> {
        let path = self.worktrees.join(format!("loop-{name}"));
        let branch = format!("loop/{name}");
        git(
            Path::new(project_path),
            &[
                "worktree",
                "add",
                "-b",
                &branch,
                &path.to_string_lossy(),
                base_branch,
            ],
        );
        Ok(PreparedLoopWorktree {
            path: path.to_string_lossy().to_string(),
            name: format!("loop-{name}"),
            branch,
        })
    }
    fn head_commit(
        &self,
        worktree_path: &str,
    ) -> Result<Option<String>, AgentRuntimeApplicationError> {
        Ok(Some(git(Path::new(worktree_path), &["rev-parse", "HEAD"])))
    }
}

impl LoopGitStatePort for LifecycleWorld {
    fn snapshot(&self, _: &str) -> Result<LoopGitStateView, AgentRuntimeApplicationError> {
        Ok(LoopGitStateView {
            branch: Some("main".to_string()),
            entries: Vec::new(),
            truncated: false,
        })
    }
}

impl LoopVerifierContextPort for LifecycleWorld {
    fn bounded_diff(&self, _: &str) -> Result<String, AgentRuntimeApplicationError> {
        Ok("diff --git a/src/app.ts b/src/app.ts\n+export const answer = 42;".to_string())
    }
}

impl LifecycleWorld {
    /// The production session gateway inserts the role session row; the fixture does the same so
    /// the iteration's foreign key points at a real session.
    fn insert_session_row(
        &self,
        id: &str,
        title: &str,
    ) -> Result<(), AgentRuntimeApplicationError> {
        let connection = self
            .database
            .connection()
            .map_err(|error| AgentRuntimeApplicationError::Loop(error.to_string()))?;
        connection
            .execute(
                r#"INSERT INTO sessions (
                    id, title, agent_id, interaction_mode, lifecycle_state,
                    pinned, archived, created_at, updated_at
                ) VALUES (?1, ?2, ?3, 'api', 'idle', 0, 0, ?4, ?4)"#,
                rusqlite::params![id, title, API_AGENT, self.now()],
            )
            .map_err(|error| AgentRuntimeApplicationError::Loop(error.to_string()))?;
        Ok(())
    }
}

impl LoopRoleSessionPort for LifecycleWorld {
    fn create_worker_session(
        &self,
        request: LoopRoleSessionRequest,
    ) -> Result<String, AgentRuntimeApplicationError> {
        let scope_ref = request
            .scope_ref
            .expect("worker sessions carry the backend scope ref");
        assert_eq!(scope_ref.role, LoopGuardRole::Worker);
        assert_eq!(
            scope_ref.requested_mode,
            LoopRequestedMode::PreventiveRequired
        );
        let id = format!("worker-session-{}", request.iteration_id);
        self.insert_session_row(&id, "Loop worker")?;
        self.sessions.lock().expect("sessions").push((
            id.clone(),
            RoleSession {
                run_id: request.run_id,
                iteration_id: request.iteration_id,
                role: LoopGuardRole::Worker,
            },
        ));
        Ok(id)
    }
    fn create_verifier_session(
        &self,
        request: LoopRoleSessionRequest,
    ) -> Result<String, AgentRuntimeApplicationError> {
        let scope_ref = request
            .scope_ref
            .expect("verifier sessions carry the backend scope ref");
        assert_eq!(scope_ref.role, LoopGuardRole::Verifier);
        let id = format!("verifier-session-{}", request.iteration_id);
        self.insert_session_row(&id, "Loop verifier")?;
        self.sessions.lock().expect("sessions").push((
            id.clone(),
            RoleSession {
                run_id: request.run_id,
                iteration_id: request.iteration_id,
                role: LoopGuardRole::Verifier,
            },
        ));
        Ok(id)
    }
}

/// The Worker "model": writes inside the scope through the mediated guard, then tries every
/// uncovered channel and every out-of-bound target, recording that each was refused.
impl LoopWorkerGenerationPort for LifecycleWorld {
    fn start_worker_generation(
        &self,
        session_id: &str,
        prompt: &str,
    ) -> Result<String, AgentRuntimeApplicationError> {
        assert!(prompt.contains("src"), "the prompt names the allowed scope");
        let (run_id, iteration_id, role) = self.session(session_id);
        assert_eq!(role, LoopGuardRole::Worker);
        let guard = self.guard(&run_id, LoopGuardRole::Worker);
        assert!(guard.mediated());
        guard
            .write("src/app.ts", WORKER_CONTENT.as_bytes())
            .expect("in-scope write is delivered");
        guard
            .write("src/lib/util.ts", b"export const util = true;\n")
            .expect("nested in-scope write creates parents");
        let mut refusals = self.refusals.lock().expect("refusals");
        for (target, content) in [
            ("docs/readme.md", "overwritten"),
            ("src/generated/out.js", "overwritten"),
            ("../escape.txt", "overwritten"),
            ("/etc/hostname", "overwritten"),
            (".git/HEAD", "ref: refs/heads/hijacked"),
        ] {
            let error = guard
                .write(target, content.as_bytes())
                .expect_err("out-of-bound write is refused");
            refusals.push(format!("write {target}: {error}"));
        }
        for channel in [
            LoopSideEffectChannel::Shell,
            LoopSideEffectChannel::Terminal,
            LoopSideEffectChannel::Mcp,
            LoopSideEffectChannel::CliInternal,
        ] {
            let error = guard
                .admit_channel(channel)
                .expect_err("uncovered channel is closed in strict mode");
            refusals.push(format!("channel {}: {error}", channel.as_str()));
        }
        self.completions.deliver(LoopRoleGenerationTerminal {
            run_id,
            iteration_id,
            role: "worker".to_string(),
            session_id: session_id.to_string(),
            message_id: "worker-message".to_string(),
            outcome: LoopRoleGenerationOutcome::Completed,
            content: Some("Implemented src/app.ts and src/lib/util.ts.".to_string()),
            error: None,
        })?;
        Ok("worker-message".to_string())
    }
}

/// The Verifier "model": proves it is read-only before recommending acceptance.
impl LoopVerifierGenerationPort for LifecycleWorld {
    fn start_verifier_generation(
        &self,
        session_id: &str,
        _: &str,
    ) -> Result<String, AgentRuntimeApplicationError> {
        let (run_id, iteration_id, role) = self.session(session_id);
        assert_eq!(role, LoopGuardRole::Verifier);
        let guard = self.guard(&run_id, LoopGuardRole::Verifier);
        assert_eq!(
            guard.read("src/app.ts").expect("read"),
            WORKER_CONTENT.as_bytes()
        );
        let mut refusals = self.refusals.lock().expect("refusals");
        let error = guard
            .write("src/app.ts", b"tampered by verifier")
            .expect_err("verifier writes are refused before any effect");
        refusals.push(format!("verifier write: {error}"));
        for channel in [
            LoopSideEffectChannel::MediatedFile,
            LoopSideEffectChannel::Shell,
            LoopSideEffectChannel::Terminal,
        ] {
            assert!(guard.admit_channel(channel).is_err());
        }
        self.completions.deliver(LoopRoleGenerationTerminal {
            run_id,
            iteration_id,
            role: "verifier".to_string(),
            session_id: session_id.to_string(),
            message_id: "verifier-message".to_string(),
            outcome: LoopRoleGenerationOutcome::Completed,
            content: Some(
                serde_json::json!({"recommendation": "pass", "findings": []}).to_string(),
            ),
            error: None,
        })?;
        Ok("verifier-message".to_string())
    }
}

impl LoopGenerationControlPort for LifecycleWorld {
    fn stop_loop_generation(&self, _: &str) -> Result<(), AgentRuntimeApplicationError> {
        Ok(())
    }
}

impl LoopVerificationEvidencePort for LifecycleWorld {
    fn project(&self, _: LoopVerificationEvidenceFact) {}
}

impl AgentTaskPort for LifecycleWorld {
    fn start_agent_launch(
        &self,
        _: &str,
        _: &str,
    ) -> Result<AgentOperation, AgentRuntimeApplicationError> {
        unreachable!()
    }
    fn start_agent_generation(
        &self,
        _: &str,
        _: &str,
        _: &str,
    ) -> Result<AgentOperation, AgentRuntimeApplicationError> {
        unreachable!()
    }
    fn start_loop_operation(
        &self,
        context: &LoopOperationContext,
        message: &str,
    ) -> Result<AgentOperation, AgentRuntimeApplicationError> {
        let mut operations = self.operations.lock().expect("operations");
        *operations += 1;
        Ok(AgentOperation {
            id: format!("lifecycle-operation-{operations}"),
            related_agent_id: Some(context.run_id.clone()),
            message: Some(message.to_string()),
        })
    }
    fn append_log(&self, _: &str, _: String) -> Result<(), AgentRuntimeApplicationError> {
        Ok(())
    }
    fn complete(&self, _: &str) -> Result<(), AgentRuntimeApplicationError> {
        Ok(())
    }
    fn fail(&self, _: &str, _: String) -> Result<(), AgentRuntimeApplicationError> {
        Ok(())
    }
    fn cancel(&self, _: &str) -> Result<(), AgentRuntimeApplicationError> {
        Ok(())
    }
}

impl LoopLoggingPort for LifecycleWorld {
    fn record_loop(&self, log: LoopLog) -> Result<(), AgentRuntimeApplicationError> {
        self.logs.lock().expect("logs").push(log);
        Ok(())
    }
}

impl AgentClockPort for LifecycleWorld {
    fn now(&self) -> String {
        chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
    }
}

struct Fixture {
    _directories: Vec<TempDirectory>,
    world: Arc<LifecycleWorld>,
    outside_file: PathBuf,
}

fn fixture(label: &str) -> Fixture {
    let project_directory = TempDirectory::new(&format!("{label}-project"));
    let worktrees = TempDirectory::new(&format!("{label}-worktrees"));
    let evidence = TempDirectory::new(&format!("{label}-evidence"));
    let data = TempDirectory::new(&format!("{label}-data"));
    let outside = TempDirectory::new(&format!("{label}-outside"));
    let outside_sentinel = sentinel("outside");
    let outside_file = outside.write("escape.txt", &outside_sentinel);
    let protected_sentinel = sentinel("generated");
    let project = std::fs::canonicalize(project_directory.path()).expect("canonical project");
    git(&project, &["init", "-q", "-b", "main"]);
    project_directory.write("src/app.ts", "export const answer = 1;\n");
    project_directory.write("src/generated/out.js", &protected_sentinel);
    project_directory.write("docs/readme.md", &sentinel("docs"));
    git(&project, &["add", "."]);
    git(&project, &["commit", "-q", "-m", "baseline"]);

    let database = NativeDatabase::new(data.path().to_path_buf()).expect("database");
    database
        .connection()
        .expect("connection")
        .execute(
            "INSERT INTO agents (id, display_name, provider, launch_kind) VALUES (?1, ?1, 'Test', 'api')",
            [API_AGENT],
        )
        .expect("seed api agent");
    let world = Arc::new(LifecycleWorld {
        database: database.clone(),
        repository: Arc::new(SqliteLoopRepository::new(database)),
        platform: Arc::new(NativeLoopScopePlatform::new(evidence.path().to_path_buf())),
        completions: Arc::new(InMemoryLoopRoleGenerationCompletions::default()),
        project,
        worktrees: std::fs::canonicalize(worktrees.path()).expect("canonical worktrees"),
        sessions: Mutex::new(Vec::new()),
        operations: Mutex::new(0),
        logs: Mutex::new(Vec::new()),
        refusals: Mutex::new(Vec::new()),
        outside_sentinel,
        protected_sentinel,
    });
    Fixture {
        _directories: vec![project_directory, worktrees, evidence, data, outside],
        world,
        outside_file,
    }
}

fn definition_request(world: &LifecycleWorld, verifier: &str) -> SaveLoopDefinitionRequest {
    SaveLoopDefinitionRequest {
        name: "Lifecycle".to_string(),
        enabled: true,
        project_path: world.project.to_string_lossy().to_string(),
        base_branch: "main".to_string(),
        goal: "Answer 42 in src/app.ts".to_string(),
        acceptance_criteria: vec!["src/app.ts exports 42".to_string()],
        allowed_paths: vec!["src".to_string()],
        protected_paths: vec!["src/generated".to_string()],
        worker_agent_id: API_AGENT.to_string(),
        verifier_agent_id: verifier.to_string(),
        verification_commands: vec![LoopVerificationCommand::new_with_kind(
            "whitespace".to_string(),
            LoopVerificationKind::NativeCheck,
            NATIVE_CHECK_PATCH_WHITESPACE.to_string(),
            Vec::new(),
            None,
            30,
            true,
        )
        .expect("native check")],
        limits: LoopLimits::new(2, 60, 600, 2, 2).expect("limits"),
        expected_version: None,
        scope_schema_version: Some(1),
        requested_mode: Some(LoopRequestedMode::PreventiveRequired),
    }
}

#[test]
fn strict_native_loop_runs_from_start_to_sealed_acceptance_with_zero_out_of_scope_effects() {
    let fixture = fixture("loop-lifecycle");
    let world = fixture.world.clone();
    let admission = world.admission();
    let definition = admission
        .create_definition(definition_request(&world, API_AGENT))
        .expect("definition");
    let readiness = admission.readiness(&definition.id).expect("readiness");
    assert!(
        readiness.ready,
        "strict readiness must pass on this host: {:?}",
        readiness.checks
    );
    let started = admission
        .start_manual(
            &definition.id,
            LoopControlEnvelope {
                expected_revision: Some(definition.version),
                idempotency_key: None,
                audit_acknowledgement_id: None,
            },
        )
        .expect("start");

    world
        .orchestrator()
        .execute(&started.run_id, LoopVerificationCancellation::default())
        .expect("orchestration");

    let awaiting = world
        .repository
        .find_run_view(&started.run_id)
        .expect("view")
        .expect("run");
    assert_eq!(
        awaiting.status,
        LoopRunStatus::AwaitingAcceptance,
        "{awaiting:?}"
    );
    let scope = awaiting.scope.clone().expect("scope view");
    assert_eq!(scope.binding_status, LoopBindingStatus::Bound);
    assert_eq!(
        scope.requested_mode,
        Some(LoopRequestedMode::PreventiveRequired)
    );
    assert!(scope.sealed_evidence_id.is_none());
    let worktree = PathBuf::from(awaiting.worktree_path.clone().expect("worktree"));
    assert_eq!(
        std::fs::read_to_string(worktree.join("src/app.ts")).expect("worker output"),
        WORKER_CONTENT
    );
    assert!(worktree.join("src/lib/util.ts").exists());
    assert_eq!(
        std::fs::read_to_string(worktree.join("src/generated/out.js")).expect("protected"),
        world.protected_sentinel
    );
    assert_eq!(
        std::fs::read_to_string(&fixture.outside_file).expect("outside"),
        world.outside_sentinel
    );
    assert!(!fixture.world.worktrees.join("escape.txt").exists());
    assert_eq!(
        git(&worktree, &["symbolic-ref", "HEAD"]),
        format!("refs/heads/loop/{}", started.run_id)
    );
    assert_eq!(
        std::fs::read_to_string(world.project.join("src/app.ts")).expect("project untouched"),
        "export const answer = 1;\n",
        "the Worker wrote only inside the isolated worktree"
    );

    let refusals = world.refusals.lock().expect("refusals").clone();
    assert_eq!(refusals.len(), 5 + 4 + 1, "{refusals:?}");
    assert!(
        refusals.iter().all(|line| line.contains("loop-scope")
            || line.contains("scope-")
            || line.contains("invalid path")),
        "{refusals:?}"
    );

    let iteration = awaiting.iterations.last().expect("iteration");
    let kinds: Vec<(&str, &str)> = iteration
        .evidence
        .iter()
        .map(|item| (item.kind.as_str(), item.status.as_str()))
        .collect();
    for expected in [
        ("scope-evidence", "passed"),
        ("verification-command", "passed"),
    ] {
        assert!(
            kinds.contains(&expected),
            "missing {expected:?} in {kinds:?}"
        );
    }
    assert_eq!(
        iteration
            .evidence
            .iter()
            .filter(|item| item.kind == "scope-evidence" && item.status == "passed")
            .count(),
        3,
        "worker, verification and verifier phases each seal complete scope evidence"
    );
    assert_eq!(iteration.verifier_recommendation.as_deref(), Some("pass"));
    // The binding row is run-level (it precedes the first iteration), so it is checked in the
    // persisted evidence table rather than through the iteration projection.
    let binding_rows: i64 = world
        .database
        .connection()
        .expect("connection")
        .query_row(
            "SELECT COUNT(*) FROM loop_evidence WHERE run_id = ?1 AND kind = 'scope-binding' AND status = 'passed' AND iteration_id IS NULL",
            [started.run_id.as_str()],
            |row| row.get(0),
        )
        .expect("binding evidence");
    assert_eq!(binding_rows, 1);

    // A stale revision or a foreign scope digest cannot accept.
    let acceptance = world.acceptance();
    assert!(acceptance
        .request(RequestLoopAcceptanceRequest {
            run_id: started.run_id.clone(),
            expected_revision: Some(awaiting.revision + 7),
            expected_scope_digest: None,
            expected_evidence_id: None,
            idempotency_key: None,
        })
        .is_err());
    assert!(acceptance
        .request(RequestLoopAcceptanceRequest {
            run_id: started.run_id.clone(),
            expected_revision: Some(awaiting.revision),
            expected_scope_digest: Some("sha256:not-this-scope".to_string()),
            expected_evidence_id: None,
            idempotency_key: None,
        })
        .is_err());
    let accepted = acceptance
        .request(RequestLoopAcceptanceRequest {
            run_id: started.run_id.clone(),
            expected_revision: Some(awaiting.revision),
            expected_scope_digest: scope.scope_digest.clone(),
            expected_evidence_id: None,
            idempotency_key: Some("accept-once".to_string()),
        })
        .expect("acceptance");
    assert_eq!(accepted.run_id, started.run_id);

    let succeeded = world
        .repository
        .find_run_view(&started.run_id)
        .expect("view")
        .expect("run");
    assert_eq!(succeeded.status, LoopRunStatus::Succeeded, "{succeeded:?}");
    assert_eq!(succeeded.terminal_reason, Some(LoopTerminalReason::GoalMet));
    let sealed = succeeded.scope.clone().expect("scope");
    let sealed_id = sealed
        .sealed_evidence_id
        .clone()
        .expect("sealed evidence id");
    assert!(sealed.acceptance_operation_id.is_none());
    assert!(succeeded.revision > awaiting.revision);
    let acceptance_evidence = succeeded
        .iterations
        .last()
        .expect("iteration")
        .evidence
        .iter()
        .find(|item| item.kind == "acceptance" && item.status == "passed")
        .expect("acceptance evidence");
    assert_eq!(
        acceptance_evidence
            .details
            .as_ref()
            .and_then(|details| details.get("sealedEvidenceId"))
            .and_then(serde_json::Value::as_str),
        Some(sealed_id.as_str())
    );
    // The same idempotency key returns the recorded outcome instead of accepting twice.
    assert!(acceptance
        .request(RequestLoopAcceptanceRequest {
            run_id: started.run_id.clone(),
            expected_revision: None,
            expected_scope_digest: None,
            expected_evidence_id: None,
            idempotency_key: Some("accept-once".to_string()),
        })
        .is_ok());
    assert!(acceptance
        .request(RequestLoopAcceptanceRequest {
            run_id: started.run_id.clone(),
            expected_revision: None,
            expected_scope_digest: None,
            expected_evidence_id: None,
            idempotency_key: None,
        })
        .is_err());
    // The worktree keeps the accepted contents; nothing outside moved.
    assert_eq!(
        std::fs::read_to_string(worktree.join("src/app.ts")).expect("accepted"),
        WORKER_CONTENT
    );
    assert_eq!(
        std::fs::read_to_string(&fixture.outside_file).expect("outside"),
        world.outside_sentinel
    );
}

#[test]
fn strict_start_refuses_a_cli_verifier_without_creating_a_run_or_worktree() {
    let fixture = fixture("loop-lifecycle-cli");
    let world = fixture.world.clone();
    let admission = world.admission();
    let definition = admission
        .create_definition(definition_request(&world, CLI_AGENT))
        .expect("definition saves; readiness decides");
    let readiness = admission.readiness(&definition.id).expect("readiness");
    assert!(!readiness.ready);
    let assessment = readiness.assessment.expect("assessment");
    assert!(!assessment.satisfies_requested_mode);
    assert!(assessment
        .surfaces
        .iter()
        .any(|surface| surface.surface == "verifier" && surface.coverage == "unsupported"));
    assert!(admission
        .start_manual(
            &definition.id,
            LoopControlEnvelope {
                expected_revision: Some(definition.version),
                idempotency_key: None,
                audit_acknowledgement_id: None,
            },
        )
        .is_err());
    assert!(world
        .repository
        .list_run_views(Some(&definition.id))
        .expect("runs")
        .is_empty());
    assert!(std::fs::read_dir(&world.worktrees)
        .expect("worktrees dir")
        .next()
        .is_none());
    assert_eq!(*world.operations.lock().expect("operations"), 0);
}
