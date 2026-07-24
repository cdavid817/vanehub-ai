DELETE FROM schema_migrations WHERE version = 28;

DROP TRIGGER terminal_output_chunks_fts_update;
DROP TRIGGER terminal_output_chunks_fts_delete;
DROP TRIGGER terminal_output_chunks_fts_insert;
DROP TABLE terminal_output_fts;

DROP INDEX idx_terminal_output_chunks_run_sequence;
DROP INDEX idx_terminal_output_chunks_connection_time;
DROP INDEX idx_terminal_output_chunks_session_time;
DROP INDEX idx_terminal_command_runs_session_started;
DROP INDEX idx_terminal_command_templates_scope;
DROP INDEX idx_sessions_remote_ssh_connection;

DROP TABLE terminal_output_chunks;
DROP TABLE terminal_command_runs;
DROP TABLE terminal_command_templates;
DROP TABLE terminal_capture_settings;
DROP TABLE ssh_host_trust;

ALTER TABLE sessions DROP COLUMN remote_ssh_connection_revision;
ALTER TABLE sessions DROP COLUMN remote_ssh_connection_id;
ALTER TABLE ssh_connections DROP COLUMN revision;
