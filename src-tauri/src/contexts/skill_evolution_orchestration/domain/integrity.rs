use serde::Serialize;
use serde_json::Value;
use sha2::{Digest, Sha256};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EvolutionIntegrityError {
    InvalidValue,
    Serialization,
    Conflict,
}

pub(crate) fn canonical_json<T: Serialize>(value: &T) -> Result<String, EvolutionIntegrityError> {
    let value = serde_json::to_value(value).map_err(|_| EvolutionIntegrityError::Serialization)?;
    let mut output = String::new();
    write_value(&value, &mut output)?;
    Ok(output)
}

pub(crate) fn canonical_hash<T: Serialize>(value: &T) -> Result<String, EvolutionIntegrityError> {
    Ok(sha256_bytes(canonical_json(value)?.as_bytes()))
}

pub(crate) fn orchestration_idempotency_key<T: Serialize>(
    subsystem: &str,
    operation: &str,
    source: &T,
) -> Result<String, EvolutionIntegrityError> {
    if !is_safe_identifier(subsystem, 64) || !is_safe_identifier(operation, 64) {
        return Err(EvolutionIntegrityError::InvalidValue);
    }
    canonical_hash(&(
        "skill-evolution-orchestration-v1",
        subsystem,
        operation,
        source,
    ))
}

pub(crate) fn next_optimistic_revision(
    expected: u64,
    current: u64,
) -> Result<u64, EvolutionIntegrityError> {
    if expected != current {
        return Err(EvolutionIntegrityError::Conflict);
    }
    current
        .checked_add(1)
        .ok_or(EvolutionIntegrityError::InvalidValue)
}

pub(crate) fn is_safe_identifier(value: &str, maximum_bytes: usize) -> bool {
    !value.is_empty()
        && value.len() <= maximum_bytes
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':'))
}

pub(crate) fn sha256_bytes(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let hex: String = digest.iter().map(|byte| format!("{byte:02x}")).collect();
    format!("sha256:{hex}")
}

fn write_value(value: &Value, output: &mut String) -> Result<(), EvolutionIntegrityError> {
    match value {
        Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) => {
            output.push_str(
                &serde_json::to_string(value)
                    .map_err(|_| EvolutionIntegrityError::Serialization)?,
            );
        }
        Value::Array(values) => {
            output.push('[');
            for (index, item) in values.iter().enumerate() {
                if index > 0 {
                    output.push(',');
                }
                write_value(item, output)?;
            }
            output.push(']');
        }
        Value::Object(values) => {
            output.push('{');
            let mut entries: Vec<_> = values.iter().collect();
            entries.sort_unstable_by(|left, right| left.0.cmp(right.0));
            for (index, (key, item)) in entries.into_iter().enumerate() {
                if index > 0 {
                    output.push(',');
                }
                output.push_str(
                    &serde_json::to_string(key)
                        .map_err(|_| EvolutionIntegrityError::Serialization)?,
                );
                output.push(':');
                write_value(item, output)?;
            }
            output.push('}');
        }
    }
    Ok(())
}
