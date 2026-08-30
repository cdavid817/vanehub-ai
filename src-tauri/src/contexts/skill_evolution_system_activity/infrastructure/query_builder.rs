use rusqlite::types::Value;

use super::ActivityProjectionRepositoryError;
use crate::contexts::skill_evolution_system_activity::domain::*;

pub(super) fn build_query(
    query: &ActivityTimelineQuery,
    generation_id: &str,
    before_sequence: Option<u64>,
) -> Result<(String, Vec<Value>), ActivityProjectionRepositoryError> {
    let mut sql = String::from(
        "SELECT e.envelope_json,i.sequence,pt.detail_unavailable_reason
         FROM evolution_activity_items i
         JOIN evolution_activity_envelopes e ON e.event_id=i.event_id
         LEFT JOIN evolution_activity_purge_tombstones pt ON pt.event_id=e.event_id
         WHERE i.session_id=? AND i.generation_id=?",
    );
    let mut values = vec![
        Value::Text(query.session_id.clone()),
        Value::Text(generation_id.into()),
    ];
    if let Some(sequence) = before_sequence {
        sql.push_str(" AND i.sequence<?");
        values.push(Value::Integer(to_i64(sequence)?));
    }
    optional_i64(
        &mut sql,
        &mut values,
        "e.committed_at_ms>=?",
        query.committed_from_ms,
    );
    optional_i64(
        &mut sql,
        &mut values,
        "e.committed_at_ms<=?",
        query.committed_to_ms,
    );
    serialized_list(&mut sql, &mut values, "e.severity", &query.severities)?;
    text_list(
        &mut sql,
        &mut values,
        "e.source_domain",
        query
            .source_domains
            .iter()
            .map(|value| value.as_str().into()),
    );
    serialized_list(&mut sql, &mut values, "e.status", &query.statuses)?;
    identity_filter(&mut sql, &mut values, "skill", query.skill_id.as_deref())?;
    identity_filter(&mut sql, &mut values, "run", query.run_id.as_deref())?;
    let curator_codes = query.curator_states.iter().map(|state| state.event_code());
    serialized_iter(&mut sql, &mut values, "e.event_code", curator_codes)?;
    serialized_list(
        &mut sql,
        &mut values,
        "e.attention_kind",
        &query.attention_kinds,
    )?;
    if let Some(search) = &query.search {
        safe_search(&mut sql, &mut values, search)?;
    }
    sql.push_str(" ORDER BY i.sequence DESC LIMIT ?");
    values.push(Value::Integer(i64::from(query.page_size) + 1));
    Ok((sql, values))
}

fn optional_i64(sql: &mut String, values: &mut Vec<Value>, clause: &str, value: Option<i64>) {
    if let Some(value) = value {
        sql.push_str(" AND ");
        sql.push_str(clause);
        values.push(Value::Integer(value));
    }
}

fn identity_filter(
    sql: &mut String,
    values: &mut Vec<Value>,
    kind: &str,
    value: Option<&str>,
) -> Result<(), ActivityProjectionRepositoryError> {
    let Some(value) = value else {
        return Ok(());
    };
    sql.push_str(
        " AND EXISTS(SELECT 1 FROM evolution_activity_safe_identities si
         WHERE si.event_id=e.event_id AND si.identity_kind=? AND si.normalized_value=?)",
    );
    values.push(Value::Text(kind.into()));
    values.push(Value::Text(
        normalize_safe_identity_token(value).map_err(invalid)?,
    ));
    Ok(())
}

fn safe_search(
    sql: &mut String,
    values: &mut Vec<Value>,
    search: &ActivitySafeSearch,
) -> Result<(), ActivityProjectionRepositoryError> {
    if search.event_alias_codes.is_empty() && search.identity_tokens.is_empty() {
        sql.push_str(" AND 0=1");
        return Ok(());
    }
    sql.push_str(" AND (");
    if !search.event_alias_codes.is_empty() {
        serialized_list_raw(sql, values, "e.event_code", &search.event_alias_codes)?;
    }
    if !search.identity_tokens.is_empty() {
        if !search.event_alias_codes.is_empty() {
            sql.push_str(" OR ");
        }
        sql.push_str("EXISTS(SELECT 1 FROM evolution_activity_safe_identities si WHERE si.event_id=e.event_id AND ");
        text_list_raw(
            sql,
            values,
            "si.normalized_value",
            search
                .identity_tokens
                .iter()
                .map(|token| normalize_safe_identity_token(token).map_err(invalid))
                .collect::<Result<Vec<_>, _>>()?,
        );
        sql.push(')');
    }
    sql.push(')');
    Ok(())
}

fn serialized_list<T: serde::Serialize>(
    sql: &mut String,
    values: &mut Vec<Value>,
    column: &str,
    items: &[T],
) -> Result<(), ActivityProjectionRepositoryError> {
    if !items.is_empty() {
        sql.push_str(" AND ");
        serialized_list_raw(sql, values, column, items)?;
    }
    Ok(())
}

fn serialized_iter<T: serde::Serialize>(
    sql: &mut String,
    values: &mut Vec<Value>,
    column: &str,
    items: impl Iterator<Item = T>,
) -> Result<(), ActivityProjectionRepositoryError> {
    let items = items.map(serialized_text).collect::<Result<Vec<_>, _>>()?;
    text_list(sql, values, column, items);
    Ok(())
}

fn serialized_list_raw<T: serde::Serialize>(
    sql: &mut String,
    values: &mut Vec<Value>,
    column: &str,
    items: &[T],
) -> Result<(), ActivityProjectionRepositoryError> {
    let items = items
        .iter()
        .map(serialized_text)
        .collect::<Result<Vec<_>, _>>()?;
    text_list_raw(sql, values, column, items);
    Ok(())
}

fn text_list(
    sql: &mut String,
    values: &mut Vec<Value>,
    column: &str,
    items: impl IntoIterator<Item = String>,
) {
    let items = items.into_iter().collect::<Vec<_>>();
    if !items.is_empty() {
        sql.push_str(" AND ");
        text_list_raw(sql, values, column, items);
    }
}

fn text_list_raw(
    sql: &mut String,
    values: &mut Vec<Value>,
    column: &str,
    items: impl IntoIterator<Item = String>,
) {
    let items = items.into_iter().collect::<Vec<_>>();
    sql.push_str(column);
    sql.push_str(" IN (");
    sql.push_str(&vec!["?"; items.len()].join(","));
    sql.push(')');
    values.extend(items.into_iter().map(Value::Text));
}

fn serialized_text(
    value: impl serde::Serialize,
) -> Result<String, ActivityProjectionRepositoryError> {
    serde_json::to_value(value)
        .map_err(|_| ActivityProjectionRepositoryError::Storage)?
        .as_str()
        .map(str::to_owned)
        .ok_or(ActivityProjectionRepositoryError::Storage)
}

fn to_i64(value: u64) -> Result<i64, ActivityProjectionRepositoryError> {
    i64::try_from(value).map_err(|_| ActivityProjectionRepositoryError::InvalidInput)
}

fn invalid(_: ActivityEnvelopeError) -> ActivityProjectionRepositoryError {
    ActivityProjectionRepositoryError::InvalidInput
}
