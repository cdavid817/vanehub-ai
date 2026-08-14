use crate::contexts::artifacts::application::{
    ArtifactCatalogPort, ArtifactCreator, ArtifactDescriptor, ArtifactEvidenceKind,
    ArtifactPublicationReference, ArtifactServiceError, ArtifactVisibility,
};
use crate::platform::database::{table_has_column, DatabaseError, NativeDatabase};
use rusqlite::{params, Connection, OptionalExtension};

#[derive(Clone)]
pub(crate) struct SqliteArtifactCatalog {
    database: NativeDatabase,
}

impl SqliteArtifactCatalog {
    pub(crate) fn new(database: NativeDatabase) -> Self {
        Self { database }
    }
}

impl ArtifactCatalogPort for SqliteArtifactCatalog {
    fn insert_immutable(&self, artifact: &ArtifactDescriptor) -> Result<(), ArtifactServiceError> {
        let mut connection = self.database.connection().map_err(catalog_error)?;
        let transaction = connection.transaction().map_err(catalog_error)?;
        transaction
            .execute(
                r#"
                INSERT INTO native_tool_artifacts (
                    id, contract_version, content_hash, media_type, size_bytes, display_name,
                    source_operation_id, created_at, expires_at, publication_ref,
                    creator_kind, creator_id, evidence_kind, visibility
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, NULL, ?10, ?11, ?12, ?13)
                "#,
                params![
                    artifact.id,
                    artifact.contract_version,
                    artifact.content_hash,
                    artifact.media_type,
                    checked_i64(artifact.size_bytes)?,
                    artifact.display_name,
                    existing_operation(&transaction, &artifact.source_operation_id)?,
                    artifact.created_at,
                    artifact.expires_at,
                    artifact.creator.kind,
                    artifact.creator.id,
                    evidence_kind(artifact.evidence_kind),
                    visibility(artifact.visibility),
                ],
            )
            .map_err(catalog_error)?;
        for (ordinal, source) in artifact.source_artifact_ids.iter().enumerate() {
            transaction
                .execute(
                    "INSERT INTO native_tool_artifact_lineage \
                     (artifact_id, source_artifact_id, ordinal) VALUES (?1, ?2, ?3)",
                    params![artifact.id, source, checked_i64(ordinal as u64)?],
                )
                .map_err(catalog_error)?;
        }
        transaction.commit().map_err(catalog_error)
    }

    fn get(&self, artifact_id: &str) -> Result<Option<ArtifactDescriptor>, ArtifactServiceError> {
        let connection = self.database.connection().map_err(catalog_error)?;
        let descriptor = connection
            .query_row(
                &format!("{} WHERE id = ?1", descriptor_select()),
                [artifact_id],
                descriptor_row,
            )
            .optional()
            .map_err(catalog_error)?;
        descriptor
            .map(|value| with_lineage(&connection, value))
            .transpose()
    }

    fn list(&self, limit: usize) -> Result<Vec<ArtifactDescriptor>, ArtifactServiceError> {
        let connection = self.database.connection().map_err(catalog_error)?;
        let sql = format!(
            "{} ORDER BY created_at DESC, id DESC LIMIT ?1",
            descriptor_select()
        );
        let mut statement = connection.prepare(&sql).map_err(catalog_error)?;
        let rows = statement
            .query_map([checked_i64(limit as u64)?], descriptor_row)
            .map_err(catalog_error)?;
        let descriptors = rows.collect::<Result<Vec<_>, _>>().map_err(catalog_error)?;
        descriptors
            .into_iter()
            .map(|value| with_lineage(&connection, value))
            .collect()
    }

    fn publish(
        &self,
        publication: &ArtifactPublicationReference,
    ) -> Result<(), ArtifactServiceError> {
        let changed = self
            .database
            .connection()
            .map_err(catalog_error)?
            .execute(
                "UPDATE native_tool_artifacts SET publication_ref = ?1, visibility = ?2 \
                 WHERE id = ?3 AND content_hash = ?4 AND publication_ref IS NULL",
                params![
                    publication.reference,
                    visibility(publication.visibility),
                    publication.artifact_id,
                    publication.content_hash,
                ],
            )
            .map_err(catalog_error)?;
        if changed == 1 {
            Ok(())
        } else {
            Err(ArtifactServiceError::PublicationFailure)
        }
    }

    fn expired_candidates(
        &self,
        now: &str,
        limit: usize,
    ) -> Result<Vec<(ArtifactDescriptor, bool)>, ArtifactServiceError> {
        let connection = self.database.connection().map_err(catalog_error)?;
        let sql = format!(
            "{} WHERE expires_at IS NOT NULL AND expires_at <= ?1 \
             ORDER BY expires_at, id LIMIT ?2",
            descriptor_select()
        );
        let mut statement = connection.prepare(&sql).map_err(catalog_error)?;
        let rows = statement
            .query_map(params![now, checked_i64(limit as u64)?], descriptor_row)
            .map_err(catalog_error)?;
        let mut candidates = Vec::new();
        for row in rows {
            let descriptor = with_lineage(&connection, row.map_err(catalog_error)?)?;
            let referenced: bool = connection
                .query_row(
                    "SELECT EXISTS(SELECT 1 FROM native_tool_artifact_lineage \
                     WHERE source_artifact_id = ?1)",
                    [&descriptor.id],
                    |row| row.get(0),
                )
                .map_err(catalog_error)?;
            candidates.push((descriptor, referenced));
        }
        Ok(candidates)
    }

    fn remove(&self, artifact_id: &str) -> Result<(), ArtifactServiceError> {
        let changed = self
            .database
            .connection()
            .map_err(catalog_error)?
            .execute(
                "DELETE FROM native_tool_artifacts WHERE id = ?1 AND publication_ref IS NULL",
                [artifact_id],
            )
            .map_err(catalog_error)?;
        if changed == 1 {
            Ok(())
        } else {
            Err(ArtifactServiceError::CatalogFailure)
        }
    }

    fn count_by_hash(&self, content_hash: &str) -> Result<u64, ArtifactServiceError> {
        let count: i64 = self
            .database
            .connection()
            .map_err(catalog_error)?
            .query_row(
                "SELECT COUNT(*) FROM native_tool_artifacts WHERE content_hash = ?1",
                [content_hash],
                |row| row.get(0),
            )
            .map_err(catalog_error)?;
        u64::try_from(count).map_err(|_| ArtifactServiceError::CatalogFailure)
    }
}

pub(crate) fn apply_artifact_catalog_schema(connection: &Connection) -> Result<(), DatabaseError> {
    for (column, sql) in [
        ("creator_kind", "ALTER TABLE native_tool_artifacts ADD COLUMN creator_kind TEXT NOT NULL DEFAULT 'legacy'"),
        ("creator_id", "ALTER TABLE native_tool_artifacts ADD COLUMN creator_id TEXT NOT NULL DEFAULT 'legacy'"),
        ("evidence_kind", "ALTER TABLE native_tool_artifacts ADD COLUMN evidence_kind TEXT NOT NULL DEFAULT 'host_verified'"),
        ("visibility", "ALTER TABLE native_tool_artifacts ADD COLUMN visibility TEXT NOT NULL DEFAULT 'private'"),
    ] {
        if !table_has_column(connection, "native_tool_artifacts", column)? {
            connection.execute(sql, [])?;
        }
    }
    Ok(())
}

fn descriptor_select() -> &'static str {
    "SELECT id, contract_version, content_hash, media_type, size_bytes, display_name, \
     COALESCE(source_operation_id, 'unknown'), created_at, expires_at, creator_kind, creator_id, \
     evidence_kind, visibility FROM native_tool_artifacts"
}

fn descriptor_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<ArtifactDescriptor> {
    Ok(ArtifactDescriptor {
        id: row.get(0)?,
        contract_version: row.get(1)?,
        content_hash: row.get(2)?,
        media_type: row.get(3)?,
        size_bytes: row.get::<_, i64>(4).and_then(|value| {
            u64::try_from(value).map_err(|_| rusqlite::Error::IntegralValueOutOfRange(4, value))
        })?,
        display_name: row.get(5)?,
        source_operation_id: row.get(6)?,
        created_at: row.get(7)?,
        expires_at: row.get(8)?,
        creator: ArtifactCreator {
            kind: row.get(9)?,
            id: row.get(10)?,
        },
        evidence_kind: parse_evidence(row.get::<_, String>(11)?.as_str())?,
        visibility: parse_visibility(row.get::<_, String>(12)?.as_str())?,
        source_artifact_ids: Vec::new(),
    })
}

fn with_lineage(
    connection: &Connection,
    mut descriptor: ArtifactDescriptor,
) -> Result<ArtifactDescriptor, ArtifactServiceError> {
    let mut statement = connection
        .prepare(
            "SELECT source_artifact_id FROM native_tool_artifact_lineage \
             WHERE artifact_id = ?1 ORDER BY ordinal",
        )
        .map_err(catalog_error)?;
    descriptor.source_artifact_ids = statement
        .query_map([&descriptor.id], |row| row.get(0))
        .map_err(catalog_error)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(catalog_error)?;
    Ok(descriptor)
}

fn existing_operation(
    connection: &Connection,
    operation_id: &str,
) -> Result<Option<String>, ArtifactServiceError> {
    connection
        .query_row(
            "SELECT id FROM native_tool_operations WHERE id = ?1",
            [operation_id],
            |row| row.get(0),
        )
        .optional()
        .map_err(catalog_error)
}

fn evidence_kind(value: ArtifactEvidenceKind) -> &'static str {
    match value {
        ArtifactEvidenceKind::HostVerified => "host_verified",
        ArtifactEvidenceKind::ProviderReported => "provider_reported",
        ArtifactEvidenceKind::UntrustedExternal => "untrusted_external",
    }
}

fn parse_evidence(value: &str) -> rusqlite::Result<ArtifactEvidenceKind> {
    match value {
        "host_verified" => Ok(ArtifactEvidenceKind::HostVerified),
        "provider_reported" => Ok(ArtifactEvidenceKind::ProviderReported),
        "untrusted_external" => Ok(ArtifactEvidenceKind::UntrustedExternal),
        _ => Err(rusqlite::Error::InvalidQuery),
    }
}

fn visibility(value: ArtifactVisibility) -> &'static str {
    match value {
        ArtifactVisibility::Private => "private",
        ArtifactVisibility::Session => "session",
    }
}

fn parse_visibility(value: &str) -> rusqlite::Result<ArtifactVisibility> {
    match value {
        "private" => Ok(ArtifactVisibility::Private),
        "session" => Ok(ArtifactVisibility::Session),
        _ => Err(rusqlite::Error::InvalidQuery),
    }
}

fn checked_i64(value: u64) -> Result<i64, ArtifactServiceError> {
    i64::try_from(value).map_err(|_| ArtifactServiceError::CatalogFailure)
}

fn catalog_error(_: impl std::fmt::Display) -> ArtifactServiceError {
    ArtifactServiceError::CatalogFailure
}

#[cfg(test)]
#[path = "sqlite_catalog_tests.rs"]
mod tests;
