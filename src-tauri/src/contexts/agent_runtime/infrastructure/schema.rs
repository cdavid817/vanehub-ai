use rusqlite::{params, Connection};

type SeedAgent = (
    &'static str,
    &'static str,
    &'static str,
    &'static str,
    Option<&'static str>,
    Option<&'static str>,
    Option<&'static str>,
    Option<&'static str>,
    &'static [&'static str],
    &'static [&'static str],
);

const AGENTS: [SeedAgent; 4] = [
    (
        "claude-code",
        "Claude Code",
        "Anthropic",
        "cli",
        Some("claude"),
        None,
        Some("claude"),
        Some("claude-sdk"),
        &["cli", "native-desktop"],
        &["coding", "cli", "agent"],
    ),
    (
        "opencode",
        "OpenCode",
        "OpenCode",
        "cli",
        Some("opencode"),
        None,
        Some("opencode"),
        None,
        &["cli"],
        &["coding", "cli", "open-source"],
    ),
    (
        "codex-cli",
        "Codex CLI",
        "OpenAI",
        "cli",
        Some("codex"),
        None,
        Some("codex"),
        Some("codex-sdk"),
        &["cli", "native-desktop"],
        &["coding", "cli", "agent"],
    ),
    (
        "gemini-cli",
        "Gemini CLI",
        "Google",
        "cli",
        Some("gemini"),
        None,
        Some("gemini"),
        None,
        &["cli", "browser"],
        &["coding", "cli", "browser"],
    ),
];

pub(crate) fn apply_api_agent_schema(
    conn: &Connection,
) -> Result<(), crate::platform::database::DatabaseError> {
    conn.execute_batch("ALTER TABLE agents ADD COLUMN model_id TEXT;")?;
    Ok(())
}

/// Adds the wire-protocol fields needed to support OpenAI Chat Completions-compatible
/// endpoints alongside the existing Anthropic-only path. Existing `launch_kind = 'api'`
/// rows (registered before this migration) are backfilled to `interface_format = 'anthropic'`
/// so they keep behaving exactly as before.
pub(crate) fn apply_openai_compatible_schema(
    conn: &Connection,
) -> Result<(), crate::platform::database::DatabaseError> {
    conn.execute_batch(
        "ALTER TABLE agents ADD COLUMN interface_format TEXT;
         ALTER TABLE agents ADD COLUMN base_url TEXT;
         UPDATE agents SET interface_format = 'anthropic' WHERE launch_kind = 'api';",
    )?;
    Ok(())
}

/// Adds the persistent, per-agent tool-approval trust flag (`add-agent-tool-trust`) — off by
/// default for every existing and newly registered agent, mirrors CLI agents' own existing
/// persisted-CLI-Profile lever for the same underlying concern.
pub(crate) fn apply_agent_tool_trust_schema(
    conn: &Connection,
) -> Result<(), crate::platform::database::DatabaseError> {
    conn.execute_batch(
        "ALTER TABLE agents ADD COLUMN auto_approve_tools INTEGER NOT NULL DEFAULT 0;",
    )?;
    Ok(())
}

pub(crate) fn seed_registry(
    connection: &Connection,
) -> Result<(), crate::platform::database::DatabaseError> {
    for (id, display_name, provider, kind, command, url, executable, sdk_dependency, modes, tags) in
        AGENTS
    {
        connection.execute(
            "INSERT OR IGNORE INTO agents (id, display_name, provider, launch_kind, launch_command, launch_url, executable_name, managed_sdk_dependency_id)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                id,
                display_name,
                provider,
                kind,
                command,
                url,
                executable,
                sdk_dependency
            ],
        )?;
        connection.execute(
            "UPDATE agents SET managed_sdk_dependency_id = ?1 WHERE id = ?2 AND managed_sdk_dependency_id IS NULL",
            params![sdk_dependency, id],
        )?;
        for mode in modes {
            connection.execute(
                "INSERT OR IGNORE INTO agent_modes (agent_id, mode) VALUES (?1, ?2)",
                params![id, mode],
            )?;
        }
        for tag in tags {
            connection.execute(
                "INSERT OR IGNORE INTO agent_capability_tags (agent_id, tag) VALUES (?1, ?2)",
                params![id, tag],
            )?;
        }
    }
    Ok(())
}
