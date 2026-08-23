// See the domain's `identity.rs` for why this lands ahead of its production caller.
#![cfg_attr(not(test), allow(dead_code))]

//! SQLite adapters for rule sets and the active pointer.
//!
//! `specificity` is computed here from the rule and never taken from a caller. It is derived, so
//! accepting it would let a writer store an ordering that disagrees with the one the evaluator
//! computes — and the disagreement would only ever show up as a trace naming the wrong decisive
//! rule, which is the kind of wrong nobody notices.
//!
//! Reads fail rather than skip. A row whose source, effect, matcher, scope, or provenance this
//! build cannot parse makes the whole rule set unreadable, because the alternative is silently
//! evaluating a set that is missing exactly the `Deny` that failed to parse.

use crate::contexts::permissions::application::rules::{
    ActiveRuleSetRepository, RuleSetRepository,
};
use crate::contexts::permissions::domain::rules::{
    ActiveRuleSet, ActiveRuleSetError, AllowedScopes, AuthorizationRule, Matcher, OperationName,
    RuleEffect, RuleId, RuleProvenance, RuleScope, RuleSet, RuleSetContentConflict, RuleSetDigest,
    RuleSetId, RuleSetOutcome, RuleSource, SourceId,
};
use crate::platform::database::{begin_read_transaction, begin_write_transaction, NativeDatabase};
use rusqlite::{params, OptionalExtension, Row};
use std::sync::Arc;

pub(crate) struct SqliteRuleSetRepository {
    database: Arc<NativeDatabase>,
}

impl SqliteRuleSetRepository {
    pub(crate) fn new(database: Arc<NativeDatabase>) -> Self {
        Self { database }
    }
}

/// Rebuilds a rule from a row, refusing any value this build cannot name.
fn read_rule(row: &Row<'_>) -> Result<AuthorizationRule, rusqlite::Error> {
    let convert = |error: crate::contexts::permissions::domain::rules::RuleIdentityError| {
        rusqlite::Error::InvalidColumnName(error.code().to_string())
    };
    Ok(AuthorizationRule {
        source: RuleSource::parse(&row.get::<_, String>(0)?).map_err(convert)?,
        source_id: SourceId::parse(&row.get::<_, String>(1)?).map_err(convert)?,
        rule_id: RuleId::parse(&row.get::<_, String>(2)?).map_err(convert)?,
        scope: RuleScope::parse(&row.get::<_, String>(3)?, &row.get::<_, String>(4)?)
            .map_err(convert)?,
        operation: OperationName::parse(&row.get::<_, String>(5)?).map_err(convert)?,
        matcher: Matcher::parse(&row.get::<_, String>(6)?).map_err(convert)?,
        effect: RuleEffect::parse(&row.get::<_, String>(7)?).map_err(convert)?,
        allowed_scopes: AllowedScopes::parse(&row.get::<_, String>(8)?).map_err(convert)?,
        priority: row.get(9)?,
        expires_at: row.get(10)?,
        provenance: RuleProvenance::parse(&row.get::<_, String>(11)?).map_err(convert)?,
    })
}

const RULE_COLUMNS: &str = "source_kind, source_id, rule_id, scope_kind, scope_key, operation, \
                            matcher, effect, allowed_scopes, priority, expires_at, provenance";

impl RuleSetRepository for SqliteRuleSetRepository {
    fn record(
        &self,
        rule_set_id: &RuleSetId,
        digest: &RuleSetDigest,
        rules: &[AuthorizationRule],
        created_at: &str,
    ) -> Result<RuleSetOutcome, String> {
        let connection = self
            .database
            .connection()
            .map_err(|error| error.to_string())?;
        let transaction =
            begin_write_transaction(&connection).map_err(|error| error.to_string())?;

        // Read inside the transaction, so two publishers racing cannot both see "unrecorded".
        let stored: Option<(String, String)> = transaction
            .query_row(
                "SELECT content_digest, created_at FROM permission_rule_sets \
                 WHERE rule_set_id = ?1",
                params![rule_set_id.as_str()],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()
            .map_err(|error| error.to_string())?;

        if let Some((stored_digest, stored_at)) = stored {
            let stored_digest =
                RuleSetDigest::parse(&stored_digest).map_err(|error| error.code().to_string())?;
            return Ok(if &stored_digest == digest {
                RuleSetOutcome::AlreadyRecorded {
                    existing: rule_set_id.clone(),
                }
            } else {
                RuleSetOutcome::Conflict(RuleSetContentConflict {
                    stored_digest,
                    offered_digest: digest.clone(),
                    stored_at,
                })
            });
        }

        // Same contents under another id: dedup rather than store a second copy, and hand back the
        // id that is actually in storage.
        let existing: Option<String> = transaction
            .query_row(
                "SELECT rule_set_id FROM permission_rule_sets WHERE content_digest = ?1",
                params![digest.as_str()],
                |row| row.get(0),
            )
            .optional()
            .map_err(|error| error.to_string())?;
        if let Some(existing) = existing {
            return Ok(RuleSetOutcome::AlreadyRecorded {
                existing: RuleSetId::parse(&existing).map_err(|error| error.code().to_string())?,
            });
        }

        transaction
            .execute(
                "INSERT INTO permission_rule_sets \
                     (rule_set_id, content_digest, rule_count, created_at) \
                 VALUES (?1, ?2, ?3, ?4)",
                params![
                    rule_set_id.as_str(),
                    digest.as_str(),
                    i64::try_from(rules.len()).unwrap_or(i64::MAX),
                    created_at,
                ],
            )
            .map_err(|error| error.to_string())?;

        for rule in rules {
            transaction
                .execute(
                    "INSERT INTO permission_authorization_rules \
                         (rule_set_id, source_kind, source_id, rule_id, scope_kind, scope_key, \
                          operation, matcher, effect, allowed_scopes, priority, specificity, \
                          expires_at, provenance) \
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
                    params![
                        rule_set_id.as_str(),
                        rule.source.as_str(),
                        rule.source_id.as_str(),
                        rule.rule_id.as_str(),
                        rule.scope.kind().as_str(),
                        rule.scope.key(),
                        rule.operation.as_str(),
                        rule.matcher.as_str(),
                        rule.effect.as_str(),
                        rule.allowed_scopes.as_str(),
                        rule.priority,
                        // Derived here, never accepted from a caller.
                        rule.specificity(),
                        rule.expires_at,
                        rule.provenance.as_str(),
                    ],
                )
                .map_err(|error| error.to_string())?;
        }
        transaction.commit().map_err(|error| error.to_string())?;
        Ok(RuleSetOutcome::Recorded)
    }

    fn rule_set(&self, rule_set_id: &RuleSetId) -> Result<Option<RuleSet>, String> {
        let connection = self
            .database
            .connection()
            .map_err(|error| error.to_string())?;
        // One snapshot for the header and its rules: a set read across a concurrent publish could
        // otherwise report a digest that does not describe the rows beside it.
        let transaction = begin_read_transaction(&connection).map_err(|error| error.to_string())?;

        let header: Option<(String, String)> = transaction
            .query_row(
                "SELECT content_digest, created_at FROM permission_rule_sets \
                 WHERE rule_set_id = ?1",
                params![rule_set_id.as_str()],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()
            .map_err(|error| error.to_string())?;
        let Some((digest, created_at)) = header else {
            return Ok(None);
        };

        let mut statement = transaction
            .prepare(&format!(
                "SELECT {RULE_COLUMNS} FROM permission_authorization_rules \
                 WHERE rule_set_id = ?1 ORDER BY source_kind, source_id, rule_id"
            ))
            .map_err(|error| error.to_string())?;
        let rules = statement
            .query_map(params![rule_set_id.as_str()], read_rule)
            .map_err(|error| error.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| error.to_string())?;
        drop(statement);

        Ok(Some(RuleSet {
            rule_set_id: rule_set_id.clone(),
            content_digest: RuleSetDigest::parse(&digest)
                .map_err(|error| error.code().to_string())?,
            rules,
            created_at,
        }))
    }

    fn by_digest(&self, digest: &RuleSetDigest) -> Result<Option<RuleSetId>, String> {
        let connection = self
            .database
            .connection()
            .map_err(|error| error.to_string())?;
        connection
            .query_row(
                "SELECT rule_set_id FROM permission_rule_sets WHERE content_digest = ?1",
                params![digest.as_str()],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map_err(|error| error.to_string())?
            .map(|value| RuleSetId::parse(&value).map_err(|error| error.code().to_string()))
            .transpose()
    }
}

pub(crate) struct SqliteActiveRuleSetRepository {
    database: Arc<NativeDatabase>,
}

impl SqliteActiveRuleSetRepository {
    pub(crate) fn new(database: Arc<NativeDatabase>) -> Self {
        Self { database }
    }
}

fn active_error(error: rusqlite::Error) -> ActiveRuleSetError {
    if error.to_string().contains("FOREIGN KEY") {
        ActiveRuleSetError::UnknownRuleSet
    } else {
        ActiveRuleSetError::Storage(error.to_string())
    }
}

fn read_active(row: (Option<String>, i64, String)) -> Result<ActiveRuleSet, ActiveRuleSetError> {
    let (rule_set_id, revision, updated_at) = row;
    Ok(ActiveRuleSet {
        rule_set_id: rule_set_id
            .map(|value| RuleSetId::parse(&value))
            .transpose()
            .map_err(|error| ActiveRuleSetError::Storage(error.code().to_string()))?,
        revision,
        updated_at,
    })
}

impl ActiveRuleSetRepository for SqliteActiveRuleSetRepository {
    fn active(&self) -> Result<ActiveRuleSet, ActiveRuleSetError> {
        let connection = self
            .database
            .connection()
            .map_err(|error| ActiveRuleSetError::Storage(error.to_string()))?;
        let row = connection
            .query_row(
                "SELECT rule_set_id, revision, updated_at FROM permission_active_rule_set \
                 WHERE id = 1",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .map_err(active_error)?;
        read_active(row)
    }

    fn activate(
        &self,
        rule_set_id: &RuleSetId,
        expected_revision: i64,
        updated_at: &str,
    ) -> Result<ActiveRuleSet, ActiveRuleSetError> {
        let connection = self
            .database
            .connection()
            .map_err(|error| ActiveRuleSetError::Storage(error.to_string()))?;
        let transaction = begin_write_transaction(&connection)
            .map_err(|error| ActiveRuleSetError::Storage(error.to_string()))?;

        // Read inside the transaction, so two activations racing cannot both see the same
        // revision and both believe they won.
        let current: i64 = transaction
            .query_row(
                "SELECT revision FROM permission_active_rule_set WHERE id = 1",
                [],
                |row| row.get(0),
            )
            .map_err(active_error)?;
        if current != expected_revision {
            return Err(ActiveRuleSetError::StaleRevision {
                expected: expected_revision,
                actual: current,
            });
        }

        let revision = current
            .checked_add(1)
            .ok_or_else(|| ActiveRuleSetError::Storage("revision_exhausted".to_string()))?;
        transaction
            .execute(
                "UPDATE permission_active_rule_set \
                 SET rule_set_id = ?1, revision = ?2, updated_at = ?3 WHERE id = 1",
                params![rule_set_id.as_str(), revision, updated_at],
            )
            .map_err(active_error)?;
        transaction
            .commit()
            .map_err(|error| ActiveRuleSetError::Storage(error.to_string()))?;

        Ok(ActiveRuleSet {
            rule_set_id: Some(rule_set_id.clone()),
            revision,
            updated_at: updated_at.to_string(),
        })
    }
}
