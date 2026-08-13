#![allow(dead_code)]

use super::SqliteSessionsRepository;
use crate::contexts::sessions::application::{
    CompletedInvocationAccounting, InvocationDetailQuery, ModelInvocationRecord,
    NewModelInvocation, NewUsageObservation, SessionsApplicationError, TokenAccountingQueryPort,
    TokenAccountingRepository, TokenUsageObservation, UsageAccountingSummary, UsageCursor,
    UsageCursorAdvance, UsageDetailPage, UsageSummaryQuery,
};
use crate::contexts::sessions::domain::{
    reconcile_cumulative_usage, CumulativeReconciliation, MeasurementQuality, TokenDimensions,
    UsageStatus,
};
use rusqlite::{params, Connection, OptionalExtension, Row, Transaction, TransactionBehavior};
use serde::{de::DeserializeOwned, Serialize};

pub(crate) fn apply_schema(
    connection: &Connection,
) -> Result<(), crate::platform::database::DatabaseError> {
    connection.execute_batch(
        r#"
        DROP TABLE IF EXISTS usage_records;

        CREATE TABLE IF NOT EXISTS model_invocations (
            id TEXT PRIMARY KEY,
            generation_id TEXT,
            run_id TEXT,
            operation_id TEXT,
            session_id TEXT NOT NULL,
            message_id TEXT,
            agent_id TEXT NOT NULL,
            provider_id TEXT,
            profile_id TEXT,
            endpoint_id TEXT,
            model_id TEXT,
            interaction_kind TEXT NOT NULL CHECK (interaction_kind IN (
                'managed-cli', 'terminal-cli', 'native-api'
            )),
            purpose TEXT NOT NULL CHECK (purpose IN (
                'assistant-initial', 'tool-continuation', 'context-compaction',
                'memory-extraction', 'retry', 'terminal-interval'
            )),
            request_sequence INTEGER NOT NULL CHECK (request_sequence >= 0),
            attempt INTEGER NOT NULL CHECK (attempt >= 0),
            status TEXT NOT NULL CHECK (status IN (
                'running', 'succeeded', 'failed', 'cancelled'
            )),
            started_at TEXT NOT NULL,
            completed_at TEXT,
            FOREIGN KEY (session_id) REFERENCES sessions(id) ON DELETE CASCADE,
            FOREIGN KEY (message_id) REFERENCES messages(id) ON DELETE SET NULL,
            FOREIGN KEY (agent_id) REFERENCES agents(id)
        );

        CREATE TABLE IF NOT EXISTS token_usage_observations (
            id TEXT PRIMARY KEY,
            invocation_id TEXT NOT NULL,
            quality TEXT NOT NULL CHECK (quality IN (
                'reported', 'reported-derived', 'estimated'
            )),
            unit TEXT NOT NULL CHECK (unit IN ('tokens', 'characters')),
            measurement_kind TEXT NOT NULL CHECK (measurement_kind IN (
                'interval', 'cumulative-snapshot'
            )),
            input_count INTEGER NOT NULL CHECK (input_count >= 0),
            output_count INTEGER NOT NULL CHECK (output_count >= 0),
            cached_input_count INTEGER NOT NULL CHECK (cached_input_count >= 0),
            cache_write_input_count INTEGER NOT NULL CHECK (cache_write_input_count >= 0),
            reasoning_output_count INTEGER NOT NULL CHECK (reasoning_output_count >= 0),
            provider_total_count INTEGER CHECK (provider_total_count >= 0),
            cache_overlap TEXT NOT NULL CHECK (cache_overlap IN (
                'subset', 'exclusive', 'unknown'
            )),
            reasoning_overlap TEXT NOT NULL CHECK (reasoning_overlap IN (
                'subset', 'exclusive', 'unknown'
            )),
            normalization_version TEXT NOT NULL,
            source TEXT NOT NULL,
            source_key TEXT NOT NULL UNIQUE,
            source_revision TEXT,
            supersedes_observation_id TEXT,
            superseded_by_observation_id TEXT,
            event_at TEXT,
            observed_at TEXT NOT NULL,
            provenance_hash TEXT,
            FOREIGN KEY (invocation_id) REFERENCES model_invocations(id) ON DELETE CASCADE,
            FOREIGN KEY (supersedes_observation_id)
                REFERENCES token_usage_observations(id),
            FOREIGN KEY (superseded_by_observation_id)
                REFERENCES token_usage_observations(id),
            CHECK (
                (quality IN ('reported', 'reported-derived') AND unit = 'tokens') OR
                (quality = 'estimated' AND unit = 'characters')
            )
        );

        CREATE TABLE IF NOT EXISTS usage_ingestion_cursors (
            source_id TEXT PRIMARY KEY,
            provider_session_id TEXT NOT NULL,
            epoch INTEGER NOT NULL CHECK (epoch >= 0),
            input_count INTEGER NOT NULL CHECK (input_count >= 0),
            output_count INTEGER NOT NULL CHECK (output_count >= 0),
            cached_input_count INTEGER NOT NULL CHECK (cached_input_count >= 0),
            cache_write_input_count INTEGER NOT NULL CHECK (cache_write_input_count >= 0),
            reasoning_output_count INTEGER NOT NULL CHECK (reasoning_output_count >= 0),
            provider_total_count INTEGER CHECK (provider_total_count >= 0),
            ordering_key TEXT NOT NULL,
            source_revision TEXT,
            revision INTEGER NOT NULL CHECK (revision >= 0),
            updated_at TEXT NOT NULL
        );

        CREATE INDEX IF NOT EXISTS idx_model_invocations_session_started
            ON model_invocations(session_id, started_at, id);
        CREATE INDEX IF NOT EXISTS idx_model_invocations_message
            ON model_invocations(message_id, request_sequence, attempt);
        CREATE INDEX IF NOT EXISTS idx_model_invocations_run
            ON model_invocations(run_id, request_sequence, attempt);
        CREATE INDEX IF NOT EXISTS idx_model_invocations_dimensions
            ON model_invocations(agent_id, provider_id, model_id, purpose, status, started_at);
        CREATE INDEX IF NOT EXISTS idx_usage_observations_active_time
            ON token_usage_observations(observed_at, quality, invocation_id)
            WHERE superseded_by_observation_id IS NULL;
        CREATE INDEX IF NOT EXISTS idx_usage_observations_invocation
            ON token_usage_observations(invocation_id, observed_at, id);
        CREATE INDEX IF NOT EXISTS idx_usage_observations_supersedes
            ON token_usage_observations(supersedes_observation_id);
        CREATE INDEX IF NOT EXISTS idx_usage_cursors_session
            ON usage_ingestion_cursors(provider_session_id, epoch, updated_at);

        "#,
    )?;
    Ok(())
}

impl TokenAccountingRepository for SqliteSessionsRepository {
    fn start_invocation(
        &self,
        invocation: &NewModelInvocation,
    ) -> Result<ModelInvocationRecord, SessionsApplicationError> {
        let connection = self.connection()?;
        insert_invocation(&connection, invocation)?;
        let saved = load_invocation(&connection, &invocation.id)?.ok_or_else(|| {
            SessionsApplicationError::Repository("invocation was not persisted".to_string())
        })?;
        if saved.invocation != *invocation {
            return Err(SessionsApplicationError::Validation(
                "invocation id already identifies a different immutable snapshot".to_string(),
            ));
        }
        Ok(saved)
    }

    fn finalize_invocation(
        &self,
        invocation_id: &str,
        status: UsageStatus,
        completed_at: &str,
    ) -> Result<ModelInvocationRecord, SessionsApplicationError> {
        if status == UsageStatus::Running {
            return Err(SessionsApplicationError::Validation(
                "finalization requires a terminal invocation status".to_string(),
            ));
        }
        let connection = self.connection()?;
        let status_value = storage(status)?;
        let changed = connection
            .execute(
                "UPDATE model_invocations
                 SET status = ?1, completed_at = ?2
                 WHERE id = ?3
                   AND (status = 'running' OR (status = ?1 AND completed_at = ?2))",
                params![status_value, completed_at, invocation_id],
            )
            .map_err(repository_error)?;
        if changed == 0 {
            return match load_invocation(&connection, invocation_id)? {
                Some(_) => Err(SessionsApplicationError::Validation(
                    "a finalized invocation cannot change terminal status or timestamp".to_string(),
                )),
                None => Err(SessionsApplicationError::Repository(format!(
                    "invocation not found: {invocation_id}"
                ))),
            };
        }
        load_invocation(&connection, invocation_id)?.ok_or_else(|| {
            SessionsApplicationError::Repository("finalized invocation disappeared".to_string())
        })
    }

    fn record_observation(
        &self,
        observation: &NewUsageObservation,
    ) -> Result<TokenUsageObservation, SessionsApplicationError> {
        validate_observation(observation)?;
        let mut connection = self.connection()?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(repository_error)?;
        insert_observation(&transaction, observation)?;
        let saved = load_observation(&transaction, &observation.source_key)?.ok_or_else(|| {
            SessionsApplicationError::Repository("observation was not persisted".to_string())
        })?;
        transaction.commit().map_err(repository_error)?;
        Ok(saved)
    }

    fn advance_cursor(
        &self,
        advance: &UsageCursorAdvance,
    ) -> Result<UsageCursor, SessionsApplicationError> {
        advance
            .current
            .dimensions
            .validate()
            .map_err(|message| SessionsApplicationError::Validation(message.to_string()))?;
        let mut connection = self.connection()?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(repository_error)?;
        let stored = load_cursor(&transaction, &advance.current.source_id)?;
        if stored != advance.previous {
            return Err(SessionsApplicationError::Transaction(
                "usage cursor revision conflict".to_string(),
            ));
        }
        validate_cursor_advance(advance)?;
        save_cursor(&transaction, &advance.current)?;
        if let Some(observation) = &advance.observation {
            validate_observation(observation)?;
            insert_observation(&transaction, observation)?;
        }
        transaction.commit().map_err(repository_error)?;
        Ok(advance.current.clone())
    }

    fn find_cursor(
        &self,
        source_id: &str,
    ) -> Result<Option<UsageCursor>, SessionsApplicationError> {
        let connection = self.connection()?;
        load_cursor(&connection, source_id)
    }
}

pub(super) fn persist_completed_invocation(
    transaction: &Transaction<'_>,
    accounting: &CompletedInvocationAccounting,
) -> Result<(), SessionsApplicationError> {
    if accounting.status == UsageStatus::Running
        || accounting.observation.invocation_id != accounting.invocation.id
    {
        return Err(SessionsApplicationError::Validation(
            "completed invocation accounting requires matching terminal records".to_string(),
        ));
    }
    insert_invocation(transaction, &accounting.invocation)?;
    validate_observation(&accounting.observation)?;
    insert_observation(transaction, &accounting.observation)?;
    let changed = transaction
        .execute(
            "UPDATE model_invocations
             SET status = ?1, completed_at = ?2
             WHERE id = ?3
               AND (status = 'running' OR (status = ?1 AND completed_at = ?2))",
            params![
                storage(accounting.status)?,
                accounting.completed_at,
                accounting.invocation.id,
            ],
        )
        .map_err(repository_error)?;
    if changed == 0 {
        return Err(SessionsApplicationError::Validation(
            "a finalized invocation cannot change terminal status or timestamp".to_string(),
        ));
    }
    Ok(())
}

fn insert_invocation(
    connection: &Connection,
    invocation: &NewModelInvocation,
) -> Result<(), SessionsApplicationError> {
    connection
        .execute(
            r#"INSERT OR IGNORE INTO model_invocations (
                id, generation_id, run_id, operation_id, session_id, message_id,
                agent_id, provider_id, profile_id, endpoint_id, model_id,
                interaction_kind, purpose, request_sequence, attempt, status, started_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, 'running', ?16)"#,
            params![
                invocation.id,
                invocation.generation_id,
                invocation.run_id,
                invocation.operation_id,
                invocation.session_id,
                invocation.message_id,
                invocation.agent_id,
                invocation.provider_id,
                invocation.profile_id,
                invocation.endpoint_id,
                invocation.model_id,
                storage(invocation.interaction_kind)?,
                storage(invocation.purpose)?,
                i64::from(invocation.request_sequence),
                i64::from(invocation.attempt),
                invocation.started_at,
            ],
        )
        .map_err(repository_error)?;
    let saved = load_invocation(connection, &invocation.id)?.ok_or_else(|| {
        SessionsApplicationError::Repository("invocation was not persisted".to_string())
    })?;
    if saved.invocation != *invocation {
        return Err(SessionsApplicationError::Validation(
            "invocation id already identifies a different immutable snapshot".to_string(),
        ));
    }
    Ok(())
}

impl TokenAccountingQueryPort for SqliteSessionsRepository {
    fn usage_summary(
        &self,
        query: &UsageSummaryQuery,
    ) -> Result<UsageAccountingSummary, SessionsApplicationError> {
        let connection = self.connection()?;
        super::usage_accounting_projection::project_usage_summary(&connection, query)
    }

    fn invocation_details(
        &self,
        query: &InvocationDetailQuery,
    ) -> Result<UsageDetailPage, SessionsApplicationError> {
        let limit = query.limit.clamp(1, 100);
        let connection = self.connection()?;
        let mut statement = connection
            .prepare(
                r#"SELECT id, generation_id, run_id, operation_id, session_id, message_id,
                           agent_id, provider_id, profile_id, endpoint_id, model_id,
                           interaction_kind, purpose, request_sequence, attempt, status,
                           started_at, completed_at
                    FROM model_invocations
                    WHERE (?1 IS NULL OR session_id = ?1)
                      AND (?2 IS NULL OR agent_id = ?2)
                      AND (?3 IS NULL OR provider_id = ?3)
                      AND (?4 IS NULL OR model_id = ?4)
                      AND (?5 IS NULL OR purpose = ?5)
                      AND (?6 IS NULL OR status = ?6)
                      AND (?7 IS NULL OR EXISTS (
                          SELECT 1 FROM token_usage_observations observation
                          WHERE observation.invocation_id = model_invocations.id
                            AND observation.quality = ?7
                            AND observation.superseded_by_observation_id IS NULL
                      ))
                      AND (?8 IS NULL OR id > ?8)
                    ORDER BY id LIMIT ?9"#,
            )
            .map_err(repository_error)?;
        let purpose = query.purpose.map(storage).transpose()?;
        let status = query.status.map(storage).transpose()?;
        let invocations = statement
            .query_map(
                params![
                    query.session_id,
                    query.agent_id,
                    query.provider_id,
                    query.model_id,
                    purpose,
                    status,
                    query.quality.map(storage).transpose()?,
                    query.after_id,
                    limit as i64 + 1,
                ],
                invocation_from_row,
            )
            .map_err(repository_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(repository_error)?;
        let next_cursor =
            (invocations.len() > limit).then(|| invocations[limit - 1].invocation.id.clone());
        let invocations = invocations.into_iter().take(limit).collect::<Vec<_>>();
        let mut observations = Vec::new();
        for invocation in &invocations {
            observations.extend(observations_for_invocation(
                &connection,
                &invocation.invocation.id,
                query.quality,
            )?);
        }
        Ok(UsageDetailPage {
            invocations,
            observations,
            next_cursor,
        })
    }
}

fn insert_observation(
    transaction: &Transaction<'_>,
    observation: &NewUsageObservation,
) -> Result<(), SessionsApplicationError> {
    let inserted = transaction
        .execute(
            r#"INSERT OR IGNORE INTO token_usage_observations (
                id, invocation_id, quality, unit, measurement_kind, input_count,
                output_count, cached_input_count, cache_write_input_count,
                reasoning_output_count, provider_total_count, cache_overlap,
                reasoning_overlap, normalization_version, source, source_key,
                source_revision, supersedes_observation_id, event_at, observed_at,
                provenance_hash
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11,
                      ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21)"#,
            params![
                observation.id,
                observation.invocation_id,
                storage(observation.quality)?,
                storage(observation.unit)?,
                storage(observation.measurement_kind)?,
                observation.dimensions.input,
                observation.dimensions.output,
                observation.dimensions.cached_input,
                observation.dimensions.cache_write_input,
                observation.dimensions.reasoning_output,
                observation.dimensions.provider_total,
                storage(observation.cache_overlap)?,
                storage(observation.reasoning_overlap)?,
                observation.normalization_version,
                observation.source,
                observation.source_key,
                observation.source_revision,
                observation.supersedes_observation_id,
                observation.event_at,
                observation.observed_at,
                observation.provenance_hash,
            ],
        )
        .map_err(repository_error)?;
    if inserted == 0 {
        let existing =
            load_observation(transaction, &observation.source_key)?.ok_or_else(|| {
                SessionsApplicationError::Repository(
                    "duplicate observation disappeared".to_string(),
                )
            })?;
        if !same_observation_facts(&existing.observation, observation) {
            return Err(SessionsApplicationError::Validation(
                "source key already identifies different usage facts".to_string(),
            ));
        }
        return Ok(());
    }
    if let Some(previous_id) = observation.supersedes_observation_id.as_deref() {
        let previous = load_observation_by_id(transaction, previous_id)?.ok_or_else(|| {
            SessionsApplicationError::Validation(
                "superseded observation does not exist".to_string(),
            )
        })?;
        if quality_rank(observation.quality) < quality_rank(previous.observation.quality) {
            return Err(SessionsApplicationError::Validation(
                "a lower-quality observation cannot supersede a higher-quality observation"
                    .to_string(),
            ));
        }
        let changed = transaction
            .execute(
                "UPDATE token_usage_observations SET superseded_by_observation_id = ?1 WHERE id = ?2 AND invocation_id = ?3 AND superseded_by_observation_id IS NULL",
                params![observation.id, previous_id, observation.invocation_id],
            )
            .map_err(repository_error)?;
        if changed != 1 {
            return Err(SessionsApplicationError::Validation(
                "superseded observation is missing, inactive, or belongs to another invocation"
                    .to_string(),
            ));
        }
    }
    Ok(())
}

fn save_cursor(
    transaction: &Transaction<'_>,
    cursor: &UsageCursor,
) -> Result<(), SessionsApplicationError> {
    transaction
        .execute(
            r#"INSERT INTO usage_ingestion_cursors (
                source_id, provider_session_id, epoch, input_count, output_count,
                cached_input_count, cache_write_input_count, reasoning_output_count,
                provider_total_count, ordering_key, source_revision, revision, updated_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)
            ON CONFLICT(source_id) DO UPDATE SET
                provider_session_id = excluded.provider_session_id,
                epoch = excluded.epoch,
                input_count = excluded.input_count,
                output_count = excluded.output_count,
                cached_input_count = excluded.cached_input_count,
                cache_write_input_count = excluded.cache_write_input_count,
                reasoning_output_count = excluded.reasoning_output_count,
                provider_total_count = excluded.provider_total_count,
                ordering_key = excluded.ordering_key,
                source_revision = excluded.source_revision,
                revision = excluded.revision,
                updated_at = excluded.updated_at"#,
            params![
                cursor.source_id,
                cursor.provider_session_id,
                checked_i64(cursor.epoch, "cursor epoch")?,
                cursor.dimensions.input,
                cursor.dimensions.output,
                cursor.dimensions.cached_input,
                cursor.dimensions.cache_write_input,
                cursor.dimensions.reasoning_output,
                cursor.dimensions.provider_total,
                cursor.ordering_key,
                cursor.source_revision,
                checked_i64(cursor.revision, "cursor revision")?,
                cursor.updated_at,
            ],
        )
        .map_err(repository_error)?;
    Ok(())
}

fn load_invocation(
    connection: &Connection,
    id: &str,
) -> Result<Option<ModelInvocationRecord>, SessionsApplicationError> {
    connection
        .query_row(
            r#"SELECT id, generation_id, run_id, operation_id, session_id, message_id,
                      agent_id, provider_id, profile_id, endpoint_id, model_id,
                      interaction_kind, purpose, request_sequence, attempt, status,
                      started_at, completed_at
               FROM model_invocations WHERE id = ?1"#,
            [id],
            invocation_from_row,
        )
        .optional()
        .map_err(repository_error)
}

fn invocation_from_row(row: &Row<'_>) -> rusqlite::Result<ModelInvocationRecord> {
    let interaction = parse(row.get::<_, String>(11)?)?;
    let purpose = parse(row.get::<_, String>(12)?)?;
    let status = parse(row.get::<_, String>(15)?)?;
    Ok(ModelInvocationRecord {
        invocation: NewModelInvocation {
            id: row.get(0)?,
            generation_id: row.get(1)?,
            run_id: row.get(2)?,
            operation_id: row.get(3)?,
            session_id: row.get(4)?,
            message_id: row.get(5)?,
            agent_id: row.get(6)?,
            provider_id: row.get(7)?,
            profile_id: row.get(8)?,
            endpoint_id: row.get(9)?,
            model_id: row.get(10)?,
            interaction_kind: interaction,
            purpose,
            request_sequence: non_negative_u32(row, 13)?,
            attempt: non_negative_u32(row, 14)?,
            started_at: row.get(16)?,
        },
        status,
        completed_at: row.get(17)?,
    })
}

fn load_observation(
    connection: &Connection,
    source_key: &str,
) -> Result<Option<TokenUsageObservation>, SessionsApplicationError> {
    connection
        .query_row(
            &format!("{OBSERVATION_SELECT} WHERE source_key = ?1"),
            [source_key],
            observation_from_row,
        )
        .optional()
        .map_err(repository_error)
}

fn load_observation_by_id(
    connection: &Connection,
    id: &str,
) -> Result<Option<TokenUsageObservation>, SessionsApplicationError> {
    connection
        .query_row(
            &format!("{OBSERVATION_SELECT} WHERE id = ?1"),
            [id],
            observation_from_row,
        )
        .optional()
        .map_err(repository_error)
}

fn observations_for_invocation(
    connection: &Connection,
    invocation_id: &str,
    quality: Option<MeasurementQuality>,
) -> Result<Vec<TokenUsageObservation>, SessionsApplicationError> {
    let quality = quality.map(storage).transpose()?;
    let mut statement = connection
        .prepare(&format!(
            "{OBSERVATION_SELECT} WHERE invocation_id = ?1
             AND superseded_by_observation_id IS NULL
             AND (?2 IS NULL OR quality = ?2) ORDER BY observed_at, id"
        ))
        .map_err(repository_error)?;
    let observations = statement
        .query_map(params![invocation_id, quality], observation_from_row)
        .map_err(repository_error)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(repository_error)?;
    Ok(observations)
}

const OBSERVATION_SELECT: &str = r#"SELECT id, invocation_id, quality, unit,
    measurement_kind, input_count, output_count, cached_input_count,
    cache_write_input_count, reasoning_output_count, provider_total_count,
    cache_overlap, reasoning_overlap, normalization_version, source, source_key,
    source_revision, supersedes_observation_id, event_at, observed_at,
    provenance_hash, superseded_by_observation_id
    FROM token_usage_observations"#;

fn observation_from_row(row: &Row<'_>) -> rusqlite::Result<TokenUsageObservation> {
    Ok(TokenUsageObservation {
        observation: NewUsageObservation {
            id: row.get(0)?,
            invocation_id: row.get(1)?,
            quality: parse(row.get::<_, String>(2)?)?,
            unit: parse(row.get::<_, String>(3)?)?,
            measurement_kind: parse(row.get::<_, String>(4)?)?,
            dimensions: TokenDimensions {
                input: row.get(5)?,
                output: row.get(6)?,
                cached_input: row.get(7)?,
                cache_write_input: row.get(8)?,
                reasoning_output: row.get(9)?,
                provider_total: row.get(10)?,
            },
            cache_overlap: parse(row.get::<_, String>(11)?)?,
            reasoning_overlap: parse(row.get::<_, String>(12)?)?,
            normalization_version: row.get(13)?,
            source: row.get(14)?,
            source_key: row.get(15)?,
            source_revision: row.get(16)?,
            supersedes_observation_id: row.get(17)?,
            event_at: row.get(18)?,
            observed_at: row.get(19)?,
            provenance_hash: row.get(20)?,
        },
        superseded_by_observation_id: row.get(21)?,
    })
}

fn load_cursor(
    connection: &Connection,
    source_id: &str,
) -> Result<Option<UsageCursor>, SessionsApplicationError> {
    connection
        .query_row(
            r#"SELECT source_id, provider_session_id, epoch, input_count,
                      output_count, cached_input_count, cache_write_input_count,
                      reasoning_output_count, provider_total_count, ordering_key,
                      source_revision, revision, updated_at
               FROM usage_ingestion_cursors WHERE source_id = ?1"#,
            [source_id],
            |row| {
                Ok(UsageCursor {
                    source_id: row.get(0)?,
                    provider_session_id: row.get(1)?,
                    epoch: non_negative_u64(row, 2)?,
                    dimensions: TokenDimensions {
                        input: row.get(3)?,
                        output: row.get(4)?,
                        cached_input: row.get(5)?,
                        cache_write_input: row.get(6)?,
                        reasoning_output: row.get(7)?,
                        provider_total: row.get(8)?,
                    },
                    ordering_key: row.get(9)?,
                    source_revision: row.get(10)?,
                    revision: non_negative_u64(row, 11)?,
                    updated_at: row.get(12)?,
                })
            },
        )
        .optional()
        .map_err(repository_error)
}

pub(super) fn storage<T: Serialize>(value: T) -> Result<String, SessionsApplicationError> {
    serde_json::to_value(value)
        .map_err(|error| SessionsApplicationError::Serialization(error.to_string()))?
        .as_str()
        .map(str::to_string)
        .ok_or_else(|| SessionsApplicationError::Serialization("invalid enum value".to_string()))
}

fn parse<T: DeserializeOwned>(value: String) -> rusqlite::Result<T> {
    serde_json::from_value(serde_json::Value::String(value)).map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(error))
    })
}

fn non_negative_u32(row: &Row<'_>, index: usize) -> rusqlite::Result<u32> {
    let value = row.get::<_, i64>(index)?;
    u32::try_from(value).map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(
            index,
            rusqlite::types::Type::Integer,
            Box::new(error),
        )
    })
}

fn non_negative_u64(row: &Row<'_>, index: usize) -> rusqlite::Result<u64> {
    let value = row.get::<_, i64>(index)?;
    u64::try_from(value).map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(
            index,
            rusqlite::types::Type::Integer,
            Box::new(error),
        )
    })
}

fn repository_error(error: rusqlite::Error) -> SessionsApplicationError {
    SessionsApplicationError::Repository(error.to_string())
}

fn validate_observation(observation: &NewUsageObservation) -> Result<(), SessionsApplicationError> {
    observation
        .dimensions
        .validate()
        .map_err(|message| SessionsApplicationError::Validation(message.to_string()))?;
    if observation.source_key.trim().is_empty()
        || observation.source.trim().is_empty()
        || observation.normalization_version.trim().is_empty()
    {
        return Err(SessionsApplicationError::Validation(
            "usage source, source key, and normalization version are required".to_string(),
        ));
    }
    let valid_unit = matches!(
        (observation.quality, observation.unit),
        (
            MeasurementQuality::Reported | MeasurementQuality::ReportedDerived,
            crate::contexts::sessions::domain::AccountingUnit::Tokens
        ) | (
            MeasurementQuality::Estimated,
            crate::contexts::sessions::domain::AccountingUnit::Characters
        )
    );
    if !valid_unit {
        return Err(SessionsApplicationError::Validation(
            "reported usage must use Tokens and estimates must use characters".to_string(),
        ));
    }
    Ok(())
}

fn validate_cursor_advance(advance: &UsageCursorAdvance) -> Result<(), SessionsApplicationError> {
    let current = &advance.current;
    match &advance.previous {
        None if current.revision != 0 || current.epoch != 0 => {
            Err(SessionsApplicationError::Validation(
                "a new usage cursor must begin at revision and epoch zero".to_string(),
            ))
        }
        None => Ok(()),
        Some(previous) => {
            let expected_revision = previous.revision.checked_add(1).ok_or_else(|| {
                SessionsApplicationError::Validation("usage cursor revision overflow".to_string())
            })?;
            if current.revision != expected_revision
                || current.ordering_key <= previous.ordering_key
                || current.epoch < previous.epoch
                || current.epoch > previous.epoch.saturating_add(1)
            {
                return Err(SessionsApplicationError::Validation(
                    "stale or discontinuous usage cursor advance".to_string(),
                ));
            }
            let session_changed = current.provider_session_id != previous.provider_session_id;
            let epoch_changed = current.epoch != previous.epoch;
            if session_changed && !epoch_changed {
                return Err(SessionsApplicationError::Validation(
                    "provider session changes must open exactly one new cursor epoch".to_string(),
                ));
            }
            if !epoch_changed
                && reconcile_cumulative_usage(previous.dimensions, current.dimensions)
                    == CumulativeReconciliation::Reset
            {
                return Err(SessionsApplicationError::Validation(
                    "a cumulative counter reset must open a new cursor epoch".to_string(),
                ));
            }
            Ok(())
        }
    }
}

fn quality_rank(quality: MeasurementQuality) -> u8 {
    match quality {
        MeasurementQuality::Estimated => 0,
        MeasurementQuality::ReportedDerived => 1,
        MeasurementQuality::Reported => 2,
    }
}

fn same_observation_facts(left: &NewUsageObservation, right: &NewUsageObservation) -> bool {
    left.invocation_id == right.invocation_id
        && left.quality == right.quality
        && left.unit == right.unit
        && left.measurement_kind == right.measurement_kind
        && left.dimensions == right.dimensions
        && left.cache_overlap == right.cache_overlap
        && left.reasoning_overlap == right.reasoning_overlap
        && left.normalization_version == right.normalization_version
        && left.source == right.source
        && left.source_key == right.source_key
        && left.source_revision == right.source_revision
        && left.supersedes_observation_id == right.supersedes_observation_id
        && left.event_at == right.event_at
        && left.provenance_hash == right.provenance_hash
}

fn checked_i64(value: u64, field: &str) -> Result<i64, SessionsApplicationError> {
    i64::try_from(value).map_err(|_| {
        SessionsApplicationError::Validation(format!("{field} exceeds SQLite integer range"))
    })
}
