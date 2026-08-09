use rusqlite::{params, Connection, OptionalExtension};

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
    &'static str,
);

const AGENTS: [SeedAgent; 6] = [
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
        "builtin",
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
        "builtin",
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
        "builtin",
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
        "builtin",
    ),
    (
        "antigravity-cli",
        "Antigravity CLI",
        "Google",
        "cli",
        Some("agy"),
        None,
        Some("agy"),
        None,
        &["cli"],
        &["coding", "cli", "agent"],
        "builtin",
    ),
    (
        "onepiece",
        "OnePiece",
        "VaneHub",
        "api",
        None,
        None,
        None,
        None,
        &["api"],
        &["coding", "api", "agent", "native"],
        "builtin",
    ),
];

pub(crate) fn apply_agent_origin_schema(
    conn: &Connection,
) -> Result<(), crate::platform::database::DatabaseError> {
    conn.execute_batch(
        "ALTER TABLE agents ADD COLUMN agent_origin TEXT NOT NULL DEFAULT 'user' \
         CHECK (agent_origin IN ('builtin', 'user')); \
         UPDATE agents SET agent_origin = 'builtin' \
         WHERE id IN ('claude-code', 'opencode', 'codex-cli', 'gemini-cli');",
    )?;
    Ok(())
}

pub(crate) fn apply_expert_role_schema(
    conn: &Connection,
) -> Result<(), crate::platform::database::DatabaseError> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS expert_roles (
            id TEXT PRIMARY KEY,
            display_name TEXT NOT NULL,
            avatar TEXT NOT NULL,
            color TEXT NOT NULL,
            responsibility TEXT NOT NULL,
            instruction TEXT NOT NULL,
            skill_ids TEXT NOT NULL DEFAULT '[]',
            peer_reviewer INTEGER NOT NULL DEFAULT 0,
            require_different_family INTEGER NOT NULL DEFAULT 0,
            preferred_providers TEXT NOT NULL DEFAULT '[]',
            origin TEXT NOT NULL DEFAULT 'user' CHECK (origin IN ('builtin', 'user')),
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );

        CREATE INDEX IF NOT EXISTS idx_expert_roles_updated
            ON expert_roles(updated_at DESC);
        "#,
    )?;
    Ok(())
}

pub(crate) fn apply_onepiece_provider_profiles_schema(
    conn: &Connection,
) -> Result<(), crate::platform::database::DatabaseError> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS onepiece_provider_profiles (
            id TEXT PRIMARY KEY,
            agent_id TEXT NOT NULL DEFAULT 'onepiece' CHECK (agent_id = 'onepiece'),
            name TEXT NOT NULL,
            provider TEXT NOT NULL,
            model_id TEXT NOT NULL,
            interface_format TEXT NOT NULL CHECK (interface_format IN ('anthropic', 'openai-compatible')),
            base_url TEXT,
            active INTEGER NOT NULL DEFAULT 0 CHECK (active IN (0, 1)),
            created_at TEXT NOT NULL DEFAULT (strftime('%s', 'now')),
            updated_at TEXT NOT NULL DEFAULT (strftime('%s', 'now')),
            FOREIGN KEY (agent_id) REFERENCES agents(id) ON DELETE CASCADE
        );
        CREATE UNIQUE INDEX IF NOT EXISTS idx_onepiece_provider_profiles_active
            ON onepiece_provider_profiles(agent_id) WHERE active = 1;

        INSERT OR IGNORE INTO onepiece_provider_profiles
            (id, agent_id, name, provider, model_id, interface_format, base_url, active)
        SELECT 'legacy-default', 'onepiece', provider, provider, model_id, interface_format, base_url, 1
        FROM agents
        WHERE id = 'onepiece'
          AND launch_kind = 'api'
          AND model_id IS NOT NULL AND trim(model_id) <> ''
          AND interface_format IN ('anthropic', 'openai-compatible')
          AND (interface_format <> 'openai-compatible' OR (base_url IS NOT NULL AND trim(base_url) <> ''));
        "#,
    )?;
    Ok(())
}

pub(crate) fn apply_onepiece_provider_catalog_schema(
    conn: &Connection,
) -> Result<(), crate::platform::database::DatabaseError> {
    conn.execute_batch(
        r#"
        ALTER TABLE onepiece_provider_profiles ADD COLUMN source_preset_id TEXT;
        ALTER TABLE onepiece_provider_profiles ADD COLUMN source_preset_version INTEGER;

        UPDATE onepiece_provider_profiles
        SET source_preset_id = CASE
                WHEN provider = 'Anthropic' AND interface_format = 'anthropic' AND base_url IS NULL THEN 'anthropic'
                WHEN provider = 'OpenAI' AND interface_format = 'openai-compatible' AND base_url = 'https://api.openai.com/v1' THEN 'openai'
                WHEN provider = 'OpenRouter' AND interface_format = 'openai-compatible' AND base_url = 'https://openrouter.ai/api/v1' THEN 'openrouter'
                WHEN provider = 'DeepSeek' AND interface_format = 'openai-compatible' AND base_url = 'https://api.deepseek.com/v1' THEN 'deepseek'
                WHEN provider = 'Zhipu GLM' AND interface_format = 'openai-compatible' AND base_url = 'https://open.bigmodel.cn/api/coding/paas/v4' THEN 'zhipu-glm'
                WHEN provider = 'Kimi / Moonshot' AND interface_format = 'openai-compatible' AND base_url = 'https://api.moonshot.cn/v1' THEN 'kimi'
                WHEN provider = 'SiliconFlow' AND interface_format = 'openai-compatible' AND base_url = 'https://api.siliconflow.cn/v1' THEN 'siliconflow'
                WHEN provider = 'Alibaba Bailian' AND interface_format = 'openai-compatible' AND base_url = 'https://dashscope.aliyuncs.com/compatible-mode/v1' THEN 'bailian'
                WHEN provider = 'Volcengine Ark' AND interface_format = 'openai-compatible' AND base_url = 'https://ark.cn-beijing.volces.com/api/v3' THEN 'volcengine-ark'
                ELSE NULL
            END;

        UPDATE onepiece_provider_profiles
        SET source_preset_version = CASE
                WHEN source_preset_id IS NOT NULL THEN 1
                ELSE NULL
            END;
        "#,
    )?;
    Ok(())
}

pub(crate) fn apply_onepiece_provider_endpoint_schema(
    conn: &Connection,
) -> Result<(), crate::platform::database::DatabaseError> {
    conn.execute_batch(
        r#"
        ALTER TABLE onepiece_provider_profiles ADD COLUMN source_provider_id TEXT;
        ALTER TABLE onepiece_provider_profiles ADD COLUMN source_endpoint_type TEXT
            CHECK (source_endpoint_type IS NULL OR source_endpoint_type IN ('anthropic-messages', 'openai-chat-completions', 'openai-responses'));

        UPDATE onepiece_provider_profiles
        SET source_provider_id = CASE
                WHEN source_preset_id IS NULL THEN NULL
                WHEN instr(source_preset_id, '::') > 0
                    THEN substr(source_preset_id, 1, instr(source_preset_id, '::') - 1)
                ELSE source_preset_id
            END;

        UPDATE onepiece_provider_profiles
        SET source_endpoint_type = CASE
                WHEN source_preset_id IS NULL THEN NULL
                WHEN instr(source_preset_id, '::') > 0
                    THEN substr(source_preset_id, instr(source_preset_id, '::') + 2)
                WHEN source_provider_id = 'anthropic' THEN 'anthropic-messages'
                ELSE 'openai-chat-completions'
            END;
        "#,
    )?;
    Ok(())
}

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
    let onepiece_kind = connection
        .query_row(
            "SELECT launch_kind FROM agents WHERE id = 'onepiece'",
            [],
            |row| row.get::<_, String>(0),
        )
        .optional()?;
    if onepiece_kind.as_deref().is_some_and(|kind| kind != "api") {
        return Err(crate::platform::database::DatabaseError::Storage(
            "cannot seed OnePiece: agent id 'onepiece' is already used by a non-API agent"
                .to_string(),
        ));
    }

    for (
        id,
        display_name,
        provider,
        kind,
        command,
        url,
        executable,
        sdk_dependency,
        modes,
        tags,
        origin,
    ) in AGENTS
    {
        connection.execute(
            "INSERT OR IGNORE INTO agents (id, display_name, provider, launch_kind, launch_command, launch_url, executable_name, managed_sdk_dependency_id, agent_origin)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                id,
                display_name,
                provider,
                kind,
                command,
                url,
                executable,
                sdk_dependency,
                origin,
            ],
        )?;
        if id == "onepiece" {
            connection.execute(
                "UPDATE agents SET agent_origin = 'builtin' WHERE id = 'onepiece' AND launch_kind = 'api'",
                [],
            )?;
        }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::database::migrate;

    fn migrated_connection() -> Connection {
        let connection = Connection::open_in_memory().expect("in-memory database");
        connection
            .pragma_update(None, "foreign_keys", "ON")
            .expect("foreign keys");
        migrate(&connection).expect("migrations");
        connection
    }

    #[test]
    fn seeds_onepiece_and_existing_cli_agents_idempotently() {
        let connection = migrated_connection();

        seed_registry(&connection).expect("first seed");
        seed_registry(&connection).expect("second seed");

        let seeded: (String, String, String, String, Option<String>, i64) = connection
            .query_row(
                "SELECT display_name, provider, launch_kind, agent_origin, model_id, auto_approve_tools FROM agents WHERE id = 'onepiece'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?, row.get(5)?)),
            )
            .expect("OnePiece seed");
        assert_eq!(
            seeded,
            (
                "OnePiece".to_string(),
                "VaneHub".to_string(),
                "api".to_string(),
                "builtin".to_string(),
                None,
                0,
            )
        );
        let agent_count: i64 = connection
            .query_row("SELECT COUNT(*) FROM agents", [], |row| row.get(0))
            .expect("agent count");
        assert_eq!(agent_count, 6);
        for cli_id in [
            "claude-code",
            "opencode",
            "codex-cli",
            "gemini-cli",
            "antigravity-cli",
        ] {
            let origin: String = connection
                .query_row(
                    "SELECT agent_origin FROM agents WHERE id = ?1",
                    [cli_id],
                    |row| row.get(0),
                )
                .expect("CLI origin");
            assert_eq!(origin, "builtin");
        }
    }

    /// Seeding runs on every bootstrap, not only on a fresh database, so a database created before
    /// Antigravity existed gains the row without a dedicated migration — and re-seeding must not
    /// disturb it.
    #[test]
    fn back_fills_antigravity_into_a_database_seeded_before_it_existed() {
        let connection = migrated_connection();
        connection
            .execute("DELETE FROM agents WHERE id = 'antigravity-cli'", [])
            .expect("simulate pre-Antigravity database");

        seed_registry(&connection).expect("back-fill seed");
        seed_registry(&connection).expect("repeat seed");

        let seeded: (String, String, String, Option<String>, Option<String>, String) = connection
            .query_row(
                "SELECT display_name, provider, launch_kind, launch_command, executable_name, agent_origin FROM agents WHERE id = 'antigravity-cli'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?, row.get(5)?)),
            )
            .expect("Antigravity seed");
        assert_eq!(
            seeded,
            (
                "Antigravity CLI".to_string(),
                "Google".to_string(),
                "cli".to_string(),
                Some("agy".to_string()),
                Some("agy".to_string()),
                "builtin".to_string(),
            )
        );

        let modes: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM agent_modes WHERE agent_id = 'antigravity-cli'",
                [],
                |row| row.get(0),
            )
            .expect("mode count");
        assert_eq!(modes, 1);
    }

    #[test]
    fn adopts_an_existing_api_onepiece_without_replacing_configuration() {
        let connection = migrated_connection();
        connection
            .execute(
                "INSERT INTO agents (id, display_name, provider, launch_kind, model_id, interface_format, base_url, agent_origin) VALUES ('onepiece', 'My OnePiece', 'Existing Provider', 'api', 'existing-model', 'openai-compatible', 'https://example.test/v1', 'user')",
                [],
            )
            .expect("existing API fixture");

        seed_registry(&connection).expect("adopt API OnePiece");

        let adopted: (String, String, String, String, String, String) = connection
            .query_row(
                "SELECT display_name, provider, model_id, interface_format, base_url, agent_origin FROM agents WHERE id = 'onepiece'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?, row.get(5)?)),
            )
            .expect("adopted row");
        assert_eq!(
            adopted,
            (
                "My OnePiece".to_string(),
                "Existing Provider".to_string(),
                "existing-model".to_string(),
                "openai-compatible".to_string(),
                "https://example.test/v1".to_string(),
                "builtin".to_string(),
            )
        );
        let api_mode: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM agent_modes WHERE agent_id = 'onepiece' AND mode = 'api'",
                [],
                |row| row.get(0),
            )
            .expect("API mode");
        assert_eq!(api_mode, 1);
    }

    #[test]
    fn rejects_a_non_api_onepiece_collision_without_modifying_it() {
        let connection = migrated_connection();
        connection
            .execute(
                "INSERT INTO agents (id, display_name, provider, launch_kind, agent_origin) VALUES ('onepiece', 'Collision', 'Local', 'cli', 'user')",
                [],
            )
            .expect("collision fixture");

        let error = seed_registry(&connection).expect_err("collision must fail");
        assert!(error.to_string().contains("non-API agent"));
        let row: (String, String) = connection
            .query_row(
                "SELECT display_name, agent_origin FROM agents WHERE id = 'onepiece'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .expect("unchanged collision");
        assert_eq!(row, ("Collision".to_string(), "user".to_string()));
    }
}
