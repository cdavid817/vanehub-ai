PRAGMA foreign_keys = ON;

CREATE TABLE schema_migrations (
    version INTEGER PRIMARY KEY,
    name TEXT NOT NULL,
    applied_at TEXT NOT NULL DEFAULT (strftime('%s', 'now'))
);

INSERT INTO schema_migrations (version, name) VALUES (1, 'initial-schema');

CREATE TABLE agents (
    id TEXT PRIMARY KEY,
    display_name TEXT NOT NULL,
    provider TEXT NOT NULL,
    launch_kind TEXT NOT NULL,
    launch_command TEXT,
    launch_url TEXT,
    executable_name TEXT
);

CREATE TABLE agent_modes (
    agent_id TEXT NOT NULL,
    mode TEXT NOT NULL,
    PRIMARY KEY (agent_id, mode),
    FOREIGN KEY (agent_id) REFERENCES agents(id) ON DELETE CASCADE
);

CREATE TABLE agent_capability_tags (
    agent_id TEXT NOT NULL,
    tag TEXT NOT NULL,
    PRIMARY KEY (agent_id, tag),
    FOREIGN KEY (agent_id) REFERENCES agents(id) ON DELETE CASCADE
);

CREATE TABLE workflow_state (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    active_agent_id TEXT,
    active_interaction_mode TEXT,
    lifecycle_state TEXT NOT NULL,
    intent TEXT NOT NULL
);

CREATE TABLE session_details (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    adapter TEXT NOT NULL,
    message TEXT NOT NULL
);

CREATE TABLE mcp_servers (
    name TEXT PRIMARY KEY,
    transport_type TEXT NOT NULL DEFAULT 'stdio',
    command TEXT,
    args TEXT,
    env TEXT,
    url TEXT,
    headers TEXT,
    description TEXT,
    active INTEGER NOT NULL DEFAULT 1,
    scope TEXT NOT NULL DEFAULT 'user',
    project_path TEXT,
    last_connection_status TEXT,
    last_connected TEXT,
    last_error TEXT,
    last_tools TEXT,
    last_test_duration_ms INTEGER,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

INSERT INTO agents (
    id, display_name, provider, launch_kind, launch_command, launch_url, executable_name
) VALUES (
    'legacy-agent', 'Legacy Agent', 'legacy', 'cli', 'legacy-agent', NULL, 'legacy-agent'
);

INSERT INTO agent_modes (agent_id, mode) VALUES ('legacy-agent', 'cli');
INSERT INTO workflow_state (id, active_agent_id, active_interaction_mode, lifecycle_state, intent)
VALUES (1, 'legacy-agent', 'cli', 'idle', 'Legacy fixture');
INSERT INTO session_details (id, adapter, message) VALUES (1, 'legacy', 'Legacy details');
INSERT INTO mcp_servers (
    name, transport_type, command, active, scope, created_at, updated_at
) VALUES (
    'legacy-mcp', 'stdio', 'legacy-mcp', 1, 'user', '2025-01-01T00:00:00Z', '2025-01-01T00:00:00Z'
);
