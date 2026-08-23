//! Deterministic port doubles for the CLI parameter application tests. No SQLite, no filesystem,
//! no clock, and no child process.

use super::error::CliParameterApplicationError;
use super::models::{PersistedCliParameterProfile, ReplaceCliParameterProfile};
use super::ports::{
    CliInstallationSnapshotPort, CliParameterCatalogPort, CliParameterDiagnosticsPort,
    CliParameterDirectoryPort, CliParameterProfileRepository,
};
use super::service::CliParameterApplicationService;
use crate::contexts::tooling::cli_parameters::domain::catalog::CliParameterCatalog;
use crate::contexts::tooling::cli_parameters::domain::compatibility::{
    CliInstallationSnapshot, DottedVersionComparator,
};
use crate::contexts::tooling::cli_parameters::domain::definition::CliParameterPlatform;
use crate::contexts::tooling::cli_parameters::domain::diagnostic::CliParameterDiagnostic;
use crate::contexts::tooling::cli_parameters::domain::profile::{
    StoredCliParameterProfile, StoredSelectionRow,
};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, Mutex};

pub(super) const TEST_CATALOG_VERSION: &str = "9.9.9";

/// A registry with the five required agent ids. Only the two agents the tests exercise carry
/// parameters; validation permits an agent with none.
pub(super) const TEST_CATALOG: &str = r#"{
  "catalogVersion": "9.9.9",
  "selectionSchemaVersion": 2,
  "agents": [
    {
      "agentId": "claude-code",
      "parameters": [
        {
          "id": "model",
          "category": "model",
          "control": "custom-text",
          "labelKey": "t.claude-code.model.label",
          "descriptionKey": "t.claude-code.model.description",
          "launchScopes": ["interactive", "chat"],
          "options": [
            { "value": "sonnet", "labelKey": "t.sonnet.label", "descriptionKey": "t.sonnet.description" }
          ],
          "renderer": { "kind": "flag-value", "flag": "--model", "slot": "global" },
          "constraints": { "maxLength": 64, "pattern": "^[A-Za-z0-9][A-Za-z0-9._:@/+-]*$" },
          "audit": {
            "sourceId": "t",
            "sourceUrl": "https://example.invalid/x",
            "reviewedAt": "2026-08-22",
            "reviewedState": "test",
            "verification": "repository-verified",
            "note": "test"
          }
        },
        {
          "id": "screenReader",
          "category": "experience",
          "control": "boolean-flag",
          "labelKey": "t.claude-code.screenReader.label",
          "descriptionKey": "t.claude-code.screenReader.description",
          "launchScopes": ["interactive"],
          "renderer": { "kind": "presence-flag", "flag": "--ax-screen-reader", "slot": "global" },
          "compatibility": { "minVersion": "2.1.181" },
          "audit": {
            "sourceId": "t",
            "sourceUrl": "https://example.invalid/x",
            "reviewedAt": "2026-08-22",
            "reviewedState": "test",
            "verification": "repository-verified",
            "note": "test"
          }
        },
        {
          "id": "permissionMode",
          "category": "runtime",
          "ownership": "policy-governed",
          "control": "enum",
          "labelKey": "t.claude-code.permissionMode.label",
          "descriptionKey": "t.claude-code.permissionMode.description",
          "launchScopes": ["interactive", "chat"],
          "options": [
            { "value": "plan", "labelKey": "t.plan.label", "descriptionKey": "t.plan.description" }
          ],
          "renderer": { "kind": "flag-value", "flag": "--permission-mode", "slot": "global" },
          "audit": {
            "sourceId": "t",
            "sourceUrl": "https://example.invalid/x",
            "reviewedAt": "2026-08-22",
            "reviewedState": "test",
            "verification": "repository-verified",
            "note": "test"
          }
        }
      ]
    },
    {
      "agentId": "codex-cli",
      "parameters": [
        {
          "id": "oss",
          "category": "runtime",
          "control": "boolean-flag",
          "labelKey": "t.codex-cli.oss.label",
          "descriptionKey": "t.codex-cli.oss.description",
          "launchScopes": ["interactive", "chat"],
          "renderer": { "kind": "presence-flag", "flag": "--oss", "slot": "global" },
          "audit": {
            "sourceId": "t",
            "sourceUrl": "https://example.invalid/x",
            "reviewedAt": "2026-08-22",
            "reviewedState": "test",
            "verification": "repository-verified",
            "note": "test"
          }
        },
        {
          "id": "localProvider",
          "category": "runtime",
          "control": "enum",
          "labelKey": "t.codex-cli.localProvider.label",
          "descriptionKey": "t.codex-cli.localProvider.description",
          "launchScopes": ["interactive", "chat"],
          "options": [
            { "value": "ollama", "labelKey": "t.ollama.label", "descriptionKey": "t.ollama.description" }
          ],
          "renderer": { "kind": "flag-value", "flag": "--local-provider", "slot": "global" },
          "dependencies": {
            "requiresAll": [{ "parameterId": "oss", "operator": "equals", "value": true }]
          },
          "audit": {
            "sourceId": "t",
            "sourceUrl": "https://example.invalid/x",
            "reviewedAt": "2026-08-22",
            "reviewedState": "test",
            "verification": "repository-verified",
            "note": "test"
          }
        },
        {
          "id": "ephemeral",
          "category": "runtime",
          "control": "boolean-flag",
          "labelKey": "t.codex-cli.ephemeral.label",
          "descriptionKey": "t.codex-cli.ephemeral.description",
          "launchScopes": ["chat"],
          "renderer": { "kind": "presence-flag", "flag": "--ephemeral", "slot": "invocation" },
          "audit": {
            "sourceId": "t",
            "sourceUrl": "https://example.invalid/x",
            "reviewedAt": "2026-08-22",
            "reviewedState": "test",
            "verification": "repository-verified",
            "note": "test"
          }
        }
      ]
    },
    { "agentId": "gemini-cli", "parameters": [] },
    { "agentId": "opencode", "parameters": [] },
    { "agentId": "antigravity-cli", "parameters": [] }
  ]
}"#;

pub(super) struct FakeCatalog {
    catalog: Arc<CliParameterCatalog>,
}

impl Default for FakeCatalog {
    fn default() -> Self {
        Self {
            catalog: Arc::new(CliParameterCatalog::parse(TEST_CATALOG).expect("test catalog")),
        }
    }
}

impl CliParameterCatalogPort for FakeCatalog {
    fn catalog(&self) -> Result<Arc<CliParameterCatalog>, CliParameterApplicationError> {
        Ok(Arc::clone(&self.catalog))
    }
}

#[derive(Default)]
pub(super) struct FakeRepository {
    profiles: Mutex<BTreeMap<String, StoredCliParameterProfile>>,
    pub(super) writes: Mutex<usize>,
}

impl FakeRepository {
    pub(super) fn seed_legacy(&self, agent_id: &str, rows: &[(&str, &str)]) {
        self.profiles.lock().expect("lock").insert(
            agent_id.to_string(),
            StoredCliParameterProfile {
                agent_id: agent_id.to_string(),
                revision: 1,
                selection_schema_version: 1,
                catalog_version: String::new(),
                updated_at: Some("2026-01-01T00:00:00Z".to_string()),
                rows: rows
                    .iter()
                    .map(|(parameter_id, value_json)| StoredSelectionRow {
                        parameter_id: (*parameter_id).to_string(),
                        value_json: (*value_json).to_string(),
                    })
                    .collect(),
            },
        );
    }

    pub(super) fn revision(&self, agent_id: &str) -> i64 {
        self.profiles
            .lock()
            .expect("lock")
            .get(agent_id)
            .map(|profile| profile.revision)
            .unwrap_or(0)
    }
}

impl CliParameterProfileRepository for FakeRepository {
    fn load(
        &self,
        agent_id: &str,
    ) -> Result<StoredCliParameterProfile, CliParameterApplicationError> {
        Ok(self
            .profiles
            .lock()
            .expect("lock")
            .get(agent_id)
            .cloned()
            .unwrap_or(StoredCliParameterProfile {
                agent_id: agent_id.to_string(),
                revision: 0,
                selection_schema_version: 2,
                catalog_version: TEST_CATALOG_VERSION.to_string(),
                updated_at: None,
                rows: Vec::new(),
            }))
    }

    fn replace_if_revision(
        &self,
        mutation: ReplaceCliParameterProfile,
    ) -> Result<PersistedCliParameterProfile, CliParameterApplicationError> {
        let mut profiles = self.profiles.lock().expect("lock");
        let current = profiles
            .get(&mutation.agent_id)
            .map(|profile| profile.revision)
            .unwrap_or(0);
        if current != mutation.expected_revision {
            return Err(CliParameterApplicationError::RevisionConflict {
                agent_id: mutation.agent_id,
                expected_revision: mutation.expected_revision,
                actual_revision: current,
            });
        }
        let revision = current + 1;
        let rows = mutation
            .selections
            .iter()
            .filter(|(_, selection)| !selection.is_inherit())
            .map(|(parameter_id, selection)| StoredSelectionRow {
                parameter_id: parameter_id.clone(),
                value_json: serde_json::to_string(selection).expect("encode"),
            })
            .collect();
        profiles.insert(
            mutation.agent_id.clone(),
            StoredCliParameterProfile {
                agent_id: mutation.agent_id.clone(),
                revision,
                selection_schema_version: 2,
                catalog_version: mutation.catalog_version.clone(),
                updated_at: Some("2026-08-22T00:00:00Z".to_string()),
                rows,
            },
        );
        *self.writes.lock().expect("lock") += 1;
        Ok(PersistedCliParameterProfile {
            agent_id: mutation.agent_id,
            revision,
            catalog_version: mutation.catalog_version,
            updated_at: "2026-08-22T00:00:00Z".to_string(),
        })
    }

    fn reset_if_revision(
        &self,
        agent_id: &str,
        expected_revision: i64,
        catalog_version: &str,
    ) -> Result<PersistedCliParameterProfile, CliParameterApplicationError> {
        self.replace_if_revision(ReplaceCliParameterProfile {
            agent_id: agent_id.to_string(),
            expected_revision,
            catalog_version: catalog_version.to_string(),
            selections: Default::default(),
        })
    }
}

#[derive(Default)]
pub(super) struct FakeInstallations {
    pub(super) snapshots: Mutex<BTreeMap<String, CliInstallationSnapshot>>,
}

impl FakeInstallations {
    pub(super) fn set(&self, agent_id: &str, snapshot: CliInstallationSnapshot) {
        self.snapshots
            .lock()
            .expect("lock")
            .insert(agent_id.to_string(), snapshot);
    }
}

impl CliInstallationSnapshotPort for FakeInstallations {
    fn active_installation(
        &self,
        agent_id: &str,
    ) -> Result<CliInstallationSnapshot, CliParameterApplicationError> {
        Ok(self
            .snapshots
            .lock()
            .expect("lock")
            .get(agent_id)
            .cloned()
            .unwrap_or(CliInstallationSnapshot {
                installed: true,
                runnable: true,
                active_path: Some("/usr/bin/tool".to_string()),
                version: Some("9.9.9".to_string()),
                conflict: false,
            }))
    }
}

#[derive(Default)]
pub(super) struct FakeDirectories {
    pub(super) existing: Mutex<BTreeSet<String>>,
}

impl CliParameterDirectoryPort for FakeDirectories {
    fn directory_exists(&self, path: &str) -> bool {
        self.existing.lock().expect("lock").contains(path)
    }
}

#[derive(Default)]
pub(super) struct RecordingDiagnostics {
    pub(super) emitted: Mutex<Vec<CliParameterDiagnostic>>,
}

impl CliParameterDiagnosticsPort for RecordingDiagnostics {
    fn emit(&self, diagnostic: &CliParameterDiagnostic) {
        self.emitted.lock().expect("lock").push(diagnostic.clone());
    }
}

pub(super) struct Harness {
    pub(super) service: CliParameterApplicationService,
    pub(super) repository: Arc<FakeRepository>,
    pub(super) installations: Arc<FakeInstallations>,
    pub(super) diagnostics: Arc<RecordingDiagnostics>,
}

pub(super) fn harness() -> Harness {
    let repository = Arc::new(FakeRepository::default());
    let installations = Arc::new(FakeInstallations::default());
    let directories = Arc::new(FakeDirectories::default());
    let diagnostics = Arc::new(RecordingDiagnostics::default());
    let service = CliParameterApplicationService {
        catalog: Arc::new(FakeCatalog::default()),
        repository: repository.clone(),
        installations: installations.clone(),
        directories: directories.clone(),
        diagnostics: diagnostics.clone(),
        comparator: Arc::new(DottedVersionComparator),
        platform: CliParameterPlatform::Linux,
    };
    Harness {
        service,
        repository,
        installations,
        diagnostics,
    }
}
