//! Persistent execution bindings: which provider, which program, which transport, and which
//! external session a VaneHub session (or seat) is bound to.
//!
//! Additive to the session tables: a session created before this change has no row here and
//! keeps routing through its original path. A row is what lets resume be exact -- the stored
//! external id, checked against the same executable fingerprint and adapter revision -- instead
//! of a guess at "the most recent session".

use crate::contexts::agent_runtime::domain::ProviderTransport;
use crate::platform::database::{DatabaseError, NativeDatabase};
use rusqlite::{params, Connection, OptionalExtension};

pub(crate) fn apply_schema(conn: &Connection) -> Result<(), DatabaseError> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS cli_execution_bindings (
            binding_key TEXT PRIMARY KEY,
            session_id TEXT NOT NULL,
            seat_id TEXT,
            provider_id TEXT NOT NULL,
            transport TEXT NOT NULL,
            executable_path TEXT NOT NULL,
            installation_fingerprint TEXT NOT NULL,
            distribution TEXT,
            adapter_revision TEXT NOT NULL,
            external_session_id TEXT,
            workspace TEXT NOT NULL,
            account_profile TEXT,
            protocol_version INTEGER,
            peer_load_session INTEGER NOT NULL DEFAULT 0,
            last_handshake TEXT,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_cli_execution_bindings_session
            ON cli_execution_bindings(session_id);
        "#,
    )?;
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ExecutionBindingRecord {
    pub(crate) binding_key: String,
    pub(crate) session_id: String,
    pub(crate) seat_id: Option<String>,
    pub(crate) provider_id: String,
    pub(crate) transport: String,
    pub(crate) executable_path: String,
    pub(crate) installation_fingerprint: String,
    pub(crate) distribution: Option<String>,
    pub(crate) adapter_revision: String,
    pub(crate) external_session_id: Option<String>,
    pub(crate) workspace: String,
    pub(crate) account_profile: Option<String>,
    pub(crate) protocol_version: Option<i64>,
    pub(crate) peer_load_session: bool,
    /// Redacted handshake summary as JSON text.
    pub(crate) last_handshake: Option<String>,
}

/// Why a stored binding cannot be resumed on the current launch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum BindingCompatibility {
    Compatible,
    Incompatible { reason_code: &'static str },
}

impl ExecutionBindingRecord {
    /// Whether this record may be resumed by a launch of `executable_path` under
    /// `adapter_revision` in `workspace`. Every mismatch is named; none is papered over.
    pub(crate) fn compatibility(
        &self,
        provider_id: &str,
        transport: &str,
        installation_fingerprint: &str,
        adapter_revision: &str,
        workspace: &str,
        account_profile: Option<&str>,
    ) -> BindingCompatibility {
        // A transport is compared as a parsed value: an unknown stored string is never equal to
        // a real one, so a corrupted row cannot resume anything.
        let stored_transport = ProviderTransport::parse(&self.transport);
        let reason = if self.provider_id != provider_id {
            Some("binding-provider-changed")
        } else if stored_transport.is_none()
            || stored_transport != ProviderTransport::parse(transport)
        {
            Some("binding-transport-changed")
        } else if self.installation_fingerprint != installation_fingerprint {
            Some("binding-installation-changed")
        } else if self.adapter_revision != adapter_revision {
            Some("binding-adapter-revision-changed")
        } else if self.workspace != workspace {
            Some("binding-workspace-changed")
        } else if self.account_profile.as_deref() != account_profile {
            Some("binding-account-profile-changed")
        } else if !self.peer_load_session {
            Some("binding-peer-cannot-load")
        } else if self
            .external_session_id
            .as_deref()
            .is_none_or(str::is_empty)
        {
            Some("binding-external-session-missing")
        } else {
            None
        };
        match reason {
            Some(reason_code) => BindingCompatibility::Incompatible { reason_code },
            None => BindingCompatibility::Compatible,
        }
    }
}

#[derive(Clone)]
pub(crate) struct SqliteExecutionBindingRepository {
    database: NativeDatabase,
}

impl SqliteExecutionBindingRepository {
    pub(crate) fn new(database: NativeDatabase) -> Self {
        Self { database }
    }

    pub(crate) fn find(&self, binding_key: &str) -> Result<Option<ExecutionBindingRecord>, String> {
        let connection = self
            .database
            .connection()
            .map_err(|error| error.to_string())?;
        connection
            .query_row(
                "SELECT binding_key, session_id, seat_id, provider_id, transport, executable_path,
                        installation_fingerprint, distribution, adapter_revision,
                        external_session_id, workspace, account_profile, protocol_version,
                        peer_load_session, last_handshake
                 FROM cli_execution_bindings WHERE binding_key = ?1",
                params![binding_key],
                |row| {
                    Ok(ExecutionBindingRecord {
                        binding_key: row.get(0)?,
                        session_id: row.get(1)?,
                        seat_id: row.get(2)?,
                        provider_id: row.get(3)?,
                        transport: row.get(4)?,
                        executable_path: row.get(5)?,
                        installation_fingerprint: row.get(6)?,
                        distribution: row.get(7)?,
                        adapter_revision: row.get(8)?,
                        external_session_id: row.get(9)?,
                        workspace: row.get(10)?,
                        account_profile: row.get(11)?,
                        protocol_version: row.get(12)?,
                        peer_load_session: row.get::<_, i64>(13)? != 0,
                        last_handshake: row.get(14)?,
                    })
                },
            )
            .optional()
            .map_err(|error| error.to_string())
    }

    pub(crate) fn upsert(&self, record: &ExecutionBindingRecord, now: &str) -> Result<(), String> {
        let connection = self
            .database
            .connection()
            .map_err(|error| error.to_string())?;
        connection
            .execute(
                "INSERT INTO cli_execution_bindings (
                    binding_key, session_id, seat_id, provider_id, transport, executable_path,
                    installation_fingerprint, distribution, adapter_revision, external_session_id,
                    workspace, account_profile, protocol_version, peer_load_session,
                    last_handshake, created_at, updated_at
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?16)
                 ON CONFLICT(binding_key) DO UPDATE SET
                    session_id = excluded.session_id,
                    seat_id = excluded.seat_id,
                    provider_id = excluded.provider_id,
                    transport = excluded.transport,
                    executable_path = excluded.executable_path,
                    installation_fingerprint = excluded.installation_fingerprint,
                    distribution = excluded.distribution,
                    adapter_revision = excluded.adapter_revision,
                    external_session_id = excluded.external_session_id,
                    workspace = excluded.workspace,
                    account_profile = excluded.account_profile,
                    protocol_version = excluded.protocol_version,
                    peer_load_session = excluded.peer_load_session,
                    last_handshake = excluded.last_handshake,
                    updated_at = excluded.updated_at",
                params![
                    record.binding_key,
                    record.session_id,
                    record.seat_id,
                    record.provider_id,
                    record.transport,
                    record.executable_path,
                    record.installation_fingerprint,
                    record.distribution,
                    record.adapter_revision,
                    record.external_session_id,
                    record.workspace,
                    record.account_profile,
                    record.protocol_version,
                    i64::from(record.peer_load_session),
                    record.last_handshake,
                    now,
                ],
            )
            .map(|_| ())
            .map_err(|error| error.to_string())
    }

    /// Removes the bindings of one session. Called from session deletion; it never touches the
    /// CLI's own history or credential store.
    pub(crate) fn delete_session(&self, session_id: &str) -> Result<usize, String> {
        let connection = self
            .database
            .connection()
            .map_err(|error| error.to_string())?;
        connection
            .execute(
                "DELETE FROM cli_execution_bindings WHERE session_id = ?1",
                params![session_id],
            )
            .map_err(|error| error.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::TempDirectory;

    fn record() -> ExecutionBindingRecord {
        ExecutionBindingRecord {
            binding_key: "session-1/seat-1".to_string(),
            session_id: "session-1".to_string(),
            seat_id: Some("seat-1".to_string()),
            provider_id: "qwen-code".to_string(),
            transport: "acp-stdio".to_string(),
            executable_path: "/usr/local/bin/qwen".to_string(),
            installation_fingerprint: "sha256:abc".to_string(),
            distribution: Some("npm".to_string()),
            adapter_revision: "acp-v1".to_string(),
            external_session_id: Some("sess_123".to_string()),
            workspace: "/work/project".to_string(),
            account_profile: None,
            protocol_version: Some(1),
            peer_load_session: true,
            last_handshake: Some("{\"loadSession\":true}".to_string()),
        }
    }

    #[test]
    fn bindings_round_trip_and_delete_by_session() {
        let directory = TempDirectory::new("acp-binding-repo");
        let database = NativeDatabase::new(directory.path().to_path_buf()).expect("database");
        let repository = SqliteExecutionBindingRepository::new(database);
        assert_eq!(repository.find("session-1/seat-1").expect("query"), None);
        repository
            .upsert(&record(), "2026-09-06T00:00:00Z")
            .expect("insert");
        let mut updated = record();
        updated.external_session_id = Some("sess_456".to_string());
        repository
            .upsert(&updated, "2026-09-06T00:01:00Z")
            .expect("update");
        let stored = repository
            .find("session-1/seat-1")
            .expect("query")
            .expect("row");
        assert_eq!(stored, updated);
        assert_eq!(repository.delete_session("session-1").expect("delete"), 1);
        assert_eq!(repository.find("session-1/seat-1").expect("query"), None);
    }

    #[test]
    fn every_identity_change_refuses_resume_with_its_own_reason() {
        let stored = record();
        let compatible = stored.compatibility(
            "qwen-code",
            "acp-stdio",
            "sha256:abc",
            "acp-v1",
            "/work/project",
            None,
        );
        assert_eq!(compatible, BindingCompatibility::Compatible);
        type Case<'a> = (
            &'a str,
            &'a str,
            &'a str,
            &'a str,
            &'a str,
            Option<&'a str>,
            &'a str,
        );
        let cases: [Case<'_>; 6] = [
            (
                "kimi-cli",
                "acp-stdio",
                "sha256:abc",
                "acp-v1",
                "/work/project",
                None,
                "binding-provider-changed",
            ),
            (
                "qwen-code",
                "terminal",
                "sha256:abc",
                "acp-v1",
                "/work/project",
                None,
                "binding-transport-changed",
            ),
            (
                "qwen-code",
                "acp-stdio",
                "sha256:other",
                "acp-v1",
                "/work/project",
                None,
                "binding-installation-changed",
            ),
            (
                "qwen-code",
                "acp-stdio",
                "sha256:abc",
                "acp-v2",
                "/work/project",
                None,
                "binding-adapter-revision-changed",
            ),
            (
                "qwen-code",
                "acp-stdio",
                "sha256:abc",
                "acp-v1",
                "/work/other",
                None,
                "binding-workspace-changed",
            ),
            (
                "qwen-code",
                "acp-stdio",
                "sha256:abc",
                "acp-v1",
                "/work/project",
                Some("china"),
                "binding-account-profile-changed",
            ),
        ];
        for (provider, transport, fingerprint, revision, workspace, profile, expected) in cases {
            assert_eq!(
                stored.compatibility(
                    provider,
                    transport,
                    fingerprint,
                    revision,
                    workspace,
                    profile
                ),
                BindingCompatibility::Incompatible {
                    reason_code: expected
                }
            );
        }
        let mut no_load = record();
        no_load.peer_load_session = false;
        assert_eq!(
            no_load.compatibility(
                "qwen-code",
                "acp-stdio",
                "sha256:abc",
                "acp-v1",
                "/work/project",
                None
            ),
            BindingCompatibility::Incompatible {
                reason_code: "binding-peer-cannot-load"
            }
        );
        let mut no_external = record();
        no_external.external_session_id = None;
        assert_eq!(
            no_external.compatibility(
                "qwen-code",
                "acp-stdio",
                "sha256:abc",
                "acp-v1",
                "/work/project",
                None
            ),
            BindingCompatibility::Incompatible {
                reason_code: "binding-external-session-missing"
            }
        );
    }
}
