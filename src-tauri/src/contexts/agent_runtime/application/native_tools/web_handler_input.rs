use super::{NativeToolErrorCode, NativeToolHandlerError};
use serde_json::{Map, Value};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use url::Url;

pub(super) fn canonical_web_url(raw: &str) -> Result<String, NativeToolHandlerError> {
    if raw.trim() != raw || raw.len() > 4096 {
        return Err(invalid_fetch_input());
    }
    let mut url = Url::parse(raw).map_err(|_| invalid_fetch_input())?;
    if !matches!(url.scheme(), "http" | "https")
        || !url.username().is_empty()
        || url.password().is_some()
        || !matches!(url.port_or_known_default(), Some(80 | 443))
        || url.host().is_none()
    {
        return Err(invalid_fetch_input());
    }
    url.set_fragment(None);
    Ok(url.to_string())
}

pub(super) fn required_string<'a>(
    object: &'a Map<String, Value>,
    name: &str,
    error: fn() -> NativeToolHandlerError,
) -> Result<&'a str, NativeToolHandlerError> {
    object.get(name).and_then(Value::as_str).ok_or_else(error)
}

pub(super) fn optional_string(
    object: &Map<String, Value>,
    name: &str,
    max_chars: usize,
    error: fn() -> NativeToolHandlerError,
) -> Result<(), NativeToolHandlerError> {
    if let Some(value) = object.get(name) {
        let value = value.as_str().ok_or_else(error)?;
        if value.is_empty() || value.chars().count() > max_chars {
            return Err(error());
        }
    }
    Ok(())
}

pub(super) fn optional_u64(
    object: &Map<String, Value>,
    name: &str,
    minimum: u64,
    maximum: u64,
    error: fn() -> NativeToolHandlerError,
) -> Result<(), NativeToolHandlerError> {
    if let Some(value) = object.get(name) {
        let value = value.as_u64().ok_or_else(error)?;
        if !(minimum..=maximum).contains(&value) {
            return Err(error());
        }
    }
    Ok(())
}

pub(super) fn reject_unknown_fields(
    object: &Map<String, Value>,
    allowed: &[&str],
) -> Result<(), NativeToolHandlerError> {
    let allowed = allowed.iter().copied().collect::<BTreeSet<_>>();
    if object.keys().any(|key| !allowed.contains(key.as_str())) {
        return Err(NativeToolHandlerError::new(
            NativeToolErrorCode::InvalidInput,
            "The Web tool input contains unsupported fields.",
        ));
    }
    Ok(())
}

pub(super) fn input_hash(
    input: &Value,
    error: fn() -> NativeToolHandlerError,
) -> Result<String, NativeToolHandlerError> {
    let bytes = serde_json::to_vec(input).map_err(|_| error())?;
    let digest = Sha256::digest(bytes);
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut encoded = String::with_capacity(digest.len() * 2);
    for byte in digest {
        encoded.push(char::from(HEX[usize::from(byte >> 4)]));
        encoded.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    Ok(format!("sha256:{encoded}"))
}

pub(super) fn invalid_search_input() -> NativeToolHandlerError {
    NativeToolHandlerError::new(
        NativeToolErrorCode::InvalidInput,
        "The Web search input is invalid.",
    )
}

pub(super) fn invalid_fetch_input() -> NativeToolHandlerError {
    NativeToolHandlerError::new(
        NativeToolErrorCode::InvalidInput,
        "The Web fetch input is invalid.",
    )
}
