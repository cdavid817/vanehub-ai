use crate::contexts::agent_runtime::infrastructure::{ManagedMcpRelayPort, PreparedMcpRelay};
use crate::contexts::execution_observability::api::{CapturePolicy, ExecutionContext};
use crate::contexts::tooling::mcp::application::McpServerRepository;
use crate::contexts::tooling::mcp::domain::{ServerConfiguration, TransportType};
use crate::contexts::tooling::mcp::infrastructure::{
    write_configuration, RelayConfiguration, RelayObservation, RelayTarget,
    SqliteMcpServerRepository,
};
use crate::platform::database::NativeDatabase;
use serde_json::{json, Map, Value};
use std::fs;
use std::path::{Path, PathBuf};
use uuid::Uuid;

const RELAY_FLAG: &str = "--vanehub-mcp-relay";
const RELAY_TIMEOUT_MS: u64 = 30_000;

pub(crate) struct InvocationScopedMcpRelayAdapter {
    repository: SqliteMcpServerRepository,
    database_path: PathBuf,
    executable: PathBuf,
    relay_directory: PathBuf,
}

impl InvocationScopedMcpRelayAdapter {
    pub(crate) fn new(database: NativeDatabase) -> Self {
        Self {
            repository: SqliteMcpServerRepository::new(database.clone()),
            database_path: database.db_path,
            executable: std::env::current_exe().unwrap_or_else(|_| PathBuf::from("vanehub")),
            relay_directory: std::env::temp_dir().join("vanehub-mcp-relay"),
        }
    }

    fn prepare_servers(
        &self,
        project_path: Option<&str>,
        context: &ExecutionContext,
    ) -> Result<Vec<PreparedServer>, String> {
        let servers = self
            .repository
            .list_visible(project_path.unwrap_or_default())
            .map_err(|error| error.to_string())?;
        servers
            .into_iter()
            .filter(ServerConfiguration::is_active)
            .map(|server| self.prepare_server(&server, context))
            .collect()
    }

    fn prepare_server(
        &self,
        server: &ServerConfiguration,
        context: &ExecutionContext,
    ) -> Result<PreparedServer, String> {
        let target = match server.transport_type() {
            TransportType::Stdio => RelayTarget::Stdio {
                command: server.command().unwrap_or_default().to_string(),
                args: server.args().unwrap_or_default().to_vec(),
                env: server.env().cloned().unwrap_or_default(),
            },
            TransportType::Sse | TransportType::StreamableHttp => RelayTarget::Http {
                url: server.url().unwrap_or_default().to_string(),
                headers: server.headers().cloned().unwrap_or_default(),
            },
        };
        let configuration = RelayConfiguration {
            target,
            traceparent: context.traceparent(),
            observation: Some(RelayObservation {
                database_path: self.database_path.clone(),
                run_id: context.run_id.as_str().to_string(),
                trace_id: context.trace_id.as_str().to_string(),
                parent_span_id: context.span_id.as_str().to_string(),
                capture_policy: capture_policy(context.capture_policy).to_string(),
            }),
            timeout_ms: RELAY_TIMEOUT_MS,
        };
        let configuration_path = write_configuration(&self.relay_directory, &configuration)?;
        Ok(PreparedServer {
            name: server.name().as_str().to_string(),
            configuration_path,
        })
    }
}

impl ManagedMcpRelayPort for InvocationScopedMcpRelayAdapter {
    fn prepare(
        &self,
        agent_id: &str,
        project_path: Option<&str>,
        context: &ExecutionContext,
    ) -> Result<PreparedMcpRelay, String> {
        if !matches!(agent_id, "claude-code" | "codex-cli") {
            return Ok(PreparedMcpRelay {
                invocation_args: Vec::new(),
                cleanup_paths: Vec::new(),
            });
        }
        let servers = self.prepare_servers(project_path, context)?;
        if servers.is_empty() {
            return Ok(PreparedMcpRelay {
                invocation_args: Vec::new(),
                cleanup_paths: Vec::new(),
            });
        }
        let mut cleanup_paths = servers
            .iter()
            .map(|server| server.configuration_path.clone())
            .collect::<Vec<_>>();
        let invocation_args = match provider_invocation_args(
            agent_id,
            &self.executable,
            &servers,
            &self.relay_directory,
        ) {
            Ok((args, provider_path)) => {
                if let Some(path) = provider_path {
                    cleanup_paths.push(path);
                }
                args
            }
            Err(error) => {
                cleanup(&cleanup_paths);
                return Err(error);
            }
        };
        Ok(PreparedMcpRelay {
            invocation_args,
            cleanup_paths,
        })
    }
}

struct PreparedServer {
    name: String,
    configuration_path: PathBuf,
}

fn provider_invocation_args(
    agent_id: &str,
    executable: &Path,
    servers: &[PreparedServer],
    directory: &Path,
) -> Result<(Vec<String>, Option<PathBuf>), String> {
    match agent_id {
        "claude-code" => {
            let path = write_claude_configuration(executable, servers, directory)?;
            Ok((
                vec![
                    "--mcp-config".to_string(),
                    path.to_string_lossy().to_string(),
                ],
                Some(path),
            ))
        }
        "codex-cli" => Ok((codex_overrides(executable, servers)?, None)),
        _ => Ok((Vec::new(), None)),
    }
}

fn write_claude_configuration(
    executable: &Path,
    servers: &[PreparedServer],
    directory: &Path,
) -> Result<PathBuf, String> {
    fs::create_dir_all(directory).map_err(|error| error.to_string())?;
    let mut entries = Map::new();
    for server in servers {
        entries.insert(
            server.name.clone(),
            json!({
                "command": executable.to_string_lossy(),
                "args": [RELAY_FLAG, server.configuration_path.to_string_lossy()]
            }),
        );
    }
    let path = directory.join(format!("claude-{}.json", Uuid::new_v4()));
    let bytes = serde_json::to_vec(&json!({ "mcpServers": Value::Object(entries) }))
        .map_err(|error| error.to_string())?;
    fs::write(&path, bytes).map_err(|error| error.to_string())?;
    Ok(path)
}

fn codex_overrides(executable: &Path, servers: &[PreparedServer]) -> Result<Vec<String>, String> {
    let executable = serde_json::to_string(&executable.to_string_lossy().to_string())
        .map_err(|error| error.to_string())?;
    let mut args = Vec::with_capacity(servers.len() * 4);
    for server in servers {
        let name = serde_json::to_string(&server.name).map_err(|error| error.to_string())?;
        let relay_args = serde_json::to_string(&vec![
            RELAY_FLAG.to_string(),
            server.configuration_path.to_string_lossy().to_string(),
        ])
        .map_err(|error| error.to_string())?;
        args.extend([
            "-c".to_string(),
            format!("mcp_servers.{name}.command={executable}"),
            "-c".to_string(),
            format!("mcp_servers.{name}.args={relay_args}"),
        ]);
    }
    Ok(args)
}

fn capture_policy(policy: CapturePolicy) -> &'static str {
    match policy {
        CapturePolicy::MetadataOnly => "metadata_only",
        CapturePolicy::RedactedContent => "redacted_content",
    }
}

fn cleanup(paths: &[PathBuf]) {
    for path in paths {
        let _ = fs::remove_file(path);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn server(name: &str) -> PreparedServer {
        PreparedServer {
            name: name.to_string(),
            configuration_path: PathBuf::from(format!("{name}.json")),
        }
    }

    #[test]
    fn codex_uses_invocation_scoped_configuration_overrides() {
        let args =
            codex_overrides(Path::new("vanehub.exe"), &[server("local-tools")]).expect("args");
        assert_eq!(args[0], "-c");
        assert!(args[1].starts_with("mcp_servers.\"local-tools\".command="));
        assert_eq!(args[2], "-c");
        assert!(args[3].contains(RELAY_FLAG));
    }

    #[test]
    fn unsupported_providers_receive_no_relay_arguments() {
        let result = provider_invocation_args(
            "gemini-cli",
            Path::new("vanehub"),
            &[server("local-tools")],
            Path::new("."),
        )
        .expect("fallback");
        assert!(result.0.is_empty());
        assert!(result.1.is_none());
    }

    #[test]
    fn relay_arguments_preserve_literal_paths_without_shell_parsing() {
        let args = codex_overrides(
            Path::new("C:\\Program Files\\VaneHub\\vanehub.exe"),
            &[PreparedServer {
                name: "local-tools".to_string(),
                configuration_path: PathBuf::from("C:\\relay files\\relay.json"),
            }],
        )
        .expect("args");
        assert!(args[1].contains("Program Files"));
        assert!(args[3].contains("relay files"));
    }
}
