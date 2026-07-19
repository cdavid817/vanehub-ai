use super::*;
use crate::contexts::agent_runtime::application::{
    AgentAvailabilityGateway, AgentRegistryRepository, AgentRuntimeApplicationError,
    AgentWorkflowRepository,
};
use crate::contexts::agent_runtime::domain::{
    AgentAvailability, AgentLifecycle, AgentWorkflow, AvailabilityAssessment, InteractionMode,
};
use crate::platform::database::NativeDatabase;
use crate::test_support::TempDirectory;
use rusqlite::params;
use std::sync::Arc;

#[derive(Clone)]
struct FakeAvailability {
    assessment: AvailabilityAssessment,
}

impl AgentAvailabilityGateway for FakeAvailability {
    fn assess(
        &self,
        _managed_sdk_dependency_id: Option<&str>,
        _executable_name: Option<&str>,
    ) -> Result<AvailabilityAssessment, AgentRuntimeApplicationError> {
        Ok(self.assessment.clone())
    }
}

fn repository(
    assessment: AvailabilityAssessment,
) -> (TempDirectory, NativeDatabase, SqliteAgentRuntimeRepository) {
    let directory = TempDirectory::new("agent-runtime-repository");
    let database = NativeDatabase::new(directory.path().to_path_buf()).expect("database");
    let repository = SqliteAgentRuntimeRepository::new(
        database.clone(),
        Arc::new(FakeAvailability { assessment }),
    );
    (directory, database, repository)
}

#[test]
fn seeded_registry_rows_map_to_stable_domain_catalog_values() {
    let (_directory, _database, repository) = repository(AvailabilityAssessment::new(
        AgentAvailability::Available,
        None,
    ));

    let agents = repository.list().expect("agents");
    assert_eq!(
        agents
            .iter()
            .map(|agent| agent.id().as_str())
            .collect::<Vec<_>>(),
        vec!["claude-code", "codex-cli", "gemini-cli", "opencode"]
    );
    let codex = agents
        .iter()
        .find(|agent| agent.id().as_str() == "codex-cli")
        .expect("codex");
    assert_eq!(codex.display_name(), "Codex CLI");
    assert_eq!(codex.provider(), "OpenAI");
    assert_eq!(codex.managed_sdk_dependency_id(), Some("codex-sdk"));
    assert_eq!(codex.launch().kind_str(), "cli");
    assert_eq!(codex.launch().command(), Some("codex"));
    assert_eq!(
        codex.supported_interaction_modes(),
        &[InteractionMode::Cli, InteractionMode::NativeDesktop]
    );
    assert_eq!(codex.capability_tags(), &["agent", "cli", "coding"]);
}

#[test]
fn availability_is_injected_from_runtime_facts_not_persisted_rows() {
    let (_directory, _database, repository) = repository(AvailabilityAssessment::new(
        AgentAvailability::Unavailable,
        Some("dependency missing".to_string()),
    ));

    let agent = repository.find("codex-cli").expect("query").expect("agent");
    assert_eq!(agent.availability().state(), AgentAvailability::Unavailable);
    assert_eq!(agent.availability().reason(), Some("dependency missing"));
}

#[test]
fn workflow_and_session_details_round_trip_through_singleton_rows() {
    let (_directory, database, repository) = repository(AvailabilityAssessment::new(
        AgentAvailability::Available,
        None,
    ));
    let agent = repository.find("codex-cli").expect("query").expect("agent");
    let mut workflow = AgentWorkflow::new("ship refactor");
    workflow
        .select(&agent, InteractionMode::Cli)
        .expect("select");
    workflow.begin_launch().expect("start");
    workflow.mark_running().expect("running");

    repository.save(&workflow).expect("save workflow");
    let loaded = repository.load().expect("load workflow");
    assert_eq!(
        loaded.active_agent_id().map(|id| id.as_str()),
        Some("codex-cli")
    );
    assert_eq!(loaded.active_interaction_mode(), Some(InteractionMode::Cli));
    assert_eq!(loaded.lifecycle(), AgentLifecycle::Running);
    assert_eq!(loaded.intent(), "ship refactor");
    let stored = database
        .connection()
        .expect("connection")
        .query_row(
            "SELECT active_agent_id, active_interaction_mode, lifecycle_state, intent FROM workflow_state WHERE id = 1",
            [],
            |row| {
                Ok((
                    row.get::<_, Option<String>>(0)?,
                    row.get::<_, Option<String>>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                ))
            },
        )
        .expect("stored workflow");
    assert_eq!(
        stored,
        (
            Some("codex-cli".to_string()),
            Some("cli".to_string()),
            "running".to_string(),
            "ship refactor".to_string(),
        )
    );

    repository
        .save_details("cli", "CLI workflow launch routed through Tauri adapter.")
        .expect("save details");
    let (adapter, details) = repository.load_details().expect("details");
    assert_eq!(adapter, "cli");
    assert_eq!(
        details.get("message").map(String::as_str),
        Some("CLI workflow launch routed through Tauri adapter.")
    );
    assert_eq!(details.get("runtime").map(String::as_str), Some("tauri"));
}

#[test]
fn invalid_registry_modes_and_incomplete_workflows_fail_explicitly() {
    let (_directory, database, repository) = repository(AvailabilityAssessment::new(
        AgentAvailability::Available,
        None,
    ));
    let connection = database.connection().expect("connection");
    connection
        .execute("DELETE FROM agent_modes WHERE agent_id = ?1", ["codex-cli"])
        .expect("delete modes");
    connection
        .execute(
            "INSERT INTO agent_modes (agent_id, mode) VALUES (?1, ?2)",
            params!["codex-cli", "terminal"],
        )
        .expect("invalid mode");
    assert!(matches!(
        repository.find("codex-cli"),
        Err(AgentRuntimeApplicationError::Domain(_))
    ));

    connection
        .execute(
            "UPDATE workflow_state SET active_agent_id = ?1, active_interaction_mode = NULL WHERE id = 1",
            ["codex-cli"],
        )
        .expect("write incomplete workflow");
    assert!(matches!(
        repository.load(),
        Err(AgentRuntimeApplicationError::Domain(_))
    ));
}
