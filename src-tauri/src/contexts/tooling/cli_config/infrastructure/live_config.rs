use super::super::application::{
    ClaudeCodeHookProjectionPort, CliGlobalConfigPort, DiscoveredLiveConfig, ImportedLiveConfig,
    LiveConfigDiscovery, LiveInspection, ProjectionOutcome,
};
use super::super::domain::{
    AppliedStateRecord, ClaudeAuthMode, CliConfigDriftState, CliConfigError, CliConfigPayload,
    CodexAuthStrategy, CodexWireApi, OpenCodeModelDefinition, ProfileRecord,
};
use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::{Arc, Mutex};
use toml_edit::{value, DocumentMut, Item, Table};
use uuid::Uuid;
use zeroize::Zeroizing;

const CLAUDE_CORE_KEYS: [&str; 7] = [
    "ANTHROPIC_BASE_URL",
    "ANTHROPIC_AUTH_TOKEN",
    "ANTHROPIC_API_KEY",
    "ANTHROPIC_MODEL",
    "ANTHROPIC_DEFAULT_HAIKU_MODEL",
    "ANTHROPIC_DEFAULT_SONNET_MODEL",
    "ANTHROPIC_DEFAULT_OPUS_MODEL",
];

#[derive(Clone)]
pub(crate) struct NativeCliGlobalConfigAdapter {
    home_dir: PathBuf,
    locks: Arc<BTreeMap<String, Arc<Mutex<()>>>>,
}

impl NativeCliGlobalConfigAdapter {
    pub(crate) fn new() -> Result<Self, CliConfigError> {
        let home_dir = dirs::home_dir().ok_or_else(|| CliConfigError::Filesystem {
            path: "user-home".into(),
        })?;
        Ok(Self::with_home(home_dir))
    }

    pub(crate) fn with_home(home_dir: PathBuf) -> Self {
        let locks = ["claude-code", "opencode", "codex-cli"]
            .into_iter()
            .map(|agent_id| (agent_id.to_string(), Arc::new(Mutex::new(()))))
            .collect();
        Self {
            home_dir,
            locks: Arc::new(locks),
        }
    }

    fn primary_path(&self, agent_id: &str) -> Result<PathBuf, CliConfigError> {
        match agent_id {
            "claude-code" => Ok(self.home_dir.join(".claude").join("settings.json")),
            "codex-cli" => Ok(self.home_dir.join(".codex").join("config.toml")),
            "opencode" => Ok(self
                .home_dir
                .join(".config")
                .join("opencode")
                .join("opencode.json")),
            _ => Err(CliConfigError::Validation(format!(
                "unsupported CLI agent id: {agent_id}"
            ))),
        }
    }

    fn auth_path(&self) -> PathBuf {
        self.home_dir.join(".codex").join("auth.json")
    }

    fn lock_for(&self, agent_id: &str) -> Result<Arc<Mutex<()>>, CliConfigError> {
        self.locks.get(agent_id).cloned().ok_or_else(|| {
            CliConfigError::Validation(format!("unsupported CLI agent id: {agent_id}"))
        })
    }
}

impl CliGlobalConfigPort for NativeCliGlobalConfigAdapter {
    fn paths(&self, agent_id: &str) -> Result<Vec<PathBuf>, CliConfigError> {
        let primary = self.primary_path(agent_id)?;
        if agent_id == "codex-cli" {
            Ok(vec![primary, self.auth_path()])
        } else {
            Ok(vec![primary])
        }
    }

    fn inspect(
        &self,
        agent_id: &str,
        applied: Option<&AppliedStateRecord>,
        applied_profile: Option<&ProfileRecord>,
    ) -> Result<LiveInspection, CliConfigError> {
        let path = self.primary_path(agent_id)?;
        let paths = self.paths(agent_id)?;
        if !path.exists() {
            let managed_fingerprint =
                fingerprint(&managed_fragment(agent_id, &[], applied_profile)?);
            return Ok(LiveInspection {
                paths,
                state: if applied.is_some() {
                    CliConfigDriftState::Missing
                } else {
                    CliConfigDriftState::Detached
                },
                managed_fingerprint,
            });
        }
        let bytes = read_file(&path)?;
        let fragment = match managed_fragment(agent_id, &bytes, applied_profile) {
            Ok(fragment) => fragment,
            Err(CliConfigError::Parse { .. }) => {
                return Ok(LiveInspection {
                    paths,
                    state: CliConfigDriftState::Malformed,
                    managed_fingerprint: fingerprint(&bytes),
                })
            }
            Err(error) => return Err(error),
        };
        let managed_fingerprint = fingerprint(&fragment);
        let state = match applied {
            None => CliConfigDriftState::Detached,
            Some(state) if state.live_fingerprint == managed_fingerprint => {
                CliConfigDriftState::Applied
            }
            Some(_) => CliConfigDriftState::Drifted,
        };
        Ok(LiveInspection {
            paths,
            state,
            managed_fingerprint,
        })
    }

    fn import_current(&self, agent_id: &str) -> Result<ImportedLiveConfig, CliConfigError> {
        let path = self.primary_path(agent_id)?;
        if !path.exists() {
            return Err(CliConfigError::Filesystem {
                path: path.display().to_string(),
            });
        }
        let bytes = read_file(&path)?;
        let mut imported = match agent_id {
            "claude-code" => import_claude(&path, &bytes),
            "codex-cli" => import_codex(&path, &bytes),
            "opencode" => import_opencode(&path, &bytes),
            _ => Err(CliConfigError::Validation(format!(
                "unsupported CLI agent id: {agent_id}"
            ))),
        }?;
        imported.source_fingerprint = format!("file:{}", fingerprint(&bytes));
        Ok(imported)
    }

    fn discover_current(&self, agent_id: &str) -> Result<LiveConfigDiscovery, CliConfigError> {
        let path = self.primary_path(agent_id)?;
        let paths = self.paths(agent_id)?;
        if !path.exists() {
            return Ok(LiveConfigDiscovery {
                paths,
                candidates: Vec::new(),
                warnings: Vec::new(),
            });
        }
        let bytes = read_file(&path)?;
        let (candidates, warnings) = match agent_id {
            "claude-code" => (
                vec![discover_exclusive(import_claude(&path, &bytes)?, true)],
                vec![],
            ),
            "codex-cli" => (
                vec![discover_exclusive(import_codex(&path, &bytes)?, true)],
                vec![],
            ),
            "opencode" => discover_opencode(&path, &bytes)?,
            _ => {
                return Err(CliConfigError::Validation(format!(
                    "unsupported CLI agent id: {agent_id}"
                )))
            }
        };
        Ok(LiveConfigDiscovery {
            paths,
            candidates,
            warnings,
        })
    }

    fn apply(
        &self,
        profile: &ProfileRecord,
        previous: Option<&ProfileRecord>,
        credential: Option<&str>,
        confirm_auth_file_replacement: bool,
        expected_live_fingerprint: &str,
    ) -> Result<ProjectionOutcome, CliConfigError> {
        let lock = self.lock_for(&profile.agent_id)?;
        let _guard = lock.lock().map_err(|_| CliConfigError::Repository)?;
        let primary = self.primary_path(&profile.agent_id)?;
        let current_bytes = if primary.exists() {
            read_file(&primary)?
        } else {
            Vec::new()
        };
        let fingerprint_matches = match expected_live_fingerprint.strip_prefix("file:") {
            Some(expected_file_fingerprint) => {
                fingerprint(&current_bytes) == expected_file_fingerprint
            }
            None => {
                let current_fragment =
                    managed_fragment(&profile.agent_id, &current_bytes, previous)?;
                fingerprint(&current_fragment) == expected_live_fingerprint
            }
        };
        if !fingerprint_matches {
            return Err(CliConfigError::DriftConflict);
        }

        let writes = match &profile.payload {
            CliConfigPayload::ClaudeCode { .. } => vec![(
                primary.clone(),
                project_claude(&primary, &current_bytes, profile, previous, credential)?,
            )],
            CliConfigPayload::CodexCli { auth_strategy, .. } => {
                let mut writes = vec![(
                    primary.clone(),
                    project_codex(&primary, &current_bytes, profile, credential)?,
                )];
                if *auth_strategy == CodexAuthStrategy::ReplaceAuth {
                    if !confirm_auth_file_replacement {
                        return Err(CliConfigError::AuthConfirmationRequired);
                    }
                    let secret = credential.ok_or(CliConfigError::CredentialRequired)?;
                    let auth_bytes =
                        serde_json::to_vec_pretty(&json!({ "OPENAI_API_KEY": secret }))
                            .map_err(|_| CliConfigError::Repository)?;
                    writes.push((self.auth_path(), auth_bytes));
                }
                writes
            }
            CliConfigPayload::Opencode { .. } => vec![(
                primary.clone(),
                project_opencode(&primary, &current_bytes, profile, credential)?,
            )],
        };

        let snapshots = writes
            .iter()
            .map(|(path, _)| snapshot(path))
            .collect::<Result<Vec<_>, _>>()?;
        let mut changed = Vec::new();
        for (path, bytes) in &writes {
            if let Err(_error) = atomic_replace(path, bytes) {
                let restored = rollback(&snapshots, &changed);
                return if restored {
                    Err(CliConfigError::Filesystem {
                        path: path.display().to_string(),
                    })
                } else {
                    Err(CliConfigError::RollbackIncomplete)
                };
            }
            changed.push(path.clone());
        }

        let written_primary = writes
            .iter()
            .find(|(path, _)| path == &primary)
            .map(|(_, bytes)| bytes.as_slice())
            .ok_or(CliConfigError::Repository)?;
        let live_fragment = managed_fragment(&profile.agent_id, written_primary, Some(profile))?;
        let live_fingerprint = fingerprint(&live_fragment);
        Ok(ProjectionOutcome {
            paths: writes.into_iter().map(|(path, _)| path).collect(),
            projection_fingerprint: fingerprint(
                &serde_json::to_vec(&profile.payload).map_err(|_| CliConfigError::Repository)?,
            ),
            live_fingerprint,
            warnings: vec![
                "Restart running CLI processes to load the new global configuration.".into(),
            ],
            restored: true,
            backups: snapshots
                .into_iter()
                .map(|snapshot| (snapshot.path, snapshot.bytes))
                .collect(),
        })
    }

    fn restore(&self, outcome: &ProjectionOutcome) -> Result<(), CliConfigError> {
        let changed = outcome.paths.clone();
        let snapshots = outcome
            .backups
            .iter()
            .map(|(path, bytes)| FileSnapshot {
                path: path.clone(),
                bytes: bytes.clone(),
            })
            .collect::<Vec<_>>();
        if rollback(&snapshots, &changed) {
            Ok(())
        } else {
            Err(CliConfigError::RollbackIncomplete)
        }
    }
}

/// The substring every VaneHub-owned `PreToolUse` hook entry's `hooks[].command` field contains
/// — used to identify and remove only VaneHub's own entries from the array without touching
/// anything a user or another tool added, since Claude Code's hook JSON has no "owner" concept
/// to check instead. Must match the wrapper binary's own name
/// (`src/bin/vanehub-permission-hook.rs`).
const PERMISSION_HOOK_MARKER: &str = "vanehub-permission-hook";

impl ClaudeCodeHookProjectionPort for NativeCliGlobalConfigAdapter {
    fn set_permission_hook_entries(&self, entries: &[Value]) -> Result<(), CliConfigError> {
        let lock = self.lock_for("claude-code")?;
        let _guard = lock.lock().map_err(|_| CliConfigError::Repository)?;
        let path = self.primary_path("claude-code")?;

        let current_bytes = if path.exists() {
            read_file(&path)?
        } else {
            Vec::new()
        };
        let new_bytes = project_permission_hook(&path, &current_bytes, entries)?;

        // Re-read immediately before writing rather than trusting the bytes read above: this is
        // a separate, narrower drift check than `apply()`'s (no `AppliedStateRecord` to compare
        // against here — this operation has no notion of a previously-recorded expected state,
        // only "don't clobber a concurrent external edit"), matching
        // `cli-agent-config-management`'s "Live file changes during projection" scenario.
        let recheck_bytes = if path.exists() {
            read_file(&path)?
        } else {
            Vec::new()
        };
        ensure_no_concurrent_edit(&current_bytes, &recheck_bytes)?;

        let file_snapshot = snapshot(&path)?;
        if atomic_replace(&path, &new_bytes).is_err() {
            rollback(&[file_snapshot], std::slice::from_ref(&path));
            return Err(CliConfigError::Filesystem {
                path: path.display().to_string(),
            });
        }
        Ok(())
    }
}

fn project_permission_hook(
    path: &Path,
    bytes: &[u8],
    entries: &[Value],
) -> Result<Vec<u8>, CliConfigError> {
    let mut document = parse_json_or_empty_at(bytes, path)?;
    let root = document
        .as_object_mut()
        .ok_or_else(|| parse_error(path, "root must be a JSON object"))?;
    let hooks = root
        .entry("hooks")
        .or_insert_with(|| Value::Object(Map::new()))
        .as_object_mut()
        .ok_or_else(|| parse_error(path, "hooks must be a JSON object"))?;
    let pre_tool_use = hooks
        .entry("PreToolUse")
        .or_insert_with(|| Value::Array(Vec::new()))
        .as_array_mut()
        .ok_or_else(|| parse_error(path, "hooks.PreToolUse must be a JSON array"))?;

    pre_tool_use.retain(|entry| !is_vanehub_owned(entry));
    pre_tool_use.extend(entries.iter().cloned());

    serde_json::to_vec_pretty(&document).map_err(|_| CliConfigError::Repository)
}

/// Isolated from `set_permission_hook_entries` specifically so the drift-conflict rule itself —
/// "bytes read immediately before writing must match what was read at the start" — can be
/// tested as a pure comparison, without needing to fabricate a real filesystem race.
fn ensure_no_concurrent_edit(initial: &[u8], recheck: &[u8]) -> Result<(), CliConfigError> {
    if initial == recheck {
        Ok(())
    } else {
        Err(CliConfigError::DriftConflict)
    }
}

fn is_vanehub_owned(entry: &Value) -> bool {
    entry
        .get("hooks")
        .and_then(Value::as_array)
        .is_some_and(|hooks| {
            hooks.iter().any(|hook| {
                hook.get("command")
                    .and_then(Value::as_str)
                    .is_some_and(|command| command.contains(PERMISSION_HOOK_MARKER))
            })
        })
}

fn managed_fragment(
    agent_id: &str,
    bytes: &[u8],
    profile: Option<&ProfileRecord>,
) -> Result<Vec<u8>, CliConfigError> {
    match agent_id {
        "claude-code" => claude_fragment(bytes, profile),
        "codex-cli" => codex_fragment(bytes, profile),
        "opencode" => opencode_fragment(bytes, profile),
        _ => Err(CliConfigError::Validation(format!(
            "unsupported CLI agent id: {agent_id}"
        ))),
    }
}

fn claude_fragment(
    bytes: &[u8],
    profile: Option<&ProfileRecord>,
) -> Result<Vec<u8>, CliConfigError> {
    let document = parse_json_or_empty(bytes, "claude settings.json")?;
    let env = document.get("env").and_then(Value::as_object);
    let mut keys = CLAUDE_CORE_KEYS
        .iter()
        .map(|key| (*key).to_string())
        .collect::<Vec<_>>();
    if let Some(profile) = profile {
        keys.extend(profile.managed_keys.iter().cloned());
    }
    keys.sort();
    keys.dedup();
    let fragment = keys
        .into_iter()
        .filter_map(|key| {
            env.and_then(|values| values.get(&key))
                .cloned()
                .map(|value| (key, value))
        })
        .collect::<BTreeMap<_, _>>();
    serde_json::to_vec(&fragment).map_err(|_| CliConfigError::Repository)
}

fn codex_fragment(
    bytes: &[u8],
    profile: Option<&ProfileRecord>,
) -> Result<Vec<u8>, CliConfigError> {
    let document = parse_toml_or_empty(bytes, "codex config.toml")?;
    let provider_id = profile.and_then(|profile| match &profile.payload {
        CliConfigPayload::CodexCli { provider_id, .. } => Some(provider_id.as_str()),
        _ => None,
    });
    let mut fragment = BTreeMap::<String, Value>::new();
    for key in ["model", "model_provider", "model_reasoning_effort"] {
        if let Some(value) = document.get(key).and_then(Item::as_value) {
            fragment.insert(key.into(), json!(value.to_string()));
        }
    }
    if let Some(provider_id) = provider_id {
        if let Some(table) = document
            .get("model_providers")
            .and_then(Item::as_table_like)
            .and_then(|providers| providers.get(provider_id))
            .and_then(Item::as_table_like)
        {
            let entries = table
                .iter()
                .map(|(key, item)| (key.to_string(), item.to_string()))
                .collect::<BTreeMap<_, _>>();
            fragment.insert(format!("provider:{provider_id}"), json!(entries));
        }
    }
    serde_json::to_vec(&fragment).map_err(|_| CliConfigError::Repository)
}

fn opencode_fragment(
    bytes: &[u8],
    profile: Option<&ProfileRecord>,
) -> Result<Vec<u8>, CliConfigError> {
    let document = parse_json5_or_empty(bytes, "OpenCode opencode.json")?;
    let provider_id = profile.and_then(|profile| match &profile.payload {
        CliConfigPayload::Opencode { provider_id, .. } => Some(provider_id.as_str()),
        _ => None,
    });
    let mut fragment = BTreeMap::<String, Value>::new();
    if let Some(model) = document.get("model") {
        fragment.insert("model".into(), model.clone());
    }
    if let Some(provider_id) = provider_id {
        if let Some(provider) = document
            .get("provider")
            .and_then(Value::as_object)
            .and_then(|providers| providers.get(provider_id))
        {
            fragment.insert(format!("provider:{provider_id}"), provider.clone());
        }
    }
    serde_json::to_vec(&fragment).map_err(|_| CliConfigError::Repository)
}

fn project_claude(
    path: &Path,
    bytes: &[u8],
    profile: &ProfileRecord,
    previous: Option<&ProfileRecord>,
    credential: Option<&str>,
) -> Result<Vec<u8>, CliConfigError> {
    let mut document = parse_json_or_empty_at(bytes, path)?;
    let root = document
        .as_object_mut()
        .ok_or_else(|| parse_error(path, "root must be a JSON object"))?;
    let env = root
        .entry("env")
        .or_insert_with(|| Value::Object(Map::new()))
        .as_object_mut()
        .ok_or_else(|| parse_error(path, "env must be a JSON object"))?;
    if let Some(previous) = previous {
        for key in &previous.managed_keys {
            env.remove(key);
        }
    }
    for key in CLAUDE_CORE_KEYS {
        env.remove(key);
    }
    let CliConfigPayload::ClaudeCode {
        base_url,
        auth_mode,
        model,
        haiku_model,
        sonnet_model,
        opus_model,
        advanced_env,
    } = &profile.payload
    else {
        return Err(CliConfigError::Validation("invalid Claude payload".into()));
    };
    env.insert("ANTHROPIC_BASE_URL".into(), json!(base_url));
    env.insert("ANTHROPIC_MODEL".into(), json!(model));
    env.insert("ANTHROPIC_DEFAULT_HAIKU_MODEL".into(), json!(haiku_model));
    env.insert("ANTHROPIC_DEFAULT_SONNET_MODEL".into(), json!(sonnet_model));
    env.insert("ANTHROPIC_DEFAULT_OPUS_MODEL".into(), json!(opus_model));
    match auth_mode {
        ClaudeAuthMode::AuthToken => {
            env.insert(
                "ANTHROPIC_AUTH_TOKEN".into(),
                json!(credential.ok_or(CliConfigError::CredentialRequired)?),
            );
        }
        ClaudeAuthMode::ApiKey => {
            env.insert(
                "ANTHROPIC_API_KEY".into(),
                json!(credential.ok_or(CliConfigError::CredentialRequired)?),
            );
        }
        ClaudeAuthMode::None => {}
    }
    for (key, value) in advanced_env {
        env.insert(key.clone(), json!(value));
    }
    serde_json::to_vec_pretty(&document).map_err(|_| CliConfigError::Repository)
}

fn project_codex(
    path: &Path,
    bytes: &[u8],
    profile: &ProfileRecord,
    credential: Option<&str>,
) -> Result<Vec<u8>, CliConfigError> {
    let mut document = parse_toml_or_empty_at(bytes, path)?;
    let CliConfigPayload::CodexCli {
        provider_id,
        base_url,
        model,
        wire_api,
        reasoning_effort,
        auth_strategy,
        advanced_toml,
    } = &profile.payload
    else {
        return Err(CliConfigError::Validation("invalid Codex payload".into()));
    };
    document["model"] = value(model);
    document["model_provider"] = value(provider_id);
    if reasoning_effort == "none" {
        document.remove("model_reasoning_effort");
    } else {
        document["model_reasoning_effort"] = value(reasoning_effort);
    }
    if document
        .get("model_providers")
        .and_then(Item::as_table)
        .is_none()
    {
        document.insert("model_providers", Item::Table(Table::new()));
    }
    let providers = document
        .get_mut("model_providers")
        .and_then(Item::as_table_mut)
        .ok_or(CliConfigError::Repository)?;
    if providers
        .get(provider_id)
        .and_then(Item::as_table)
        .is_none()
    {
        providers.insert(provider_id, Item::Table(Table::new()));
    }
    let provider = providers
        .get_mut(provider_id)
        .and_then(Item::as_table_mut)
        .ok_or(CliConfigError::Repository)?;
    provider["name"] = value(provider_id);
    provider["base_url"] = value(base_url);
    provider["wire_api"] = value(match wire_api {
        CodexWireApi::Responses => "responses",
        CodexWireApi::Chat => "chat",
    });
    provider.remove("experimental_bearer_token");
    if *auth_strategy == CodexAuthStrategy::BearerToken {
        provider["experimental_bearer_token"] =
            value(credential.ok_or(CliConfigError::CredentialRequired)?);
    }
    for (key, entry) in advanced_toml {
        provider[key] = json_value_to_toml(entry)?;
    }
    Ok(document.to_string().into_bytes())
}

fn project_opencode(
    path: &Path,
    bytes: &[u8],
    profile: &ProfileRecord,
    credential: Option<&str>,
) -> Result<Vec<u8>, CliConfigError> {
    let mut document = parse_json5_or_empty_at(bytes, path)?;
    let root = document
        .as_object_mut()
        .ok_or_else(|| parse_error(path, "root must be a JSON object"))?;
    let CliConfigPayload::Opencode {
        provider_id,
        provider_name,
        npm,
        base_url,
        headers,
        models,
        default_model,
    } = &profile.payload
    else {
        return Err(CliConfigError::Validation(
            "invalid OpenCode payload".into(),
        ));
    };
    let providers = root
        .entry("provider")
        .or_insert_with(|| Value::Object(Map::new()))
        .as_object_mut()
        .ok_or_else(|| parse_error(path, "provider must be a JSON object"))?;
    let models = models
        .iter()
        .map(|model| (model.id.clone(), json!({ "name": model.name })))
        .collect::<Map<_, _>>();
    providers.insert(
        provider_id.clone(),
        json!({
            "name": provider_name,
            "npm": npm,
            "options": {
                "baseURL": base_url,
                "apiKey": credential.ok_or(CliConfigError::CredentialRequired)?,
                "headers": headers,
            },
            "models": models,
        }),
    );
    root.insert(
        "model".into(),
        json!(format!("{provider_id}/{default_model}")),
    );
    serde_json::to_vec_pretty(&document).map_err(|_| CliConfigError::Repository)
}

fn import_claude(path: &Path, bytes: &[u8]) -> Result<ImportedLiveConfig, CliConfigError> {
    let document = parse_json_or_empty_at(bytes, path)?;
    let env = document
        .get("env")
        .and_then(Value::as_object)
        .ok_or_else(|| parse_error(path, "env must be a JSON object"))?;
    let credential = env
        .get("ANTHROPIC_AUTH_TOKEN")
        .or_else(|| env.get("ANTHROPIC_API_KEY"))
        .and_then(Value::as_str)
        .map(|secret| Zeroizing::new(secret.to_string()));
    let auth_mode = if env.contains_key("ANTHROPIC_AUTH_TOKEN") {
        ClaudeAuthMode::AuthToken
    } else if env.contains_key("ANTHROPIC_API_KEY") {
        ClaudeAuthMode::ApiKey
    } else {
        ClaudeAuthMode::None
    };
    let string = |key: &str, fallback: &str| {
        env.get(key)
            .and_then(Value::as_str)
            .unwrap_or(fallback)
            .to_string()
    };
    let model = string("ANTHROPIC_MODEL", "default");
    Ok(ImportedLiveConfig {
        payload: CliConfigPayload::ClaudeCode {
            base_url: string("ANTHROPIC_BASE_URL", "https://api.anthropic.com"),
            auth_mode,
            haiku_model: string("ANTHROPIC_DEFAULT_HAIKU_MODEL", &model),
            sonnet_model: string("ANTHROPIC_DEFAULT_SONNET_MODEL", &model),
            opus_model: string("ANTHROPIC_DEFAULT_OPUS_MODEL", &model),
            model,
            advanced_env: BTreeMap::new(),
        },
        credential,
        source_fingerprint: String::new(),
    })
}

fn import_codex(path: &Path, bytes: &[u8]) -> Result<ImportedLiveConfig, CliConfigError> {
    let document = parse_toml_or_empty_at(bytes, path)?;
    let provider_id = document
        .get("model_provider")
        .and_then(Item::as_str)
        .unwrap_or("openai")
        .to_string();
    let model = document
        .get("model")
        .and_then(Item::as_str)
        .unwrap_or("gpt-5.4")
        .to_string();
    let provider = document
        .get("model_providers")
        .and_then(Item::as_table_like)
        .and_then(|providers| providers.get(&provider_id))
        .and_then(Item::as_table_like);
    let base_url = provider
        .and_then(|table| table.get("base_url"))
        .and_then(Item::as_str)
        .unwrap_or("https://api.openai.com/v1")
        .to_string();
    let credential = provider
        .and_then(|table| table.get("experimental_bearer_token"))
        .and_then(Item::as_str)
        .map(|secret| Zeroizing::new(secret.to_string()));
    let wire_api = match provider
        .and_then(|table| table.get("wire_api"))
        .and_then(Item::as_str)
    {
        Some("chat") => CodexWireApi::Chat,
        _ => CodexWireApi::Responses,
    };
    Ok(ImportedLiveConfig {
        payload: CliConfigPayload::CodexCli {
            provider_id,
            base_url,
            model,
            wire_api,
            reasoning_effort: document
                .get("model_reasoning_effort")
                .and_then(Item::as_str)
                .unwrap_or("medium")
                .to_string(),
            auth_strategy: if credential.is_some() {
                CodexAuthStrategy::BearerToken
            } else {
                CodexAuthStrategy::PreserveOfficial
            },
            advanced_toml: BTreeMap::new(),
        },
        credential,
        source_fingerprint: String::new(),
    })
}

fn import_opencode(path: &Path, bytes: &[u8]) -> Result<ImportedLiveConfig, CliConfigError> {
    let document = parse_json5_or_empty_at(bytes, path)?;
    let selected = document
        .get("model")
        .and_then(Value::as_str)
        .ok_or_else(|| parse_error(path, "global model must be provider/model"))?;
    let (provider_id, default_model) = selected
        .split_once('/')
        .ok_or_else(|| parse_error(path, "global model must be provider/model"))?;
    let provider = document
        .get("provider")
        .and_then(Value::as_object)
        .and_then(|providers| providers.get(provider_id))
        .and_then(Value::as_object)
        .ok_or_else(|| parse_error(path, "selected provider is missing"))?;
    let options = provider
        .get("options")
        .and_then(Value::as_object)
        .ok_or_else(|| parse_error(path, "provider options are missing"))?;
    let models = provider
        .get("models")
        .and_then(Value::as_object)
        .map(|models| {
            models
                .iter()
                .map(|(id, value)| OpenCodeModelDefinition {
                    id: id.clone(),
                    name: value
                        .get("name")
                        .and_then(Value::as_str)
                        .unwrap_or(id)
                        .to_string(),
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_else(|| {
            vec![OpenCodeModelDefinition {
                id: default_model.to_string(),
                name: default_model.to_string(),
            }]
        });
    let headers = options
        .get("headers")
        .and_then(Value::as_object)
        .map(|headers| {
            headers
                .iter()
                .filter_map(|(key, value)| {
                    value.as_str().map(|value| (key.clone(), value.to_string()))
                })
                .collect()
        })
        .unwrap_or_default();
    Ok(ImportedLiveConfig {
        payload: CliConfigPayload::Opencode {
            provider_id: provider_id.to_string(),
            provider_name: provider
                .get("name")
                .and_then(Value::as_str)
                .unwrap_or(provider_id)
                .to_string(),
            npm: provider
                .get("npm")
                .and_then(Value::as_str)
                .unwrap_or("@ai-sdk/openai-compatible")
                .to_string(),
            base_url: options
                .get("baseURL")
                .and_then(Value::as_str)
                .unwrap_or("https://api.openai.com/v1")
                .to_string(),
            headers,
            models,
            default_model: default_model.to_string(),
        },
        credential: options
            .get("apiKey")
            .and_then(Value::as_str)
            .map(|secret| Zeroizing::new(secret.to_string())),
        source_fingerprint: String::new(),
    })
}

fn discover_exclusive(imported: ImportedLiveConfig, is_default: bool) -> DiscoveredLiveConfig {
    let (suggested_name, provider_name, endpoint, model) = match &imported.payload {
        CliConfigPayload::ClaudeCode {
            base_url, model, ..
        } => (
            "Local Claude Code".to_string(),
            endpoint_provider_name(base_url, "Anthropic"),
            base_url.clone(),
            model.clone(),
        ),
        CliConfigPayload::CodexCli {
            provider_id,
            base_url,
            model,
            ..
        } => (
            provider_id.clone(),
            provider_id.clone(),
            base_url.clone(),
            model.clone(),
        ),
        CliConfigPayload::Opencode { .. } => unreachable!("exclusive discovery is not OpenCode"),
    };
    DiscoveredLiveConfig {
        candidate_key: "current".to_string(),
        suggested_name,
        provider_name,
        endpoint,
        model,
        is_default,
        payload: imported.payload,
        credential: imported.credential,
    }
}

fn discover_opencode(
    path: &Path,
    bytes: &[u8],
) -> Result<(Vec<DiscoveredLiveConfig>, Vec<String>), CliConfigError> {
    let document = parse_json5_or_empty_at(bytes, path)?;
    let root = document
        .as_object()
        .ok_or_else(|| parse_error(path, "root must be a JSON object"))?;
    let providers = match root.get("provider") {
        None => return Ok((Vec::new(), Vec::new())),
        Some(value) => value
            .as_object()
            .ok_or_else(|| parse_error(path, "provider must be a JSON object"))?,
    };
    let selected = root.get("model").and_then(Value::as_str);
    let mut candidates = Vec::new();
    let mut warnings = Vec::new();
    for (provider_id, value) in providers {
        let Some(provider) = value.as_object() else {
            warnings.push("Skipped one provider because it is not an object.".into());
            continue;
        };
        let Some(options) = provider.get("options").and_then(Value::as_object) else {
            warnings.push("Skipped one provider because options are missing.".into());
            continue;
        };
        let selected_model = selected.and_then(|value| {
            value
                .split_once('/')
                .filter(|(selected_provider, _)| *selected_provider == provider_id)
                .map(|(_, model)| model.to_string())
        });
        let models = provider
            .get("models")
            .and_then(Value::as_object)
            .map(|models| {
                models
                    .iter()
                    .map(|(id, value)| OpenCodeModelDefinition {
                        id: id.clone(),
                        name: value
                            .get("name")
                            .and_then(Value::as_str)
                            .unwrap_or(id)
                            .to_string(),
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let default_model = selected_model
            .clone()
            .or_else(|| models.first().map(|model| model.id.clone()));
        let Some(default_model) = default_model else {
            warnings.push("Skipped one provider because no model is configured.".into());
            continue;
        };
        let provider_name = provider
            .get("name")
            .and_then(Value::as_str)
            .unwrap_or(provider_id)
            .to_string();
        let base_url = options
            .get("baseURL")
            .and_then(Value::as_str)
            .unwrap_or("https://api.openai.com/v1")
            .to_string();
        let headers = options
            .get("headers")
            .and_then(Value::as_object)
            .map(|headers| {
                headers
                    .iter()
                    .filter_map(|(key, value)| {
                        value.as_str().map(|value| (key.clone(), value.to_string()))
                    })
                    .collect()
            })
            .unwrap_or_default();
        let payload = CliConfigPayload::Opencode {
            provider_id: provider_id.clone(),
            provider_name: provider_name.clone(),
            npm: provider
                .get("npm")
                .and_then(Value::as_str)
                .unwrap_or("@ai-sdk/openai-compatible")
                .to_string(),
            base_url: base_url.clone(),
            headers,
            models: if models.is_empty() {
                vec![OpenCodeModelDefinition {
                    id: default_model.clone(),
                    name: default_model.clone(),
                }]
            } else {
                models
            },
            default_model: default_model.clone(),
        };
        if payload.validate().is_err() {
            warnings
                .push("Skipped one provider because its managed fields are incompatible.".into());
            continue;
        }
        candidates.push(DiscoveredLiveConfig {
            candidate_key: provider_id.clone(),
            suggested_name: provider_name.clone(),
            provider_name,
            endpoint: base_url,
            model: default_model,
            is_default: selected_model.is_some(),
            payload,
            credential: options
                .get("apiKey")
                .and_then(Value::as_str)
                .map(|secret| Zeroizing::new(secret.to_string())),
        });
    }
    Ok((candidates, warnings))
}

fn endpoint_provider_name(endpoint: &str, official_name: &str) -> String {
    if endpoint.contains("anthropic.com") {
        official_name.to_string()
    } else {
        endpoint
            .split("//")
            .nth(1)
            .and_then(|value| value.split('/').next())
            .unwrap_or(endpoint)
            .to_string()
    }
}

fn parse_json_or_empty(bytes: &[u8], label: &str) -> Result<Value, CliConfigError> {
    if bytes.is_empty() {
        return Ok(json!({}));
    }
    serde_json::from_slice(bytes).map_err(|error| CliConfigError::Parse {
        path: label.into(),
        message: error.to_string(),
    })
}

fn parse_json_or_empty_at(bytes: &[u8], path: &Path) -> Result<Value, CliConfigError> {
    parse_json_or_empty(bytes, &path.display().to_string())
}

fn parse_json5_or_empty(bytes: &[u8], label: &str) -> Result<Value, CliConfigError> {
    if bytes.is_empty() {
        return Ok(json!({}));
    }
    let text = std::str::from_utf8(bytes).map_err(|error| CliConfigError::Parse {
        path: label.into(),
        message: error.to_string(),
    })?;
    json5::from_str(text).map_err(|error| CliConfigError::Parse {
        path: label.into(),
        message: error.to_string(),
    })
}

fn parse_json5_or_empty_at(bytes: &[u8], path: &Path) -> Result<Value, CliConfigError> {
    parse_json5_or_empty(bytes, &path.display().to_string())
}

fn parse_toml_or_empty(bytes: &[u8], label: &str) -> Result<DocumentMut, CliConfigError> {
    if bytes.is_empty() {
        return Ok(DocumentMut::new());
    }
    let text = std::str::from_utf8(bytes).map_err(|error| CliConfigError::Parse {
        path: label.into(),
        message: error.to_string(),
    })?;
    DocumentMut::from_str(text).map_err(|error| CliConfigError::Parse {
        path: label.into(),
        message: error.to_string(),
    })
}

fn parse_toml_or_empty_at(bytes: &[u8], path: &Path) -> Result<DocumentMut, CliConfigError> {
    parse_toml_or_empty(bytes, &path.display().to_string())
}

fn json_value_to_toml(entry: &Value) -> Result<Item, CliConfigError> {
    match entry {
        Value::String(entry_value) => Ok(value(entry_value)),
        Value::Bool(entry_value) => Ok(value(*entry_value)),
        Value::Number(number) if number.is_i64() => Ok(value(number.as_i64().unwrap_or_default())),
        Value::Number(number) if number.is_f64() => Ok(value(number.as_f64().unwrap_or_default())),
        _ => Err(CliConfigError::Validation(
            "advanced TOML value must be scalar".into(),
        )),
    }
}

fn read_file(path: &Path) -> Result<Vec<u8>, CliConfigError> {
    fs::read(path).map_err(|_| CliConfigError::Filesystem {
        path: path.display().to_string(),
    })
}

#[derive(Debug)]
struct FileSnapshot {
    path: PathBuf,
    bytes: Option<Vec<u8>>,
}

fn snapshot(path: &Path) -> Result<FileSnapshot, CliConfigError> {
    let bytes = if path.exists() {
        Some(read_file(path)?)
    } else {
        None
    };
    Ok(FileSnapshot {
        path: path.to_path_buf(),
        bytes,
    })
}

fn atomic_replace(path: &Path, bytes: &[u8]) -> Result<(), std::io::Error> {
    let parent = path
        .parent()
        .ok_or_else(|| std::io::Error::other("target has no parent"))?;
    fs::create_dir_all(parent)?;
    let temp = parent.join(format!(".vanehub-{}.tmp", Uuid::new_v4()));
    let result = (|| {
        let mut file = File::create(&temp)?;
        file.write_all(bytes)?;
        file.sync_all()?;
        restrict_file_permissions(&temp)?;
        replace_path(&temp, path)
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temp);
    }
    result
}

#[cfg(windows)]
fn replace_path(source: &Path, target: &Path) -> Result<(), std::io::Error> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::{
        MoveFileExW, MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH,
    };
    let source = source
        .as_os_str()
        .encode_wide()
        .chain(Some(0))
        .collect::<Vec<_>>();
    let target = target
        .as_os_str()
        .encode_wide()
        .chain(Some(0))
        .collect::<Vec<_>>();
    let success = unsafe {
        MoveFileExW(
            source.as_ptr(),
            target.as_ptr(),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
        )
    };
    if success == 0 {
        Err(std::io::Error::last_os_error())
    } else {
        Ok(())
    }
}

#[cfg(not(windows))]
fn replace_path(source: &Path, target: &Path) -> Result<(), std::io::Error> {
    fs::rename(source, target)
}

#[cfg(unix)]
fn restrict_file_permissions(path: &Path) -> Result<(), std::io::Error> {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(0o600))
}

#[cfg(not(unix))]
fn restrict_file_permissions(_path: &Path) -> Result<(), std::io::Error> {
    Ok(())
}

fn rollback(snapshots: &[FileSnapshot], changed: &[PathBuf]) -> bool {
    snapshots
        .iter()
        .filter(|snapshot| changed.contains(&snapshot.path))
        .rev()
        .all(|snapshot| match &snapshot.bytes {
            Some(bytes) => atomic_replace(&snapshot.path, bytes).is_ok(),
            None if snapshot.path.exists() => fs::remove_file(&snapshot.path).is_ok(),
            None => true,
        })
}

fn fingerprint(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn parse_error(path: &Path, message: &str) -> CliConfigError {
    CliConfigError::Parse {
        path: path.display().to_string(),
        message: message.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contexts::tooling::cli_config::domain::PAYLOAD_VERSION;
    use crate::test_support::TempDirectory;

    fn profile(agent_id: &str, payload: CliConfigPayload) -> ProfileRecord {
        ProfileRecord {
            id: format!("{agent_id}-profile"),
            agent_id: agent_id.into(),
            name: "Profile".into(),
            payload_version: PAYLOAD_VERSION,
            managed_keys: payload.managed_keys(),
            payload,
            source_preset_id: None,
            source_preset_version: None,
            created_at: "2026-01-01T00:00:00Z".into(),
            updated_at: "2026-01-01T00:00:00Z".into(),
            sort_position: 0,
        }
    }

    #[test]
    fn claude_projection_preserves_unmanaged_settings_and_removes_old_keys() {
        let directory = TempDirectory::new("cli-config-claude");
        let adapter = NativeCliGlobalConfigAdapter::with_home(directory.path().to_path_buf());
        let path = adapter.primary_path("claude-code").expect("path");
        fs::create_dir_all(path.parent().expect("parent")).expect("directory");
        fs::write(
            &path,
            br#"{"permissions":{"allow":["Read"]},"env":{"UNRELATED":"kept","OLD_MANAGED":"old"}}"#,
        )
        .expect("fixture");
        let previous = profile(
            "claude-code",
            CliConfigPayload::ClaudeCode {
                base_url: "https://old.example.com".into(),
                auth_mode: ClaudeAuthMode::AuthToken,
                model: "old".into(),
                haiku_model: "old".into(),
                sonnet_model: "old".into(),
                opus_model: "old".into(),
                advanced_env: BTreeMap::from([("OLD_MANAGED".into(), "old".into())]),
            },
        );
        let current = profile(
            "claude-code",
            CliConfigPayload::ClaudeCode {
                base_url: "https://api.example.com".into(),
                auth_mode: ClaudeAuthMode::AuthToken,
                model: "model".into(),
                haiku_model: "haiku".into(),
                sonnet_model: "sonnet".into(),
                opus_model: "opus".into(),
                advanced_env: BTreeMap::new(),
            },
        );
        let before = fs::read(&path).expect("read");
        let expected = fingerprint(
            &managed_fragment("claude-code", &before, Some(&previous)).expect("fragment"),
        );
        adapter
            .apply(
                &current,
                Some(&previous),
                Some("top-secret"),
                false,
                &expected,
            )
            .expect("apply");
        let written: Value =
            serde_json::from_slice(&fs::read(&path).expect("written")).expect("json");
        assert_eq!(written["permissions"]["allow"][0], "Read");
        assert_eq!(written["env"]["UNRELATED"], "kept");
        assert!(written["env"].get("OLD_MANAGED").is_none());
        assert_eq!(written["env"]["ANTHROPIC_AUTH_TOKEN"], "top-secret");
    }

    #[test]
    fn installing_the_permission_hook_preserves_unrelated_hooks_and_top_level_fields() {
        let directory = TempDirectory::new("cli-config-hook-install");
        let adapter = NativeCliGlobalConfigAdapter::with_home(directory.path().to_path_buf());
        let path = adapter.primary_path("claude-code").expect("path");
        fs::create_dir_all(path.parent().expect("parent")).expect("directory");
        fs::write(
            &path,
            br#"{"env":{"UNRELATED":"kept"},"hooks":{"PreToolUse":[{"matcher":"Bash","hooks":[{"type":"command","command":"my-other-tool"}]}],"PostToolUse":[{"matcher":"*","hooks":[{"type":"command","command":"unrelated"}]}]}}"#,
        )
        .expect("fixture");

        let entry = json!({
            "matcher": "Bash",
            "hooks": [{"type": "command", "command": "/opt/vanehub/bin/vanehub-permission-hook"}]
        });
        adapter
            .set_permission_hook_entries(&[entry])
            .expect("install");

        let written: Value =
            serde_json::from_slice(&fs::read(&path).expect("written")).expect("json");
        assert_eq!(written["env"]["UNRELATED"], "kept");
        assert_eq!(written["hooks"]["PostToolUse"][0]["matcher"], "*");
        let pre_tool_use = written["hooks"]["PreToolUse"].as_array().expect("array");
        assert_eq!(pre_tool_use.len(), 2, "the other tool's entry must survive");
        assert!(pre_tool_use
            .iter()
            .any(|entry| entry["hooks"][0]["command"] == "my-other-tool"));
        assert!(pre_tool_use.iter().any(|entry| entry["hooks"][0]
            ["command"]
            .as_str()
            .unwrap()
            .contains("vanehub-permission-hook")));
    }

    #[test]
    fn reinstalling_the_permission_hook_replaces_rather_than_duplicates() {
        let directory = TempDirectory::new("cli-config-hook-reinstall");
        let adapter = NativeCliGlobalConfigAdapter::with_home(directory.path().to_path_buf());
        let entry = json!({
            "matcher": "Bash",
            "hooks": [{"type": "command", "command": "/opt/vanehub/bin/vanehub-permission-hook"}]
        });

        adapter
            .set_permission_hook_entries(std::slice::from_ref(&entry))
            .expect("first install");
        adapter
            .set_permission_hook_entries(&[entry])
            .expect("second install");

        let path = adapter.primary_path("claude-code").expect("path");
        let written: Value =
            serde_json::from_slice(&fs::read(&path).expect("written")).expect("json");
        assert_eq!(written["hooks"]["PreToolUse"].as_array().expect("array").len(), 1);
    }

    #[test]
    fn removing_the_permission_hook_only_removes_vanehubs_own_entry() {
        let directory = TempDirectory::new("cli-config-hook-remove");
        let adapter = NativeCliGlobalConfigAdapter::with_home(directory.path().to_path_buf());
        let path = adapter.primary_path("claude-code").expect("path");
        fs::create_dir_all(path.parent().expect("parent")).expect("directory");
        fs::write(
            &path,
            br#"{"hooks":{"PreToolUse":[{"matcher":"Bash","hooks":[{"type":"command","command":"my-other-tool"}]}]}}"#,
        )
        .expect("fixture");
        adapter
            .set_permission_hook_entries(&[json!({
                "matcher": "Bash",
                "hooks": [{"type": "command", "command": "/opt/vanehub/bin/vanehub-permission-hook"}]
            })])
            .expect("install");

        adapter
            .set_permission_hook_entries(&[])
            .expect("remove");

        let written: Value =
            serde_json::from_slice(&fs::read(&path).expect("written")).expect("json");
        let pre_tool_use = written["hooks"]["PreToolUse"].as_array().expect("array");
        assert_eq!(pre_tool_use.len(), 1);
        assert_eq!(pre_tool_use[0]["hooks"][0]["command"], "my-other-tool");
    }

    #[test]
    fn permission_hook_projection_rejects_a_malformed_file_without_modifying_it() {
        let directory = TempDirectory::new("cli-config-hook-malformed");
        let adapter = NativeCliGlobalConfigAdapter::with_home(directory.path().to_path_buf());
        let path = adapter.primary_path("claude-code").expect("path");
        fs::create_dir_all(path.parent().expect("parent")).expect("directory");
        fs::write(&path, b"{not valid json").expect("fixture");

        let result = adapter.set_permission_hook_entries(&[json!({"matcher": "Bash"})]);

        assert!(matches!(result, Err(CliConfigError::Parse { .. })));
        assert_eq!(fs::read(&path).expect("unchanged"), b"{not valid json");
    }

    #[test]
    fn concurrent_edit_between_the_initial_read_and_the_pre_write_recheck_is_rejected() {
        let original = br#"{"env":{"ORIGINAL":"value"}}"#;
        let edited_externally = br#"{"env":{"EXTERNALLY_EDITED":"value"}}"#;

        assert!(matches!(
            ensure_no_concurrent_edit(original, edited_externally),
            Err(CliConfigError::DriftConflict)
        ));
    }

    #[test]
    fn no_concurrent_edit_is_accepted() {
        let bytes = br#"{"env":{"ORIGINAL":"value"}}"#;
        assert!(ensure_no_concurrent_edit(bytes, bytes).is_ok());
    }

    #[test]
    fn codex_projection_preserves_comments_and_unrelated_tables() {
        let directory = TempDirectory::new("cli-config-codex");
        let adapter = NativeCliGlobalConfigAdapter::with_home(directory.path().to_path_buf());
        let path = adapter.primary_path("codex-cli").expect("path");
        fs::create_dir_all(path.parent().expect("parent")).expect("directory");
        fs::write(&path, "# keep me\n[mcp_servers.demo]\ncommand = \"node\"\n").expect("fixture");
        let current = profile(
            "codex-cli",
            CliConfigPayload::CodexCli {
                provider_id: "openrouter".into(),
                base_url: "https://openrouter.ai/api/v1".into(),
                model: "openai/gpt-5.4".into(),
                wire_api: CodexWireApi::Responses,
                reasoning_effort: "high".into(),
                auth_strategy: CodexAuthStrategy::BearerToken,
                advanced_toml: BTreeMap::new(),
            },
        );
        let before = fs::read(&path).expect("read");
        let expected =
            fingerprint(&managed_fragment("codex-cli", &before, None).expect("fragment"));
        adapter
            .apply(&current, None, Some("secret"), false, &expected)
            .expect("apply");
        let written = fs::read_to_string(path).expect("written");
        assert!(written.contains("# keep me"));
        assert!(written.contains("[mcp_servers.demo]"));
        assert!(written.contains("[model_providers.openrouter]"));
    }

    #[test]
    fn opencode_projection_accepts_json5_and_preserves_other_providers() {
        let directory = TempDirectory::new("cli-config-opencode");
        let adapter = NativeCliGlobalConfigAdapter::with_home(directory.path().to_path_buf());
        let path = adapter.primary_path("opencode").expect("path");
        fs::create_dir_all(path.parent().expect("parent")).expect("directory");
        fs::write(
            &path,
            "{ provider: { existing: { npm: '@ai-sdk/openai' } }, plugins: ['kept'], }",
        )
        .expect("fixture");
        let current = profile(
            "opencode",
            CliConfigPayload::Opencode {
                provider_id: "deepseek".into(),
                provider_name: "DeepSeek".into(),
                npm: "@ai-sdk/openai-compatible".into(),
                base_url: "https://api.deepseek.com/v1".into(),
                headers: BTreeMap::new(),
                models: vec![OpenCodeModelDefinition {
                    id: "deepseek-chat".into(),
                    name: "DeepSeek Chat".into(),
                }],
                default_model: "deepseek-chat".into(),
            },
        );
        let before = fs::read(&path).expect("read");
        let expected = fingerprint(&managed_fragment("opencode", &before, None).expect("fragment"));
        adapter
            .apply(&current, None, Some("secret"), false, &expected)
            .expect("apply");
        let written: Value =
            serde_json::from_slice(&fs::read(path).expect("written")).expect("json");
        assert_eq!(written["model"], "deepseek/deepseek-chat");
        assert!(written["provider"].get("existing").is_some());
        assert_eq!(written["plugins"][0], "kept");
    }

    #[test]
    fn drift_check_rejects_a_changed_apply_plan() {
        let directory = TempDirectory::new("cli-config-drift");
        let adapter = NativeCliGlobalConfigAdapter::with_home(directory.path().to_path_buf());
        let profile = profile(
            "claude-code",
            CliConfigPayload::ClaudeCode {
                base_url: "https://api.anthropic.com".into(),
                auth_mode: ClaudeAuthMode::None,
                model: "claude".into(),
                haiku_model: "claude".into(),
                sonnet_model: "claude".into(),
                opus_model: "claude".into(),
                advanced_env: BTreeMap::new(),
            },
        );
        assert!(matches!(
            adapter.apply(&profile, None, None, false, "stale"),
            Err(CliConfigError::DriftConflict)
        ));
    }

    #[test]
    fn first_claude_apply_creates_a_valid_file() {
        let directory = TempDirectory::new("cli-config-claude-first");
        let adapter = NativeCliGlobalConfigAdapter::with_home(directory.path().to_path_buf());
        let current = profile(
            "claude-code",
            CliConfigPayload::ClaudeCode {
                base_url: "https://api.anthropic.com".into(),
                auth_mode: ClaudeAuthMode::None,
                model: "claude-sonnet-4-6".into(),
                haiku_model: "claude-haiku-4-5".into(),
                sonnet_model: "claude-sonnet-4-6".into(),
                opus_model: "claude-opus-4-6".into(),
                advanced_env: BTreeMap::new(),
            },
        );
        let expected = fingerprint(
            &managed_fragment("claude-code", &[], None).expect("empty managed fragment"),
        );

        adapter
            .apply(&current, None, None, false, &expected)
            .expect("first apply");
        let path = adapter.primary_path("claude-code").expect("path");
        let written: Value =
            serde_json::from_slice(&fs::read(path).expect("written file")).expect("valid JSON");
        assert_eq!(written["env"]["ANTHROPIC_MODEL"], "claude-sonnet-4-6");
    }

    #[test]
    fn malformed_live_documents_are_reported_without_modification() {
        for (agent_id, relative, body) in [
            ("claude-code", ".claude/settings.json", "{broken"),
            ("codex-cli", ".codex/config.toml", "model = [broken"),
            (
                "opencode",
                ".config/opencode/opencode.json",
                "{provider: broken",
            ),
        ] {
            let directory = TempDirectory::new(&format!("cli-config-malformed-{agent_id}"));
            let adapter = NativeCliGlobalConfigAdapter::with_home(directory.path().to_path_buf());
            let path = directory.path().join(relative);
            fs::create_dir_all(path.parent().expect("parent")).expect("directory");
            fs::write(&path, body).expect("fixture");

            let inspection = adapter.inspect(agent_id, None, None).expect("inspection");
            assert_eq!(inspection.state, CliConfigDriftState::Malformed);
            assert_eq!(fs::read_to_string(path).expect("unchanged"), body);
        }
    }

    #[test]
    fn codex_preserves_official_auth_and_requires_confirmation_to_replace_it() {
        let directory = TempDirectory::new("cli-config-codex-auth");
        let adapter = NativeCliGlobalConfigAdapter::with_home(directory.path().to_path_buf());
        let auth_path = adapter.auth_path();
        fs::create_dir_all(auth_path.parent().expect("parent")).expect("directory");
        let official_auth = br#"{"tokens":{"access_token":"official"}}"#;
        fs::write(&auth_path, official_auth).expect("auth fixture");
        let mut current = profile(
            "codex-cli",
            CliConfigPayload::CodexCli {
                provider_id: "openai".into(),
                base_url: "https://api.openai.com/v1".into(),
                model: "gpt-5.4".into(),
                wire_api: CodexWireApi::Responses,
                reasoning_effort: "medium".into(),
                auth_strategy: CodexAuthStrategy::PreserveOfficial,
                advanced_toml: BTreeMap::new(),
            },
        );
        let expected = fingerprint(&managed_fragment("codex-cli", &[], None).expect("fragment"));
        adapter
            .apply(&current, None, None, false, &expected)
            .expect("official apply");
        assert_eq!(fs::read(&auth_path).expect("auth"), official_auth);

        if let CliConfigPayload::CodexCli { auth_strategy, .. } = &mut current.payload {
            *auth_strategy = CodexAuthStrategy::ReplaceAuth;
        }
        let config =
            fs::read(adapter.primary_path("codex-cli").expect("config path")).expect("config");
        let expected =
            fingerprint(&managed_fragment("codex-cli", &config, Some(&current)).expect("fragment"));
        assert!(matches!(
            adapter.apply(
                &current,
                Some(&current),
                Some("replace-secret"),
                false,
                &expected
            ),
            Err(CliConfigError::AuthConfirmationRequired)
        ));
        assert_eq!(fs::read(&auth_path).expect("auth"), official_auth);
    }

    #[test]
    fn rollback_restores_exact_bytes_and_prior_absence() {
        let directory = TempDirectory::new("cli-config-rollback");
        let existing = directory.path().join("existing.json");
        let created = directory.path().join("created.json");
        fs::write(&existing, b"before").expect("fixture");
        let snapshots = vec![
            snapshot(&existing).expect("snapshot"),
            snapshot(&created).expect("snapshot"),
        ];
        fs::write(&existing, b"after").expect("changed");
        fs::write(&created, b"new").expect("created");

        assert!(rollback(&snapshots, &[existing.clone(), created.clone()]));
        assert_eq!(fs::read(existing).expect("restored"), b"before");
        assert!(!created.exists());
    }

    #[test]
    fn failed_atomic_replace_cleans_up_its_sibling_temporary_file() {
        let directory = TempDirectory::new("cli-config-atomic-failure");
        let target = directory.path().join("target-directory");
        fs::create_dir_all(&target).expect("target directory");
        assert!(atomic_replace(&target, b"content").is_err());
        let leftovers = fs::read_dir(directory.path())
            .expect("directory")
            .filter_map(Result::ok)
            .filter(|entry| entry.file_name().to_string_lossy().starts_with(".vanehub-"))
            .count();
        assert_eq!(leftovers, 0);
    }

    #[test]
    fn discovery_is_read_only_and_missing_files_return_no_candidates() {
        let directory = TempDirectory::new("cli-config-discovery-missing");
        let adapter = NativeCliGlobalConfigAdapter::with_home(directory.path().to_path_buf());

        for agent_id in ["claude-code", "opencode", "codex-cli"] {
            let discovery = adapter.discover_current(agent_id).expect("discovery");
            assert!(discovery.candidates.is_empty());
        }
        assert!(fs::read_dir(directory.path())
            .expect("temporary home")
            .next()
            .is_none());
    }

    #[test]
    fn discovery_returns_one_exclusive_candidate_without_mutating_claude_config() {
        let directory = TempDirectory::new("cli-config-discovery-claude");
        let adapter = NativeCliGlobalConfigAdapter::with_home(directory.path().to_path_buf());
        let path = adapter.primary_path("claude-code").expect("path");
        fs::create_dir_all(path.parent().expect("parent")).expect("directory");
        let fixture = br#"{"env":{"ANTHROPIC_BASE_URL":"https://proxy.example.com","ANTHROPIC_AUTH_TOKEN":"secret-not-for-dto","ANTHROPIC_MODEL":"claude-sonnet"}}"#;
        fs::write(&path, fixture).expect("fixture");

        let discovery = adapter.discover_current("claude-code").expect("discovery");
        assert_eq!(discovery.candidates.len(), 1);
        assert_eq!(discovery.candidates[0].candidate_key, "current");
        assert!(discovery.candidates[0].credential.is_some());
        assert_eq!(fs::read(path).expect("unchanged"), fixture);
    }

    #[test]
    fn codex_discovery_returns_the_selected_provider_as_one_candidate() {
        let directory = TempDirectory::new("cli-config-discovery-codex");
        let adapter = NativeCliGlobalConfigAdapter::with_home(directory.path().to_path_buf());
        let path = adapter.primary_path("codex-cli").expect("path");
        fs::create_dir_all(path.parent().expect("parent")).expect("directory");
        let fixture = r#"model = "gpt-5"
model_provider = "openrouter"
[model_providers.openrouter]
base_url = "https://openrouter.ai/api/v1"
wire_api = "responses"
experimental_bearer_token = "codex-secret"
"#;
        fs::write(&path, fixture).expect("fixture");

        let discovery = adapter.discover_current("codex-cli").expect("discovery");
        assert_eq!(discovery.candidates.len(), 1);
        assert_eq!(discovery.candidates[0].provider_name, "openrouter");
        assert_eq!(discovery.candidates[0].model, "gpt-5");
        assert!(discovery.candidates[0].credential.is_some());
        assert_eq!(fs::read_to_string(path).expect("unchanged"), fixture);
    }

    #[test]
    fn opencode_discovery_returns_every_compatible_provider() {
        let directory = TempDirectory::new("cli-config-discovery-opencode");
        let adapter = NativeCliGlobalConfigAdapter::with_home(directory.path().to_path_buf());
        let path = adapter.primary_path("opencode").expect("path");
        fs::create_dir_all(path.parent().expect("parent")).expect("directory");
        fs::write(
            &path,
            r#"{
              model: 'deepseek/deepseek-chat',
              provider: {
                deepseek: { name: 'DeepSeek', options: { baseURL: 'https://api.deepseek.com/v1', apiKey: 'deep-secret' }, models: { 'deepseek-chat': { name: 'DeepSeek Chat' } } },
                openrouter: { name: 'OpenRouter', options: { baseURL: 'https://openrouter.ai/api/v1', apiKey: 'router-secret' }, models: { 'openai/gpt-5': { name: 'GPT-5' } } }
              }
            }"#,
        )
        .expect("fixture");

        let discovery = adapter.discover_current("opencode").expect("discovery");
        assert_eq!(discovery.candidates.len(), 2);
        assert_eq!(
            discovery
                .candidates
                .iter()
                .filter(|candidate| candidate.is_default)
                .map(|candidate| candidate.candidate_key.as_str())
                .collect::<Vec<_>>(),
            vec!["deepseek"]
        );
        assert!(discovery
            .candidates
            .iter()
            .all(|candidate| candidate.credential.is_some()));
    }
}
