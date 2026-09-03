use serde::Serialize;
use serde_json::Value;
use sha2::{Digest, Sha256};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CanonicalEncodingError;

pub(crate) fn canonical_json<T: Serialize>(value: &T) -> Result<String, CanonicalEncodingError> {
    let value = serde_json::to_value(value).map_err(|_| CanonicalEncodingError)?;
    let mut output = String::new();
    write_value(&value, &mut output)?;
    Ok(output)
}

pub(crate) fn canonical_hash<T: Serialize>(value: &T) -> Result<String, CanonicalEncodingError> {
    let canonical = canonical_json(value)?;
    Ok(sha256_bytes(canonical.as_bytes()))
}

pub(crate) fn sha256_bytes(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let hex: String = digest.iter().map(|byte| format!("{byte:02x}")).collect();
    format!("sha256:{hex}")
}

fn write_value(value: &Value, output: &mut String) -> Result<(), CanonicalEncodingError> {
    match value {
        Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) => {
            output.push_str(&serde_json::to_string(value).map_err(|_| CanonicalEncodingError)?)
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
                output.push_str(&serde_json::to_string(key).map_err(|_| CanonicalEncodingError)?);
                output.push(':');
                write_value(item, output)?;
            }
            output.push('}');
        }
    }
    Ok(())
}
