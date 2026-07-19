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
