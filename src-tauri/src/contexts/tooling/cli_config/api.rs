use super::application::{
    CliConfigCredentialPort, CliConfigRepository, CliGlobalConfigPort, ProjectionOutcome,
};

/// Re-exported for `permissions` (`add-claude-code-permission-callback` design.md D7) — the
/// first, and so far only, cross-context consumer of `cli_config`. Everything else in this file
/// stays `cli_config`-internal.
pub(crate) use super::application::ClaudeCodeHookProjectionPort;
pub(crate) use super::domain::CliConfigError;
use super::domain::{
    validate_profile_input, validate_supported_agent, AppliedStateRecord,
    ApplyCliConfigProfileInput, CliConfigAppliedState, CliConfigApplyResult,
    CliConfigDiscoveryCandidate, CliConfigDiscoveryResult, CliConfigDiscoveryState,
    CliConfigDriftResolution, CliConfigDriftState, CliConfigPayload, CliConfigProfile,
    CliConfigStartupSyncResult, CliConfigStartupSyncState, CliConfigStatus,
    CliConfigValidationState, DeleteCliConfigProfileInput, ImportCliConfigProfileInput,
    ImportDiscoveredCliConfigInput, ImportDiscoveredCliConfigResult, ProfileRecord,
    SaveCliConfigProfileInput, SkippedCliConfigDiscoveryCandidate,
    ValidateCliConfigCredentialInput, PAYLOAD_VERSION, SUPPORTED_AGENT_IDS,
};
use crate::contexts::operations::api::{DiagnosticLog, DiagnosticLogPort, LogSeverity};
use crate::platform::network::{
    probe_provider_credential, unsupported_provider_credential_validation,
    ProviderCredentialProbeAuthentication, ProviderCredentialProbeProtocol,
    ProviderCredentialProbeRequest, ProviderCredentialValidationResult,
};
use chrono::Utc;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, Mutex};
use uuid::Uuid;

#[derive(Clone)]
pub(crate) struct CliConfigApi {
    repository: Arc<dyn CliConfigRepository>,
    credentials: Arc<dyn CliConfigCredentialPort>,
    live: Arc<dyn CliGlobalConfigPort>,
    logging: Arc<dyn DiagnosticLogPort>,
    apply_locks: Arc<BTreeMap<String, Arc<Mutex<()>>>>,
    startup_sync: Arc<Mutex<BTreeMap<String, CliConfigStartupSyncResult>>>,
}

impl CliConfigApi {
    pub(crate) fn new(
        repository: Arc<dyn CliConfigRepository>,
        credentials: Arc<dyn CliConfigCredentialPort>,
        live: Arc<dyn CliGlobalConfigPort>,
        logging: Arc<dyn DiagnosticLogPort>,
    ) -> Self {
        let apply_locks = super::domain::SUPPORTED_AGENT_IDS
            .into_iter()
            .map(|agent_id| (agent_id.to_string(), Arc::new(Mutex::new(()))))
            .collect();
        let startup_sync = SUPPORTED_AGENT_IDS
            .into_iter()
            .map(|agent_id| {
                (
                    agent_id.to_string(),
                    startup_sync_result(agent_id, CliConfigStartupSyncState::Pending),
                )
            })
            .collect();
        Self {
            repository,
            credentials,
            live,
            logging,
            apply_locks: Arc::new(apply_locks),
            startup_sync: Arc::new(Mutex::new(startup_sync)),
        }
    }

    pub(crate) fn synchronize_startup(&self) -> Vec<CliConfigStartupSyncResult> {
        SUPPORTED_AGENT_IDS
            .into_iter()
            .map(|agent_id| {
                let result = match self.synchronize_startup_agent(agent_id) {
                    Ok(result) => result,
                    Err(error) => {
                        let mut result =
                            startup_sync_result(agent_id, CliConfigStartupSyncState::Warning);
                        result.skipped = 1;
                        result.warnings.push(startup_sync_warning(&error));
                        result.synchronized_at = Some(Utc::now().to_rfc3339());
                        result
                    }
                };
                self.write_log(
                    if result.state == CliConfigStartupSyncState::Warning {
                        LogSeverity::Warn
                    } else {
                        LogSeverity::Info
                    },
                    "cli.config.startup.sync",
                    agent_id,
                    None,
                    None,
                    &format!(
                        "startup CLI configuration sync completed: state={:?}, imported={}, updated={}, skipped={}",
                        result.state, result.imported, result.updated, result.skipped
                    ),
                );
                if let Ok(mut stored) = self.startup_sync.lock() {
                    stored.insert(agent_id.to_string(), result.clone());
                }
                result
            })
            .collect()
    }

    fn synchronize_startup_agent(
        &self,
        agent_id: &str,
    ) -> Result<CliConfigStartupSyncResult, CliConfigError> {
        if agent_id == "opencode" {
            return self.synchronize_opencode_startup();
        }
        let existing = self.repository.list_profiles(agent_id)?;
        if !existing.is_empty() {
            let mut result = startup_sync_result(agent_id, CliConfigStartupSyncState::Skipped);
            result.skipped = 1;
            result.synchronized_at = Some(Utc::now().to_rfc3339());
            return Ok(result);
        }
        let discovery = self.live.discover_current(agent_id)?;
        let Some(candidate) = discovery.candidates.into_iter().next() else {
            let mut result = startup_sync_result(agent_id, CliConfigStartupSyncState::Unchanged);
            result.warnings = discovery.warnings;
            result.synchronized_at = Some(Utc::now().to_rfc3339());
            return Ok(result);
        };
        let profile_id = format!("{agent_id}-default");
        let imported_credential = candidate.credential.is_some();
        let record = self.persist_live_profile(
            agent_id,
            profile_id,
            "default".to_string(),
            candidate.payload,
            candidate.credential.as_deref().map(String::as_str),
            None,
        )?;
        let inspection = self.live.inspect(agent_id, None, Some(&record))?;
        let applied = AppliedStateRecord {
            agent_id: agent_id.to_string(),
            profile_id: Some(record.id.clone()),
            projection_fingerprint: inspection.managed_fingerprint.clone(),
            live_fingerprint: inspection.managed_fingerprint,
            drift_state: CliConfigDriftState::Applied,
            applied_at: Utc::now().to_rfc3339(),
            applied_payload: Some(record.payload.clone()),
            managed_keys: record.managed_keys.clone(),
        };
        if let Err(error) = self.repository.save_applied_state(&applied) {
            let credential_restored =
                !imported_credential || self.credentials.delete(agent_id, &record.id).is_ok();
            let profile_restored = self.repository.delete_profile(agent_id, &record.id).is_ok();
            if !credential_restored || !profile_restored {
                return Err(CliConfigError::RollbackIncomplete);
            }
            return Err(error);
        }
        let mut result = startup_sync_result(agent_id, CliConfigStartupSyncState::Imported);
        result.imported = 1;
        result.warnings = discovery.warnings;
        result.synchronized_at = Some(Utc::now().to_rfc3339());
        Ok(result)
    }

    fn synchronize_opencode_startup(&self) -> Result<CliConfigStartupSyncResult, CliConfigError> {
        let agent_id = "opencode";
        let discovery = self.live.discover_current(agent_id)?;
        let mut existing = self.repository.list_profiles(agent_id)?;
        let mut result = startup_sync_result(agent_id, CliConfigStartupSyncState::Unchanged);
        result.warnings = discovery.warnings;
        for candidate in discovery.candidates {
            let provider_id = match &candidate.payload {
                CliConfigPayload::Opencode { provider_id, .. } => provider_id.clone(),
                _ => continue,
            };
            let matched = existing.iter().find(|profile| {
                matches!(
                    &profile.payload,
                    CliConfigPayload::Opencode {
                        provider_id: existing_id,
                        ..
                    } if existing_id == &provider_id
                )
            });
            let profile_id = match matched {
                Some(profile) => profile.id.clone(),
                None => self.next_profile_id(&format!("opencode-{provider_id}"))?,
            };
            let prior = matched.cloned();
            let prior_secret = self.credentials.load(agent_id, &profile_id)?;
            let incoming_secret = candidate.credential.as_deref().map(String::as_str);
            let changed = prior.as_ref().is_none_or(|profile| {
                profile.payload != candidate.payload
                    || profile.name != candidate.suggested_name
                    || prior_secret.as_deref().map(String::as_str) != incoming_secret
            });
            if !changed {
                result.skipped += 1;
                continue;
            }
            let record = self.persist_live_profile(
                agent_id,
                profile_id,
                candidate.suggested_name,
                candidate.payload,
                incoming_secret,
                prior.as_ref(),
            )?;
            if prior.is_some() {
                result.updated += 1;
            } else {
                result.imported += 1;
                existing.push(record);
            }
        }
        result.state = if result.updated > 0 {
            CliConfigStartupSyncState::Updated
        } else if result.imported > 0 {
            CliConfigStartupSyncState::Imported
        } else {
            CliConfigStartupSyncState::Unchanged
        };
        result.synchronized_at = Some(Utc::now().to_rfc3339());
        Ok(result)
    }

    fn persist_live_profile(
        &self,
        agent_id: &str,
        profile_id: String,
        name: String,
        payload: CliConfigPayload,
        credential: Option<&str>,
        existing: Option<&ProfileRecord>,
    ) -> Result<ProfileRecord, CliConfigError> {
        payload.validate()?;
        if payload.agent_id() != agent_id {
            return Err(CliConfigError::Validation(
                "live configuration payload does not match the selected Agent".into(),
            ));
        }
        if credential
            .is_some_and(|secret| secret.is_empty() || secret.chars().any(char::is_control))
        {
            return Err(CliConfigError::Validation(
                "live configuration credential is invalid".into(),
            ));
        }
        let sort_position = match existing {
            Some(profile) => profile.sort_position,
            None => self.repository.list_profiles(agent_id)?.len() as i64,
        };
        let previous_secret = self.credentials.load(agent_id, &profile_id)?;
        match credential {
            Some(secret) => self.credentials.store(agent_id, &profile_id, secret)?,
            None if previous_secret.is_some() => self.credentials.delete(agent_id, &profile_id)?,
            None => {}
        }
        let now = Utc::now().to_rfc3339();
        let record = ProfileRecord {
            id: profile_id.clone(),
            agent_id: agent_id.to_string(),
            name,
            payload_version: PAYLOAD_VERSION,
            managed_keys: payload.managed_keys(),
            payload,
            source_preset_id: None,
            source_preset_version: None,
            created_at: existing
                .map(|profile| profile.created_at.clone())
                .unwrap_or_else(|| now.clone()),
            updated_at: now,
            sort_position,
        };
        if let Err(error) = self.repository.save_profile(&record) {
            if self
                .restore_credential(
                    agent_id,
                    &profile_id,
                    previous_secret.as_deref().map(String::as_str),
                )
                .is_err()
            {
                return Err(CliConfigError::RollbackIncomplete);
            }
            return Err(error);
        }
        Ok(record)
    }

    pub(crate) fn list_profiles(
        &self,
        agent_id: &str,
    ) -> Result<Vec<CliConfigProfile>, CliConfigError> {
        validate_supported_agent(agent_id)?;
        let status = self.inspect_status(agent_id)?;
        self.repository
            .list_profiles(agent_id)?
            .into_iter()
            .map(|record| self.profile_view(record, &status))
            .collect()
    }

    pub(crate) async fn validate_credential(
        &self,
        input: ValidateCliConfigCredentialInput,
    ) -> Result<ProviderCredentialValidationResult, CliConfigError> {
        let api = self.clone();
        tauri::async_runtime::spawn_blocking(move || api.validate_credential_inner(input))
            .await
            .map_err(|_| {
                CliConfigError::Validation("CLI credential validation task failed".into())
            })?
    }

    fn validate_credential_inner(
        &self,
        input: ValidateCliConfigCredentialInput,
    ) -> Result<ProviderCredentialValidationResult, CliConfigError> {
        validate_supported_agent(&input.agent_id)?;
        if input
            .credential
            .as_deref()
            .is_some_and(|value| value.is_empty() || value.chars().any(char::is_control))
        {
            return Err(CliConfigError::Validation(
                "credential must not be empty or contain control characters".into(),
            ));
        }

        let stored = input
            .profile_id
            .as_deref()
            .map(|profile_id| self.repository.get_profile(&input.agent_id, profile_id))
            .transpose()?;
        let payload = input
            .payload
            .as_ref()
            .or_else(|| stored.as_ref().map(|profile| &profile.payload))
            .ok_or_else(|| {
                CliConfigError::Validation(
                    "a profile or a complete provider configuration is required".into(),
                )
            })?;
        if payload.agent_id() != input.agent_id {
            return Err(CliConfigError::Validation(
                "profile payload does not match the selected Agent".into(),
            ));
        }
        payload.validate()?;
        if !payload.requires_credential() {
            return Ok(unsupported_provider_credential_validation());
        }

        let stored_credential = match (&input.credential, &input.profile_id) {
            (Some(_), _) => None,
            (None, Some(profile_id)) => self.credentials.load(&input.agent_id, profile_id)?,
            (None, None) => None,
        };
        let credential = input
            .credential
            .as_deref()
            .or_else(|| stored_credential.as_deref().map(String::as_str))
            .ok_or(CliConfigError::CredentialRequired)?;
        let source_preset_id = input.source_preset_id.as_deref().or_else(|| {
            stored
                .as_ref()
                .and_then(|profile| profile.source_preset_id.as_deref())
        });
        let request = cli_probe_request(payload, source_preset_id, credential)?;
        let result = probe_provider_credential(request)
            .map_err(|error| CliConfigError::Validation(error.to_string()))?;
        self.write_log(
            LogSeverity::Info,
            "cli.config.credential-validation",
            &input.agent_id,
            input.profile_id.as_deref(),
            None,
            &format!(
                "CLI provider credential validation completed: status={:?}",
                result.status
            ),
        );
        Ok(result)
    }

    pub(crate) fn inspect_status(&self, agent_id: &str) -> Result<CliConfigStatus, CliConfigError> {
        validate_supported_agent(agent_id)?;
        let applied = self.repository.applied_state(agent_id)?;
        let applied_profile = applied
            .as_ref()
            .map(|state| self.applied_profile_snapshot(agent_id, state))
            .transpose()?
            .flatten();
        let inspection = self
            .live
            .inspect(agent_id, applied.as_ref(), applied_profile.as_ref())?;
        Ok(CliConfigStatus {
            agent_id: agent_id.to_string(),
            applied_profile_id: applied.as_ref().and_then(|state| state.profile_id.clone()),
            drift_state: inspection.state,
            resolved_paths: inspection
                .paths
                .into_iter()
                .map(|path| path.display().to_string())
                .collect(),
            last_applied_at: applied.map(|state| state.applied_at),
            simulated: false,
            startup_sync: self.startup_sync_result(agent_id),
        })
    }

    fn startup_sync_result(&self, agent_id: &str) -> CliConfigStartupSyncResult {
        self.startup_sync
            .lock()
            .ok()
            .and_then(|results| results.get(agent_id).cloned())
            .unwrap_or_else(|| startup_sync_result(agent_id, CliConfigStartupSyncState::Warning))
    }

    pub(crate) fn save_profile(
        &self,
        input: SaveCliConfigProfileInput,
    ) -> Result<CliConfigProfile, CliConfigError> {
        let agent_id = input.agent_id.clone();
        let result = self.save_profile_inner(input);
        self.log_result(
            "cli.config.profile.save",
            &agent_id,
            result.as_ref().ok().map(|p| p.id.as_str()),
            &result,
        );
        result
    }

    fn save_profile_inner(
        &self,
        input: SaveCliConfigProfileInput,
    ) -> Result<CliConfigProfile, CliConfigError> {
        validate_profile_input(&input)?;
        if input.credential.is_some() && input.remove_credential {
            return Err(CliConfigError::Validation(
                "credential cannot be replaced and removed in one request".into(),
            ));
        }
        if let Some(source_id) = input.source_preset_id.as_deref() {
            let compatible_prefix = format!("{}-", input.agent_id);
            if !source_id.starts_with(&compatible_prefix) {
                return Err(CliConfigError::Validation(
                    "preset is incompatible with the selected Agent".into(),
                ));
            }
        }

        let existing = input
            .id
            .as_deref()
            .map(|id| self.repository.get_profile(&input.agent_id, id))
            .transpose()?;
        let id = match &existing {
            Some(profile) => profile.id.clone(),
            None => self.next_profile_id(&input.name)?,
        };
        let previous_secret = self.credentials.load(&input.agent_id, &id)?;
        if existing.is_none()
            && input.payload.requires_credential()
            && input.credential.is_none()
            && !input.remove_credential
        {
            return Err(CliConfigError::CredentialRequired);
        }

        let wrote_credential = if let Some(secret) = input.credential.as_deref() {
            self.credentials.store(&input.agent_id, &id, secret)?;
            true
        } else if input.remove_credential {
            self.credentials.delete(&input.agent_id, &id)?;
            true
        } else {
            false
        };

        let now = Utc::now().to_rfc3339();
        let record = ProfileRecord {
            id: id.clone(),
            agent_id: input.agent_id.clone(),
            name: input.name.trim().to_string(),
            payload_version: PAYLOAD_VERSION,
            managed_keys: input.payload.managed_keys(),
            payload: input.payload,
            source_preset_id: input.source_preset_id,
            source_preset_version: input.source_preset_version,
            created_at: existing
                .as_ref()
                .map(|profile| profile.created_at.clone())
                .unwrap_or_else(|| now.clone()),
            updated_at: now,
            sort_position: existing
                .as_ref()
                .map(|profile| profile.sort_position)
                .unwrap_or(self.repository.list_profiles(&input.agent_id)?.len() as i64),
        };
        if let Err(error) = self.repository.save_profile(&record) {
            if wrote_credential {
                self.restore_credential(
                    &input.agent_id,
                    &id,
                    previous_secret.as_deref().map(String::as_str),
                )?;
            }
            return Err(error);
        }
        let status = self.inspect_status(&input.agent_id)?;
        self.profile_view(record, &status)
    }

    pub(crate) fn duplicate_profile(
        &self,
        agent_id: &str,
        profile_id: &str,
    ) -> Result<CliConfigProfile, CliConfigError> {
        validate_supported_agent(agent_id)?;
        let source = self.repository.get_profile(agent_id, profile_id)?;
        let credential = self.credentials.load(agent_id, profile_id)?;
        let names = self
            .repository
            .list_profiles(agent_id)?
            .into_iter()
            .map(|profile| profile.name)
            .collect::<Vec<_>>();
        let name = unique_copy_name(&source.name, &names);
        self.save_profile(SaveCliConfigProfileInput {
            id: None,
            agent_id: source.agent_id,
            name,
            payload: source.payload,
            source_preset_id: source.source_preset_id,
            source_preset_version: source.source_preset_version,
            credential: credential.as_deref().map(ToString::to_string),
            remove_credential: false,
        })
    }

    pub(crate) fn delete_profile(
        &self,
        input: DeleteCliConfigProfileInput,
    ) -> Result<(), CliConfigError> {
        validate_supported_agent(&input.agent_id)?;
        let profile = self
            .repository
            .get_profile(&input.agent_id, &input.profile_id)?;
        let applied = self.repository.applied_state(&input.agent_id)?;
        let is_applied = applied
            .as_ref()
            .and_then(|state| state.profile_id.as_deref())
            == Some(input.profile_id.as_str());
        if is_applied && !input.detach_applied {
            return Err(CliConfigError::Validation(
                "applied profile must be detached before deletion".into(),
            ));
        }
        let previous_secret = self.credentials.load(&input.agent_id, &input.profile_id)?;
        if is_applied {
            self.repository.clear_applied_state(&input.agent_id)?;
        }
        if let Err(error) = self.credentials.delete(&input.agent_id, &input.profile_id) {
            if let Some(state) = &applied {
                self.repository.save_applied_state(state)?;
            }
            return Err(error);
        }
        if let Err(error) = self
            .repository
            .delete_profile(&input.agent_id, &input.profile_id)
        {
            self.restore_credential(
                &input.agent_id,
                &input.profile_id,
                previous_secret.as_deref().map(String::as_str),
            )?;
            if let Some(state) = &applied {
                self.repository.save_applied_state(state)?;
            }
            return Err(error);
        }
        self.write_log(
            LogSeverity::Info,
            "cli.config.profile.delete",
            &input.agent_id,
            Some(&profile.id),
            None,
            "deleted CLI configuration profile",
        );
        Ok(())
    }

    pub(crate) fn import_current(
        &self,
        input: ImportCliConfigProfileInput,
    ) -> Result<CliConfigProfile, CliConfigError> {
        validate_supported_agent(&input.agent_id)?;
        let imported = self.live.import_current(&input.agent_id)?;
        let result = self.save_profile(SaveCliConfigProfileInput {
            id: None,
            agent_id: input.agent_id.clone(),
            name: input.name,
            payload: imported.payload,
            source_preset_id: None,
            source_preset_version: None,
            credential: imported.credential.as_deref().map(ToString::to_string),
            remove_credential: false,
        });
        self.log_result(
            "cli.config.profile.import",
            &input.agent_id,
            result.as_ref().ok().map(|profile| profile.id.as_str()),
            &result,
        );
        result
    }

    pub(crate) fn discover_profiles(
        &self,
        agent_id: &str,
    ) -> Result<CliConfigDiscoveryResult, CliConfigError> {
        validate_supported_agent(agent_id)?;
        let resolved_paths = self
            .live
            .paths(agent_id)?
            .into_iter()
            .map(|path| path.display().to_string())
            .collect::<Vec<_>>();
        match self.live.discover_current(agent_id) {
            Ok(discovery) => {
                let candidate_paths = discovery
                    .paths
                    .iter()
                    .map(|path| path.display().to_string())
                    .collect::<Vec<_>>();
                Ok(CliConfigDiscoveryResult {
                    agent_id: agent_id.to_string(),
                    state: CliConfigDiscoveryState::Available,
                    candidates: discovery
                        .candidates
                        .into_iter()
                        .map(|candidate| CliConfigDiscoveryCandidate {
                            candidate_key: candidate.candidate_key,
                            suggested_name: candidate.suggested_name,
                            provider_name: candidate.provider_name,
                            endpoint: candidate.endpoint,
                            model: candidate.model,
                            credential_detected: candidate.credential.is_some(),
                            is_default: candidate.is_default,
                            resolved_paths: candidate_paths.clone(),
                        })
                        .collect(),
                    resolved_paths: candidate_paths,
                    warnings: discovery.warnings,
                    error: None,
                    simulated: false,
                })
            }
            Err(CliConfigError::Parse { path, message: _ }) => Ok(CliConfigDiscoveryResult {
                agent_id: agent_id.to_string(),
                state: CliConfigDiscoveryState::ParseError,
                candidates: Vec::new(),
                resolved_paths,
                warnings: Vec::new(),
                error: Some(format!(
                    "Could not parse {path} as a supported CLI configuration."
                )),
                simulated: false,
            }),
            Err(error) => Err(error),
        }
    }

    pub(crate) fn import_discovered_profiles(
        &self,
        input: ImportDiscoveredCliConfigInput,
    ) -> Result<ImportDiscoveredCliConfigResult, CliConfigError> {
        validate_supported_agent(&input.agent_id)?;
        if input.candidate_keys.is_empty() {
            return Err(CliConfigError::Validation(
                "at least one discovered candidate must be selected".into(),
            ));
        }
        if input.candidate_keys.len() > 64 {
            return Err(CliConfigError::Validation(
                "too many discovered candidates were selected".into(),
            ));
        }
        let requested = input.candidate_keys.into_iter().collect::<BTreeSet<_>>();
        let discovery = self.live.discover_current(&input.agent_id)?;
        let available = discovery
            .candidates
            .iter()
            .map(|candidate| candidate.candidate_key.clone())
            .collect::<BTreeSet<_>>();
        if let Some(missing) = requested.difference(&available).next() {
            return Err(CliConfigError::Validation(format!(
                "discovered candidate is no longer available: {missing}"
            )));
        }
        let mut existing_names = self
            .repository
            .list_profiles(&input.agent_id)?
            .into_iter()
            .map(|profile| profile.name.trim().to_lowercase())
            .collect::<BTreeSet<_>>();
        let mut imported = Vec::new();
        let mut skipped = Vec::new();
        for candidate in discovery
            .candidates
            .into_iter()
            .filter(|candidate| requested.contains(&candidate.candidate_key))
        {
            let normalized_name = candidate.suggested_name.trim().to_lowercase();
            if existing_names.contains(&normalized_name) {
                skipped.push(SkippedCliConfigDiscoveryCandidate {
                    candidate_key: candidate.candidate_key,
                    suggested_name: candidate.suggested_name,
                    reason: "name-conflict".into(),
                });
                continue;
            }
            let profile = self.save_profile(SaveCliConfigProfileInput {
                id: None,
                agent_id: input.agent_id.clone(),
                name: candidate.suggested_name,
                payload: candidate.payload,
                source_preset_id: None,
                source_preset_version: None,
                credential: candidate.credential.as_deref().map(ToString::to_string),
                remove_credential: false,
            })?;
            existing_names.insert(normalized_name);
            imported.push(profile);
        }
        self.write_log(
            LogSeverity::Info,
            "cli.config.discovery.import",
            &input.agent_id,
            None,
            None,
            &format!(
                "imported {} discovered CLI configurations and skipped {}",
                imported.len(),
                skipped.len()
            ),
        );
        Ok(ImportDiscoveredCliConfigResult { imported, skipped })
    }

    pub(crate) fn apply_profile(
        &self,
        input: ApplyCliConfigProfileInput,
    ) -> Result<CliConfigApplyResult, CliConfigError> {
        validate_supported_agent(&input.agent_id)?;
        let operation_id = Uuid::new_v4().to_string();
        let result = self.apply_profile_inner(&input);
        let apply_result = match result {
            Ok((profile, outcome, backfilled_profile_id)) => apply_result(
                operation_id,
                &profile,
                outcome,
                if backfilled_profile_id.is_some() {
                    Some(CliConfigDriftResolution::ImportCurrent)
                } else {
                    input.drift_resolution.clone()
                },
                backfilled_profile_id,
            ),
            Err(error) => failed_apply_result(
                operation_id,
                &input,
                self.live.paths(&input.agent_id).unwrap_or_default(),
                &error,
            ),
        };
        self.write_apply_log(&apply_result);
        Ok(apply_result)
    }

    fn apply_profile_inner(
        &self,
        input: &ApplyCliConfigProfileInput,
    ) -> Result<(ProfileRecord, ProjectionOutcome, Option<String>), CliConfigError> {
        let apply_lock = self
            .apply_locks
            .get(&input.agent_id)
            .cloned()
            .ok_or(CliConfigError::Repository)?;
        let _apply_guard = apply_lock.lock().map_err(|_| CliConfigError::Repository)?;
        let profile = self
            .repository
            .get_profile(&input.agent_id, &input.profile_id)?;
        profile.payload.validate()?;
        let credential = self.credentials.load(&input.agent_id, &input.profile_id)?;
        if profile.payload.requires_credential() && credential.is_none() {
            return Err(CliConfigError::CredentialRequired);
        }
        let previous_state = self.repository.applied_state(&input.agent_id)?;
        let mut previous_profile = previous_state
            .as_ref()
            .map(|state| self.applied_profile_snapshot(&input.agent_id, state))
            .transpose()?
            .flatten();
        let inspection = self.live.inspect(
            &input.agent_id,
            previous_state.as_ref(),
            previous_profile.as_ref(),
        )?;
        if inspection.state == CliConfigDriftState::Malformed {
            return Err(CliConfigError::Parse {
                path: inspection
                    .paths
                    .first()
                    .map(|path| path.display().to_string())
                    .unwrap_or_else(|| "configuration".into()),
                message: "existing configuration is malformed".into(),
            });
        }
        let switching_exclusive_profile = input.agent_id != "opencode"
            && previous_profile
                .as_ref()
                .is_some_and(|previous| previous.id != profile.id);
        let backfilled_profile_id = if switching_exclusive_profile {
            let previous = previous_profile
                .as_ref()
                .ok_or(CliConfigError::Repository)?;
            let (updated, source_fingerprint) = self.backfill_profile(&input.agent_id, previous)?;
            let id = updated.id.clone();
            previous_profile = Some(updated);
            Some((id, source_fingerprint))
        } else {
            None
        };
        let expected_live_fingerprint = backfilled_profile_id
            .as_ref()
            .map(|(_, fingerprint)| fingerprint.clone())
            .unwrap_or(inspection.managed_fingerprint);
        let backfilled_profile_id = backfilled_profile_id.map(|(id, _)| id);

        let outcome = self.live.apply(
            &profile,
            previous_profile.as_ref(),
            credential.as_deref().map(|secret| secret.as_str()),
            input.confirm_auth_file_replacement,
            &expected_live_fingerprint,
        )?;
        let applied = AppliedStateRecord {
            agent_id: input.agent_id.clone(),
            profile_id: Some(profile.id.clone()),
            projection_fingerprint: outcome.projection_fingerprint.clone(),
            live_fingerprint: outcome.live_fingerprint.clone(),
            drift_state: CliConfigDriftState::Applied,
            applied_at: Utc::now().to_rfc3339(),
            applied_payload: Some(profile.payload.clone()),
            managed_keys: profile.managed_keys.clone(),
        };
        if let Err(error) = self.repository.save_applied_state(&applied) {
            let live_restored = self.live.restore(&outcome).is_ok();
            let state_restored = if let Some(previous) = &previous_state {
                self.repository.save_applied_state(previous).is_ok()
            } else {
                self.repository.clear_applied_state(&input.agent_id).is_ok()
            };
            if !live_restored || !state_restored {
                return Err(CliConfigError::RollbackIncomplete);
            }
            return Err(error);
        }
        Ok((profile, outcome, backfilled_profile_id))
    }

    fn backfill_profile(
        &self,
        agent_id: &str,
        previous: &ProfileRecord,
    ) -> Result<(ProfileRecord, String), CliConfigError> {
        let imported = self.live.import_current(agent_id)?;
        let source_fingerprint = imported.source_fingerprint.clone();
        imported.payload.validate()?;
        let mut updated = previous.clone();
        updated.payload = imported.payload;
        updated.managed_keys = previous
            .managed_keys
            .iter()
            .cloned()
            .chain(updated.payload.managed_keys())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
        updated.updated_at = Utc::now().to_rfc3339();
        let previous_secret = self.credentials.load(agent_id, &updated.id)?;
        let incoming_secret = imported.credential.as_deref().map(String::as_str);
        match incoming_secret {
            Some(secret) => self.credentials.store(agent_id, &updated.id, secret)?,
            None if previous_secret.is_some() => self.credentials.delete(agent_id, &updated.id)?,
            None => {}
        }
        if let Err(error) = self.repository.save_profile(&updated) {
            if self
                .restore_credential(
                    agent_id,
                    &updated.id,
                    previous_secret.as_deref().map(String::as_str),
                )
                .is_err()
            {
                return Err(CliConfigError::RollbackIncomplete);
            }
            return Err(error);
        }
        self.write_log(
            LogSeverity::Info,
            "cli.config.profile.backfill",
            agent_id,
            Some(&updated.id),
            None,
            "backfilled current live configuration into the leaving profile",
        );
        Ok((updated, source_fingerprint))
    }

    fn profile_view(
        &self,
        record: ProfileRecord,
        status: &CliConfigStatus,
    ) -> Result<CliConfigProfile, CliConfigError> {
        let credential_configured = self
            .credentials
            .load(&record.agent_id, &record.id)?
            .is_some();
        let validation_state = if record.payload.validate().is_err() {
            CliConfigValidationState::Invalid
        } else if record.payload.requires_credential() && !credential_configured {
            CliConfigValidationState::NeedsCredential
        } else {
            CliConfigValidationState::Valid
        };
        let applied_state = if status.applied_profile_id.as_deref() == Some(record.id.as_str()) {
            if status.drift_state == CliConfigDriftState::Applied {
                CliConfigAppliedState::Applied
            } else {
                CliConfigAppliedState::Drifted
            }
        } else {
            CliConfigAppliedState::Saved
        };
        Ok(CliConfigProfile {
            id: record.id,
            agent_id: record.agent_id,
            name: record.name,
            payload_version: record.payload_version,
            payload: record.payload,
            source_preset_id: record.source_preset_id,
            source_preset_version: record.source_preset_version,
            credential_configured,
            validation_state,
            applied_state,
            created_at: record.created_at,
            updated_at: record.updated_at,
        })
    }

    fn applied_profile_snapshot(
        &self,
        agent_id: &str,
        state: &AppliedStateRecord,
    ) -> Result<Option<ProfileRecord>, CliConfigError> {
        let Some(profile_id) = state.profile_id.as_deref() else {
            return Ok(None);
        };
        let mut profile = self.repository.get_profile(agent_id, profile_id)?;
        if let Some(payload) = &state.applied_payload {
            profile.payload = payload.clone();
        }
        if !state.managed_keys.is_empty() {
            profile.managed_keys = state.managed_keys.clone();
        }
        Ok(Some(profile))
    }

    fn next_profile_id(&self, name: &str) -> Result<String, CliConfigError> {
        let base = slug(name);
        if !self.repository.profile_id_exists(&base)? {
            return Ok(base);
        }
        for suffix in 2..10_000 {
            let candidate = format!("{base}-{suffix}");
            if !self.repository.profile_id_exists(&candidate)? {
                return Ok(candidate);
            }
        }
        Err(CliConfigError::Repository)
    }

    fn restore_credential(
        &self,
        agent_id: &str,
        profile_id: &str,
        previous: Option<&str>,
    ) -> Result<(), CliConfigError> {
        match previous {
            Some(secret) => self.credentials.store(agent_id, profile_id, secret),
            None => self.credentials.delete(agent_id, profile_id),
        }
    }

    fn log_result<T>(
        &self,
        category: &str,
        agent_id: &str,
        profile_id: Option<&str>,
        result: &Result<T, CliConfigError>,
    ) {
        self.write_log(
            if result.is_ok() {
                LogSeverity::Info
            } else {
                LogSeverity::Error
            },
            category,
            agent_id,
            profile_id,
            None,
            if result.is_ok() {
                "CLI configuration operation succeeded"
            } else {
                "CLI configuration operation failed"
            },
        );
    }

    fn write_log(
        &self,
        severity: LogSeverity,
        category: &str,
        agent_id: &str,
        profile_id: Option<&str>,
        operation_id: Option<&str>,
        message: &str,
    ) {
        let mut context = BTreeMap::from([("agentId".into(), agent_id.to_string())]);
        if let Some(profile_id) = profile_id {
            context.insert("profileId".into(), profile_id.to_string());
        }
        if let Some(operation_id) = operation_id {
            context.insert("operationId".into(), operation_id.to_string());
        }
        context.insert(
            "status".into(),
            if severity == LogSeverity::Error {
                "failed"
            } else {
                "succeeded"
            }
            .into(),
        );
        let _ = self.logging.write_diagnostic(DiagnosticLog {
            severity,
            category: category.into(),
            message: message.into(),
            context,
        });
    }

    fn write_apply_log(&self, result: &CliConfigApplyResult) {
        let mut context = BTreeMap::from([
            ("agentId".into(), result.agent_id.clone()),
            ("profileId".into(), result.profile_id.clone()),
            ("operationId".into(), result.operation_id.clone()),
            ("status".into(), result.status.clone()),
            ("restored".into(), result.restored.to_string()),
        ]);
        let target_files = result
            .affected_paths
            .iter()
            .filter_map(|path| std::path::Path::new(path).file_name())
            .map(|name| name.to_string_lossy())
            .collect::<Vec<_>>()
            .join(",");
        if !target_files.is_empty() {
            context.insert("targetFiles".into(), target_files);
        }
        let succeeded = result.status == "succeeded";
        let _ = self.logging.write_diagnostic(DiagnosticLog {
            severity: if succeeded {
                LogSeverity::Info
            } else {
                LogSeverity::Error
            },
            category: "cli.config.profile.apply".into(),
            message: if succeeded {
                "applied CLI configuration profile"
            } else {
                "CLI configuration profile application failed"
            }
            .into(),
            context,
        });
    }
}

fn cli_probe_request<'a>(
    payload: &'a CliConfigPayload,
    source_preset_id: Option<&str>,
    credential: &'a str,
) -> Result<ProviderCredentialProbeRequest<'a>, CliConfigError> {
    let request = match payload {
        // Antigravity holds no credential to probe: it authenticates through the OS keyring with
        // Google Sign-In, so there is nothing VaneHub could validate against an endpoint.
        CliConfigPayload::Antigravity { .. } => {
            return Err(CliConfigError::Validation(
                "Antigravity CLI authenticates through the system keyring and has no credential to validate".into(),
            ));
        }
        CliConfigPayload::ClaudeCode {
            base_url,
            auth_mode,
            model,
            ..
        } => ProviderCredentialProbeRequest {
            base_url,
            model,
            protocol: ProviderCredentialProbeProtocol::AnthropicMessages,
            authentication: match auth_mode {
                super::domain::ClaudeAuthMode::ApiKey => {
                    ProviderCredentialProbeAuthentication::AnthropicApiKey
                }
                super::domain::ClaudeAuthMode::AuthToken => {
                    ProviderCredentialProbeAuthentication::Bearer
                }
                super::domain::ClaudeAuthMode::None => {
                    return Err(CliConfigError::Validation(
                        "the selected authentication mode does not use an API key".into(),
                    ));
                }
            },
            credential,
        },
        CliConfigPayload::CodexCli {
            base_url,
            model,
            wire_api,
            ..
        } => ProviderCredentialProbeRequest {
            base_url,
            model,
            protocol: match wire_api {
                super::domain::CodexWireApi::Responses => {
                    ProviderCredentialProbeProtocol::OpenAiResponses
                }
                super::domain::CodexWireApi::Chat => {
                    ProviderCredentialProbeProtocol::OpenAiChatCompletions
                }
            },
            authentication: ProviderCredentialProbeAuthentication::Bearer,
            credential,
        },
        CliConfigPayload::Opencode {
            npm,
            base_url,
            default_model,
            ..
        } => {
            let anthropic = source_preset_id
                .is_some_and(|value| value.ends_with("-anthropic-messages"))
                || npm.contains("anthropic");
            ProviderCredentialProbeRequest {
                base_url,
                model: default_model,
                protocol: if anthropic {
                    ProviderCredentialProbeProtocol::AnthropicMessages
                } else if source_preset_id.is_some_and(|value| value.ends_with("-openai-responses"))
                {
                    ProviderCredentialProbeProtocol::OpenAiResponses
                } else {
                    ProviderCredentialProbeProtocol::OpenAiChatCompletions
                },
                authentication: if anthropic {
                    ProviderCredentialProbeAuthentication::AnthropicApiKey
                } else {
                    ProviderCredentialProbeAuthentication::Bearer
                },
                credential,
            }
        }
    };
    Ok(request)
}

fn startup_sync_result(
    agent_id: &str,
    state: CliConfigStartupSyncState,
) -> CliConfigStartupSyncResult {
    CliConfigStartupSyncResult {
        agent_id: agent_id.to_string(),
        state,
        imported: 0,
        updated: 0,
        skipped: 0,
        warnings: Vec::new(),
        synchronized_at: None,
        simulated: false,
    }
}

fn startup_sync_warning(error: &CliConfigError) -> String {
    match error {
        CliConfigError::Parse { .. } => {
            "The local CLI configuration could not be parsed; startup synchronization was skipped."
        }
        CliConfigError::Filesystem { .. } => {
            "The local CLI configuration could not be read; startup synchronization was skipped."
        }
        CliConfigError::Credential | CliConfigError::CredentialRequired => {
            "The local CLI credential could not be synchronized safely."
        }
        CliConfigError::RollbackIncomplete => {
            "Startup synchronization could not fully restore an interrupted profile update."
        }
        _ => "Startup synchronization could not be completed.",
    }
    .to_string()
}

fn apply_result(
    operation_id: String,
    profile: &ProfileRecord,
    outcome: ProjectionOutcome,
    drift_resolution: Option<super::domain::CliConfigDriftResolution>,
    backfilled_profile_id: Option<String>,
) -> CliConfigApplyResult {
    CliConfigApplyResult {
        operation_id,
        status: "succeeded".into(),
        agent_id: profile.agent_id.clone(),
        profile_id: profile.id.clone(),
        affected_paths: outcome
            .paths
            .into_iter()
            .map(|path| path.display().to_string())
            .collect(),
        drift_resolution,
        backfilled_profile_id,
        warnings: outcome.warnings,
        restart_required: true,
        simulated: false,
        restored: outcome.restored,
        error: None,
    }
}

fn failed_apply_result(
    operation_id: String,
    input: &ApplyCliConfigProfileInput,
    paths: Vec<std::path::PathBuf>,
    error: &CliConfigError,
) -> CliConfigApplyResult {
    CliConfigApplyResult {
        operation_id,
        status: "failed".into(),
        agent_id: input.agent_id.clone(),
        profile_id: input.profile_id.clone(),
        affected_paths: paths
            .into_iter()
            .map(|path| path.display().to_string())
            .collect(),
        drift_resolution: input.drift_resolution.clone(),
        backfilled_profile_id: None,
        warnings: Vec::new(),
        restart_required: false,
        simulated: false,
        restored: !matches!(error, CliConfigError::RollbackIncomplete),
        error: Some(error.to_string()),
    }
}

fn slug(value: &str) -> String {
    let mut output = String::new();
    let mut separator = false;
    for character in value.trim().chars() {
        if character.is_ascii_alphanumeric() {
            output.push(character.to_ascii_lowercase());
            separator = false;
        } else if !separator && !output.is_empty() {
            output.push('-');
            separator = true;
        }
    }
    while output.ends_with('-') {
        output.pop();
    }
    if output.is_empty() {
        "profile".into()
    } else {
        output
    }
}

fn unique_copy_name(source: &str, existing: &[String]) -> String {
    let base = format!("{source} Copy");
    if !existing.contains(&base) {
        return base;
    }
    for suffix in 2..10_000 {
        let candidate = format!("{base} {suffix}");
        if !existing.contains(&candidate) {
            return candidate;
        }
    }
    format!("{base} {}", Uuid::new_v4())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contexts::operations::api::OperationsError;
    use crate::contexts::tooling::cli_config::application::{ImportedLiveConfig, LiveInspection};
    use crate::contexts::tooling::cli_config::domain::{
        ApplyCliConfigProfileInput, ClaudeAuthMode, CliConfigDriftResolution, CliConfigPayload,
        ImportDiscoveredCliConfigInput, SaveCliConfigProfileInput,
    };
    use crate::contexts::tooling::cli_config::infrastructure::NativeCliGlobalConfigAdapter;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::thread;
    use std::time::Duration;

    #[derive(Default)]
    struct MemoryRepository {
        profiles: Mutex<Vec<ProfileRecord>>,
        applied: Mutex<BTreeMap<String, AppliedStateRecord>>,
        fail_next_save: AtomicUsize,
    }

    impl CliConfigRepository for MemoryRepository {
        fn list_profiles(&self, agent_id: &str) -> Result<Vec<ProfileRecord>, CliConfigError> {
            Ok(self
                .profiles
                .lock()
                .map_err(|_| CliConfigError::Repository)?
                .iter()
                .filter(|profile| profile.agent_id == agent_id)
                .cloned()
                .collect())
        }

        fn get_profile(
            &self,
            agent_id: &str,
            profile_id: &str,
        ) -> Result<ProfileRecord, CliConfigError> {
            self.profiles
                .lock()
                .map_err(|_| CliConfigError::Repository)?
                .iter()
                .find(|profile| profile.agent_id == agent_id && profile.id == profile_id)
                .cloned()
                .ok_or(CliConfigError::NotFound)
        }

        fn profile_id_exists(&self, profile_id: &str) -> Result<bool, CliConfigError> {
            Ok(self
                .profiles
                .lock()
                .map_err(|_| CliConfigError::Repository)?
                .iter()
                .any(|profile| profile.id == profile_id))
        }

        fn save_profile(&self, profile: &ProfileRecord) -> Result<(), CliConfigError> {
            if self.fail_next_save.swap(0, Ordering::SeqCst) > 0 {
                return Err(CliConfigError::Repository);
            }
            let mut profiles = self
                .profiles
                .lock()
                .map_err(|_| CliConfigError::Repository)?;
            profiles.retain(|candidate| candidate.id != profile.id);
            profiles.push(profile.clone());
            Ok(())
        }

        fn delete_profile(&self, agent_id: &str, profile_id: &str) -> Result<(), CliConfigError> {
            let mut profiles = self
                .profiles
                .lock()
                .map_err(|_| CliConfigError::Repository)?;
            let before = profiles.len();
            profiles.retain(|profile| profile.agent_id != agent_id || profile.id != profile_id);
            if profiles.len() == before {
                return Err(CliConfigError::NotFound);
            }
            Ok(())
        }

        fn applied_state(
            &self,
            agent_id: &str,
        ) -> Result<Option<AppliedStateRecord>, CliConfigError> {
            Ok(self
                .applied
                .lock()
                .map_err(|_| CliConfigError::Repository)?
                .get(agent_id)
                .cloned())
        }

        fn save_applied_state(&self, state: &AppliedStateRecord) -> Result<(), CliConfigError> {
            self.applied
                .lock()
                .map_err(|_| CliConfigError::Repository)?
                .insert(state.agent_id.clone(), state.clone());
            Ok(())
        }

        fn clear_applied_state(&self, agent_id: &str) -> Result<(), CliConfigError> {
            self.applied
                .lock()
                .map_err(|_| CliConfigError::Repository)?
                .remove(agent_id);
            Ok(())
        }
    }

    #[derive(Default)]
    struct MemoryCredentials {
        values: Mutex<BTreeMap<(String, String), String>>,
    }

    impl CliConfigCredentialPort for MemoryCredentials {
        fn load(
            &self,
            agent_id: &str,
            profile_id: &str,
        ) -> Result<Option<zeroize::Zeroizing<String>>, CliConfigError> {
            Ok(self
                .values
                .lock()
                .map_err(|_| CliConfigError::Credential)?
                .get(&(agent_id.to_string(), profile_id.to_string()))
                .cloned()
                .map(zeroize::Zeroizing::new))
        }

        fn store(
            &self,
            agent_id: &str,
            profile_id: &str,
            secret: &str,
        ) -> Result<(), CliConfigError> {
            self.values
                .lock()
                .map_err(|_| CliConfigError::Credential)?
                .insert(
                    (agent_id.to_string(), profile_id.to_string()),
                    secret.to_string(),
                );
            Ok(())
        }

        fn delete(&self, agent_id: &str, profile_id: &str) -> Result<(), CliConfigError> {
            self.values
                .lock()
                .map_err(|_| CliConfigError::Credential)?
                .remove(&(agent_id.to_string(), profile_id.to_string()));
            Ok(())
        }
    }

    #[derive(Default)]
    struct SerialProbeLiveConfig {
        active: AtomicUsize,
        max_active: AtomicUsize,
    }

    impl CliGlobalConfigPort for SerialProbeLiveConfig {
        fn paths(&self, _agent_id: &str) -> Result<Vec<PathBuf>, CliConfigError> {
            Ok(vec![PathBuf::from("safe-config")])
        }

        fn inspect(
            &self,
            agent_id: &str,
            _applied: Option<&AppliedStateRecord>,
            _applied_profile: Option<&ProfileRecord>,
        ) -> Result<LiveInspection, CliConfigError> {
            Ok(LiveInspection {
                paths: self.paths(agent_id)?,
                state: CliConfigDriftState::Detached,
                managed_fingerprint: "before".into(),
            })
        }

        fn import_current(&self, _agent_id: &str) -> Result<ImportedLiveConfig, CliConfigError> {
            Err(CliConfigError::NotFound)
        }

        fn apply(
            &self,
            _profile: &ProfileRecord,
            _previous: Option<&ProfileRecord>,
            _credential: Option<&str>,
            _confirm_auth_file_replacement: bool,
            _expected_live_fingerprint: &str,
        ) -> Result<ProjectionOutcome, CliConfigError> {
            let active = self.active.fetch_add(1, Ordering::SeqCst) + 1;
            self.max_active.fetch_max(active, Ordering::SeqCst);
            thread::sleep(Duration::from_millis(30));
            self.active.fetch_sub(1, Ordering::SeqCst);
            Ok(ProjectionOutcome {
                paths: vec![PathBuf::from("safe-config")],
                projection_fingerprint: "projection".into(),
                live_fingerprint: "live".into(),
                warnings: Vec::new(),
                restored: true,
                backups: Vec::new(),
            })
        }

        fn restore(&self, _outcome: &ProjectionOutcome) -> Result<(), CliConfigError> {
            Ok(())
        }
    }

    #[derive(Default)]
    struct NoopLog;

    impl DiagnosticLogPort for NoopLog {
        fn write_diagnostic(&self, _log: DiagnosticLog) -> Result<(), OperationsError> {
            Ok(())
        }
    }

    #[derive(Default)]
    struct CapturingLog {
        entries: Mutex<Vec<DiagnosticLog>>,
    }

    impl DiagnosticLogPort for CapturingLog {
        fn write_diagnostic(&self, log: DiagnosticLog) -> Result<(), OperationsError> {
            self.entries.lock().expect("log entries").push(log);
            Ok(())
        }
    }

    #[derive(Default)]
    struct RollbackFailLiveConfig;

    impl CliGlobalConfigPort for RollbackFailLiveConfig {
        fn paths(&self, _agent_id: &str) -> Result<Vec<PathBuf>, CliConfigError> {
            Ok(vec![PathBuf::from("safe-config")])
        }

        fn inspect(
            &self,
            agent_id: &str,
            _applied: Option<&AppliedStateRecord>,
            _applied_profile: Option<&ProfileRecord>,
        ) -> Result<LiveInspection, CliConfigError> {
            Ok(LiveInspection {
                paths: self.paths(agent_id)?,
                state: CliConfigDriftState::Detached,
                managed_fingerprint: "before".into(),
            })
        }

        fn import_current(&self, _agent_id: &str) -> Result<ImportedLiveConfig, CliConfigError> {
            Err(CliConfigError::NotFound)
        }

        fn apply(
            &self,
            _profile: &ProfileRecord,
            _previous: Option<&ProfileRecord>,
            _credential: Option<&str>,
            _confirm_auth_file_replacement: bool,
            _expected_live_fingerprint: &str,
        ) -> Result<ProjectionOutcome, CliConfigError> {
            Err(CliConfigError::RollbackIncomplete)
        }

        fn restore(&self, _outcome: &ProjectionOutcome) -> Result<(), CliConfigError> {
            Err(CliConfigError::RollbackIncomplete)
        }
    }

    #[derive(Default)]
    struct DriftImportLiveConfig;

    impl CliGlobalConfigPort for DriftImportLiveConfig {
        fn paths(&self, _agent_id: &str) -> Result<Vec<PathBuf>, CliConfigError> {
            Ok(vec![PathBuf::from("safe-config")])
        }

        fn inspect(
            &self,
            agent_id: &str,
            _applied: Option<&AppliedStateRecord>,
            _applied_profile: Option<&ProfileRecord>,
        ) -> Result<LiveInspection, CliConfigError> {
            Ok(LiveInspection {
                paths: self.paths(agent_id)?,
                state: CliConfigDriftState::Drifted,
                managed_fingerprint: "drifted".into(),
            })
        }

        fn import_current(&self, _agent_id: &str) -> Result<ImportedLiveConfig, CliConfigError> {
            Ok(ImportedLiveConfig {
                payload: CliConfigPayload::ClaudeCode {
                    base_url: "https://drifted.example.com".into(),
                    auth_mode: ClaudeAuthMode::AuthToken,
                    model: "drifted-model".into(),
                    haiku_model: "drifted-model".into(),
                    sonnet_model: "drifted-model".into(),
                    opus_model: "drifted-model".into(),
                    advanced_env: BTreeMap::new(),
                },
                credential: Some(zeroize::Zeroizing::new("new-secret".into())),
                source_fingerprint: "file:drifted".into(),
            })
        }

        fn apply(
            &self,
            _profile: &ProfileRecord,
            _previous: Option<&ProfileRecord>,
            _credential: Option<&str>,
            _confirm_auth_file_replacement: bool,
            _expected_live_fingerprint: &str,
        ) -> Result<ProjectionOutcome, CliConfigError> {
            Err(CliConfigError::Repository)
        }

        fn restore(&self, _outcome: &ProjectionOutcome) -> Result<(), CliConfigError> {
            Ok(())
        }
    }

    fn claude_without_credential(name: &str) -> SaveCliConfigProfileInput {
        SaveCliConfigProfileInput {
            id: None,
            agent_id: "claude-code".into(),
            name: name.into(),
            payload: CliConfigPayload::ClaudeCode {
                base_url: "https://api.anthropic.com".into(),
                auth_mode: ClaudeAuthMode::None,
                model: "claude-sonnet-4-6".into(),
                haiku_model: "claude-haiku-4-5".into(),
                sonnet_model: "claude-sonnet-4-6".into(),
                opus_model: "claude-opus-4-6".into(),
                advanced_env: BTreeMap::new(),
            },
            source_preset_id: None,
            source_preset_version: None,
            credential: None,
            remove_credential: false,
        }
    }

    #[test]
    fn profile_ids_are_stable_safe_slugs() {
        assert_eq!(slug(" OpenRouter Primary "), "openrouter-primary");
        assert_eq!(slug("智谱 GLM"), "glm");
        assert_eq!(slug("智谱"), "profile");
    }

    #[test]
    fn duplicate_names_do_not_collide() {
        assert_eq!(unique_copy_name("DeepSeek", &[]), "DeepSeek Copy");
        assert_eq!(
            unique_copy_name("DeepSeek", &["DeepSeek Copy".into()]),
            "DeepSeek Copy 2"
        );
    }

    #[test]
    fn discovered_import_redacts_credentials_and_skips_case_insensitive_name_conflicts() {
        let directory = tempfile::tempdir().expect("temporary home");
        let path = directory.path().join(".claude").join("settings.json");
        std::fs::create_dir_all(path.parent().expect("parent")).expect("directory");
        std::fs::write(
            &path,
            r#"{"env":{"ANTHROPIC_AUTH_TOKEN":"discovery-secret","ANTHROPIC_MODEL":"claude-sonnet-4-6"}}"#,
        )
        .expect("fixture");
        let repository = Arc::new(MemoryRepository::default());
        let credentials = Arc::new(MemoryCredentials::default());
        let api = CliConfigApi::new(
            repository,
            credentials.clone(),
            Arc::new(NativeCliGlobalConfigAdapter::with_home(
                directory.path().to_path_buf(),
            )),
            Arc::new(NoopLog),
        );

        let discovery = api.discover_profiles("claude-code").expect("discovery");
        assert_eq!(discovery.candidates.len(), 1);
        assert!(discovery.candidates[0].credential_detected);
        let serialized = serde_json::to_string(&discovery).expect("serialize discovery");
        assert!(!serialized.contains("discovery-secret"));

        let first = api
            .import_discovered_profiles(ImportDiscoveredCliConfigInput {
                agent_id: "claude-code".into(),
                candidate_keys: vec!["current".into()],
            })
            .expect("first import");
        assert_eq!(first.imported.len(), 1);
        assert!(credentials
            .load("claude-code", &first.imported[0].id)
            .expect("credential")
            .is_some());

        let second = api
            .import_discovered_profiles(ImportDiscoveredCliConfigInput {
                agent_id: "claude-code".into(),
                candidate_keys: vec!["current".into()],
            })
            .expect("conflict is reported");
        assert!(second.imported.is_empty());
        assert_eq!(second.skipped.len(), 1);
        assert_eq!(second.skipped[0].reason, "name-conflict");

        std::fs::write(&path, r#"{"env":{"ANTHROPIC_AUTH_TOKEN":"must-not-leak"}"#)
            .expect("malformed fixture");
        let malformed = api
            .discover_profiles("claude-code")
            .expect("parse state is data");
        assert_eq!(malformed.state, CliConfigDiscoveryState::ParseError);
        assert!(malformed.candidates.is_empty());
        assert!(!serde_json::to_string(&malformed)
            .expect("serialize malformed result")
            .contains("must-not-leak"));
    }

    #[test]
    fn command_errors_do_not_render_secret_values() {
        let rendered = CliConfigError::Credential.to_string();
        assert!(!rendered.contains("sk-secret"));
        assert_eq!(rendered, "secure credential operation failed");
    }

    #[test]
    fn concurrent_applies_for_one_agent_are_serialized_end_to_end() {
        let repository = Arc::new(MemoryRepository::default());
        let live = Arc::new(SerialProbeLiveConfig::default());
        let api = CliConfigApi::new(
            repository,
            Arc::new(MemoryCredentials::default()),
            live.clone(),
            Arc::new(NoopLog),
        );
        let first = api
            .save_profile(claude_without_credential("First"))
            .expect("first profile");
        let second = api
            .save_profile(claude_without_credential("Second"))
            .expect("second profile");

        let first_api = api.clone();
        let first_handle = thread::spawn(move || {
            first_api.apply_profile(ApplyCliConfigProfileInput {
                agent_id: "claude-code".into(),
                profile_id: first.id,
                drift_resolution: None,
                confirm_auth_file_replacement: false,
            })
        });
        let second_api = api.clone();
        let second_handle = thread::spawn(move || {
            second_api.apply_profile(ApplyCliConfigProfileInput {
                agent_id: "claude-code".into(),
                profile_id: second.id,
                drift_resolution: None,
                confirm_auth_file_replacement: false,
            })
        });

        first_handle
            .join()
            .expect("first thread")
            .expect("first apply");
        second_handle
            .join()
            .expect("second thread")
            .expect("second apply");
        assert_eq!(live.max_active.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn failed_profile_update_restores_the_previous_credential() {
        let repository = Arc::new(MemoryRepository::default());
        let credentials = Arc::new(MemoryCredentials::default());
        let api = CliConfigApi::new(
            repository.clone(),
            credentials.clone(),
            Arc::new(SerialProbeLiveConfig::default()),
            Arc::new(NoopLog),
        );
        let mut create = claude_without_credential("Credential profile");
        if let CliConfigPayload::ClaudeCode { auth_mode, .. } = &mut create.payload {
            *auth_mode = ClaudeAuthMode::AuthToken;
        }
        create.credential = Some("old-secret".into());
        let saved = api.save_profile(create.clone()).expect("initial profile");

        repository.fail_next_save.store(1, Ordering::SeqCst);
        create.id = Some(saved.id.clone());
        create.credential = Some("new-secret".into());
        assert!(matches!(
            api.save_profile(create),
            Err(CliConfigError::Repository)
        ));
        let restored = credentials
            .load("claude-code", &saved.id)
            .expect("credential load")
            .expect("restored credential");
        assert_eq!(restored.as_str(), "old-secret");
    }

    #[test]
    fn applied_snapshot_removes_keys_owned_before_profile_edit() {
        let directory = tempfile::tempdir().expect("temporary home");
        let repository = Arc::new(MemoryRepository::default());
        let api = CliConfigApi::new(
            repository,
            Arc::new(MemoryCredentials::default()),
            Arc::new(NativeCliGlobalConfigAdapter::with_home(
                directory.path().to_path_buf(),
            )),
            Arc::new(NoopLog),
        );
        let mut first_input = claude_without_credential("First");
        if let CliConfigPayload::ClaudeCode { advanced_env, .. } = &mut first_input.payload {
            advanced_env.insert("VANEHUB_OLD_MANAGED".into(), "old".into());
        }
        let first = api
            .save_profile(first_input.clone())
            .expect("first profile");
        let first_apply = api
            .apply_profile(ApplyCliConfigProfileInput {
                agent_id: "claude-code".into(),
                profile_id: first.id.clone(),
                drift_resolution: None,
                confirm_auth_file_replacement: false,
            })
            .expect("first apply");
        assert_eq!(first_apply.status, "succeeded");

        first_input.id = Some(first.id);
        if let CliConfigPayload::ClaudeCode { advanced_env, .. } = &mut first_input.payload {
            advanced_env.clear();
        }
        api.save_profile(first_input).expect("edit applied profile");
        let second = api
            .save_profile(claude_without_credential("Second"))
            .expect("second profile");
        let second_apply = api
            .apply_profile(ApplyCliConfigProfileInput {
                agent_id: "claude-code".into(),
                profile_id: second.id,
                drift_resolution: None,
                confirm_auth_file_replacement: false,
            })
            .expect("second apply");
        assert_eq!(second_apply.status, "succeeded");

        let settings =
            std::fs::read_to_string(directory.path().join(".claude").join("settings.json"))
                .expect("settings");
        assert!(!settings.contains("VANEHUB_OLD_MANAGED"));
    }

    #[test]
    fn apply_failure_returns_restoration_status_and_redacted_log() {
        let repository = Arc::new(MemoryRepository::default());
        let logging = Arc::new(CapturingLog::default());
        let api = CliConfigApi::new(
            repository,
            Arc::new(MemoryCredentials::default()),
            Arc::new(RollbackFailLiveConfig),
            logging.clone(),
        );
        let profile = api
            .save_profile(claude_without_credential("Failure"))
            .expect("profile");
        let result = api
            .apply_profile(ApplyCliConfigProfileInput {
                agent_id: "claude-code".into(),
                profile_id: profile.id,
                drift_resolution: None,
                confirm_auth_file_replacement: false,
            })
            .expect("structured failure");

        assert_eq!(result.status, "failed");
        assert!(!result.restored);
        assert_eq!(
            result.error.as_deref(),
            Some("configuration rollback was incomplete")
        );
        let entries = logging.entries.lock().expect("logs");
        let apply_log = entries
            .iter()
            .find(|entry| entry.category == "cli.config.profile.apply")
            .expect("apply log");
        assert_eq!(
            apply_log.context.get("status").map(String::as_str),
            Some("failed")
        );
        assert_eq!(
            apply_log.context.get("restored").map(String::as_str),
            Some("false")
        );
        assert!(!format!("{apply_log:?}").contains("secret"));
    }

    #[test]
    fn drift_import_restores_previous_credential_when_profile_save_fails() {
        let repository = Arc::new(MemoryRepository::default());
        let credentials = Arc::new(MemoryCredentials::default());
        let api = CliConfigApi::new(
            repository.clone(),
            credentials.clone(),
            Arc::new(DriftImportLiveConfig),
            Arc::new(NoopLog),
        );
        let mut previous_input = claude_without_credential("Previous");
        if let CliConfigPayload::ClaudeCode { auth_mode, .. } = &mut previous_input.payload {
            *auth_mode = ClaudeAuthMode::AuthToken;
        }
        previous_input.credential = Some("old-secret".into());
        let previous = api.save_profile(previous_input).expect("previous profile");
        let target = api
            .save_profile(claude_without_credential("Target"))
            .expect("target profile");
        repository
            .save_applied_state(&AppliedStateRecord {
                agent_id: "claude-code".into(),
                profile_id: Some(previous.id.clone()),
                projection_fingerprint: "projection".into(),
                live_fingerprint: "live".into(),
                drift_state: CliConfigDriftState::Applied,
                applied_at: "2026-01-01T00:00:00Z".into(),
                applied_payload: Some(previous.payload.clone()),
                managed_keys: previous.payload.managed_keys(),
            })
            .expect("applied state");
        repository.fail_next_save.store(1, Ordering::SeqCst);

        let result = api
            .apply_profile(ApplyCliConfigProfileInput {
                agent_id: "claude-code".into(),
                profile_id: target.id,
                drift_resolution: Some(CliConfigDriftResolution::ImportCurrent),
                confirm_auth_file_replacement: false,
            })
            .expect("structured failure");
        assert_eq!(result.status, "failed");
        assert!(result.restored);
        let restored = credentials
            .load("claude-code", &previous.id)
            .expect("credential load")
            .expect("restored credential");
        assert_eq!(restored.as_str(), "old-secret");
    }

    #[test]
    fn startup_sync_bootstraps_an_exclusive_profile_once_without_rewriting_live() {
        let directory = tempfile::tempdir().expect("temporary home");
        let settings_path = directory.path().join(".claude").join("settings.json");
        std::fs::create_dir_all(settings_path.parent().expect("settings parent"))
            .expect("create settings directory");
        let live_bytes = br#"{"env":{"ANTHROPIC_BASE_URL":"https://external.example.com","ANTHROPIC_MODEL":"claude-external"}}"#;
        std::fs::write(&settings_path, live_bytes).expect("write live settings");
        let repository = Arc::new(MemoryRepository::default());
        let api = CliConfigApi::new(
            repository.clone(),
            Arc::new(MemoryCredentials::default()),
            Arc::new(NativeCliGlobalConfigAdapter::with_home(
                directory.path().to_path_buf(),
            )),
            Arc::new(NoopLog),
        );

        let first = api.synchronize_startup();
        let claude = first
            .iter()
            .find(|result| result.agent_id == "claude-code")
            .expect("claude startup result");
        assert_eq!(claude.state, CliConfigStartupSyncState::Imported);
        assert_eq!(claude.imported, 1);
        let profiles = repository
            .list_profiles("claude-code")
            .expect("list imported profiles");
        assert_eq!(profiles.len(), 1);
        assert_eq!(profiles[0].id, "claude-code-default");
        assert_eq!(profiles[0].name, "default");
        assert_eq!(
            repository
                .applied_state("claude-code")
                .expect("applied state")
                .and_then(|state| state.profile_id),
            Some("claude-code-default".into())
        );
        assert_eq!(
            std::fs::read(&settings_path).expect("read live"),
            live_bytes
        );

        let second = api.synchronize_startup();
        assert_eq!(
            second
                .iter()
                .find(|result| result.agent_id == "claude-code")
                .expect("second claude result")
                .state,
            CliConfigStartupSyncState::Skipped
        );
        assert_eq!(
            repository
                .list_profiles("claude-code")
                .expect("list profiles after second startup")
                .len(),
            1
        );
    }

    #[test]
    fn startup_sync_upserts_opencode_by_provider_id_without_deleting_missing_profiles() {
        let directory = tempfile::tempdir().expect("temporary home");
        let config_path = directory
            .path()
            .join(".config")
            .join("opencode")
            .join("opencode.json");
        std::fs::create_dir_all(config_path.parent().expect("config parent"))
            .expect("create config directory");
        std::fs::write(
            &config_path,
            r#"{"model":"deepseek/deepseek-chat","provider":{"deepseek":{"name":"DeepSeek","npm":"@ai-sdk/openai-compatible","options":{"baseURL":"https://api.deepseek.com/v1","apiKey":"startup-secret"},"models":{"deepseek-chat":{"name":"DeepSeek Chat"}}}}}"#,
        )
        .expect("write OpenCode config");
        let repository = Arc::new(MemoryRepository::default());
        let credentials = Arc::new(MemoryCredentials::default());
        let api = CliConfigApi::new(
            repository.clone(),
            credentials.clone(),
            Arc::new(NativeCliGlobalConfigAdapter::with_home(
                directory.path().to_path_buf(),
            )),
            Arc::new(NoopLog),
        );

        let first = api.synchronize_startup();
        assert_eq!(
            first
                .iter()
                .find(|result| result.agent_id == "opencode")
                .expect("OpenCode startup result")
                .imported,
            1
        );
        let imported = repository
            .list_profiles("opencode")
            .expect("OpenCode profiles")
            .into_iter()
            .next()
            .expect("imported profile");
        assert!(credentials
            .load("opencode", &imported.id)
            .expect("credential lookup")
            .is_some());

        std::fs::write(
            &config_path,
            r#"{"model":"deepseek/deepseek-reasoner","provider":{"deepseek":{"name":"DeepSeek Updated","npm":"@ai-sdk/openai-compatible","options":{"baseURL":"https://api.deepseek.com/v2","apiKey":"updated-secret"},"models":{"deepseek-reasoner":{"name":"DeepSeek Reasoner"}}}}}"#,
        )
        .expect("update OpenCode config");
        let second = api.synchronize_startup();
        assert_eq!(
            second
                .iter()
                .find(|result| result.agent_id == "opencode")
                .expect("updated OpenCode result")
                .updated,
            1
        );
        let updated = repository
            .get_profile("opencode", &imported.id)
            .expect("updated profile");
        assert_eq!(updated.name, "DeepSeek Updated");
        assert!(matches!(
            updated.payload,
            CliConfigPayload::Opencode { ref base_url, ref default_model, .. }
                if base_url == "https://api.deepseek.com/v2" && default_model == "deepseek-reasoner"
        ));

        std::fs::write(&config_path, r#"{"provider":{}}"#).expect("remove live provider");
        api.synchronize_startup();
        assert!(repository.get_profile("opencode", &imported.id).is_ok());
    }

    #[test]
    fn exclusive_switch_backfills_external_live_changes_without_a_resolution_choice() {
        let directory = tempfile::tempdir().expect("temporary home");
        let repository = Arc::new(MemoryRepository::default());
        let api = CliConfigApi::new(
            repository.clone(),
            Arc::new(MemoryCredentials::default()),
            Arc::new(NativeCliGlobalConfigAdapter::with_home(
                directory.path().to_path_buf(),
            )),
            Arc::new(NoopLog),
        );
        let previous = api
            .save_profile(claude_without_credential("Previous"))
            .expect("previous profile");
        let target = api
            .save_profile(claude_without_credential("Target"))
            .expect("target profile");
        assert_eq!(
            api.apply_profile(ApplyCliConfigProfileInput {
                agent_id: "claude-code".into(),
                profile_id: previous.id.clone(),
                drift_resolution: None,
                confirm_auth_file_replacement: false,
            })
            .expect("apply previous")
            .status,
            "succeeded"
        );
        let settings_path = directory.path().join(".claude").join("settings.json");
        std::fs::write(
            &settings_path,
            r#"{"env":{"ANTHROPIC_BASE_URL":"https://edited.example.com","ANTHROPIC_MODEL":"claude-edited"}}"#,
        )
        .expect("external edit");

        let switched = api
            .apply_profile(ApplyCliConfigProfileInput {
                agent_id: "claude-code".into(),
                profile_id: target.id,
                drift_resolution: None,
                confirm_auth_file_replacement: false,
            })
            .expect("switch after external edit");
        assert_eq!(switched.status, "succeeded");
        assert_eq!(switched.backfilled_profile_id, Some(previous.id.clone()));
        assert_eq!(
            switched.drift_resolution,
            Some(CliConfigDriftResolution::ImportCurrent)
        );
        let stored = repository
            .get_profile("claude-code", &previous.id)
            .expect("backfilled profile");
        assert!(matches!(
            stored.payload,
            CliConfigPayload::ClaudeCode { ref base_url, ref model, .. }
                if base_url == "https://edited.example.com" && model == "claude-edited"
        ));
    }

    #[test]
    fn credential_probe_resolution_follows_each_cli_wire_protocol() {
        let claude = CliConfigPayload::ClaudeCode {
            base_url: "https://api.example.com/anthropic".into(),
            auth_mode: ClaudeAuthMode::AuthToken,
            model: "claude-test".into(),
            haiku_model: "claude-test".into(),
            sonnet_model: "claude-test".into(),
            opus_model: "claude-test".into(),
            advanced_env: BTreeMap::new(),
        };
        let claude_request = cli_probe_request(&claude, None, "secret").expect("Claude probe");
        assert_eq!(
            claude_request.protocol,
            ProviderCredentialProbeProtocol::AnthropicMessages
        );
        assert_eq!(
            claude_request.authentication,
            ProviderCredentialProbeAuthentication::Bearer
        );

        let codex = CliConfigPayload::CodexCli {
            provider_id: "example".into(),
            base_url: "https://api.example.com/v1".into(),
            model: "gpt-test".into(),
            wire_api: super::super::domain::CodexWireApi::Responses,
            reasoning_effort: "medium".into(),
            auth_strategy: super::super::domain::CodexAuthStrategy::BearerToken,
            advanced_toml: BTreeMap::new(),
        };
        assert_eq!(
            cli_probe_request(&codex, None, "secret")
                .expect("Codex probe")
                .protocol,
            ProviderCredentialProbeProtocol::OpenAiResponses
        );

        let opencode = CliConfigPayload::Opencode {
            provider_id: "example".into(),
            provider_name: "Example".into(),
            npm: "@ai-sdk/openai-compatible".into(),
            base_url: "https://api.example.com/v1".into(),
            headers: BTreeMap::new(),
            models: vec![super::super::domain::OpenCodeModelDefinition {
                id: "model-test".into(),
                name: "Model test".into(),
            }],
            default_model: "model-test".into(),
        };
        assert_eq!(
            cli_probe_request(
                &opencode,
                Some("opencode-example-openai-chat-completions"),
                "secret",
            )
            .expect("OpenCode probe")
            .protocol,
            ProviderCredentialProbeProtocol::OpenAiChatCompletions
        );
    }
}
