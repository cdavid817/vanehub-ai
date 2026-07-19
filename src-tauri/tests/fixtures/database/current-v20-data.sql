INSERT INTO settings (key, value, created_at, updated_at)
VALUES ('application_language', 'en', '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z');

INSERT INTO sessions (
    id, title, agent_id, interaction_mode, lifecycle_state, pinned, archived, created_at, updated_at
) VALUES (
    'fixture-session', 'Current fixture', 'codex-cli', 'cli', 'idle', 0, 0,
    '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z'
);

INSERT INTO messages (
    id, session_id, role, status, content, created_at, updated_at
) VALUES (
    'fixture-message', 'fixture-session', 'assistant', 'completed', 'Persisted fixture',
    '2026-01-01T00:00:01Z', '2026-01-01T00:00:01Z'
);

INSERT INTO known_projects (path, display_name, is_git, last_opened_at)
VALUES ('D:\\code\\fixture', 'fixture', 1, '2026-01-01T00:00:00Z');

INSERT INTO mcp_servers (
    name, transport_type, command, active, scope, created_at, updated_at
) VALUES (
    'fixture-mcp', 'stdio', 'fixture-mcp', 1, 'user',
    '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z'
);
