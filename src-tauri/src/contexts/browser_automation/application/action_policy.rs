use super::{BrowserAction, BrowserOperationRequest, BrowserOwnership};
use crate::contexts::web_research::api::{
    GuardedUrlPolicy, GuardedUrlPolicyError, PublicUrlResolution, UrlResolverPort,
};
use serde_json::Value;
use sha2::{Digest, Sha256};
use url::Url;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BrowserRiskClass {
    ReadOnly,
    Effectful,
}

impl BrowserRiskClass {
    #[allow(dead_code)]
    pub(crate) const fn requires_unified_permission(self) -> bool {
        matches!(self, Self::Effectful)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct BrowserActionApprovalWitness {
    pub(crate) ownership: BrowserOwnership,
    pub(crate) action: BrowserAction,
    pub(crate) risk: BrowserRiskClass,
    pub(crate) canonical_origin: String,
    pub(crate) safe_target_summary: String,
    pub(crate) input_hash: String,
    pub(crate) navigation_resolution: Option<PublicUrlResolution>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BrowserActionPolicyError {
    InvalidInput,
    UnsafeOrigin,
    StaleApproval,
}

pub(crate) struct BrowserActionPolicy;

impl BrowserActionPolicy {
    pub(crate) fn prepare(
        request: &BrowserOperationRequest,
        current_url: Option<&str>,
        resolver: &dyn UrlResolverPort,
    ) -> Result<BrowserActionApprovalWitness, BrowserActionPolicyError> {
        let input_hash = input_hash(&request.input)?;
        let (canonical_origin, navigation_resolution) = if request.action == BrowserAction::Navigate
        {
            let target = request
                .input
                .get("url")
                .and_then(Value::as_str)
                .ok_or(BrowserActionPolicyError::InvalidInput)?;
            let resolution =
                GuardedUrlPolicy::resolve_public(target, resolver).map_err(map_url_policy_error)?;
            (
                canonical_origin(&resolution.normalized_url)?,
                Some(resolution),
            )
        } else {
            (
                canonical_origin(current_url.ok_or(BrowserActionPolicyError::UnsafeOrigin)?)?,
                None,
            )
        };
        Ok(BrowserActionApprovalWitness {
            ownership: request.ownership.clone(),
            action: request.action,
            risk: risk_class(request.action),
            canonical_origin,
            safe_target_summary: safe_target_summary(request.action, &request.input, &input_hash),
            input_hash,
            navigation_resolution,
        })
    }

    pub(crate) fn revalidate(
        witness: &BrowserActionApprovalWitness,
        request: &BrowserOperationRequest,
        current_url: Option<&str>,
        resolver: &dyn UrlResolverPort,
    ) -> Result<(), BrowserActionPolicyError> {
        let current = Self::prepare(request, current_url, resolver)?;
        if &current != witness {
            return Err(BrowserActionPolicyError::StaleApproval);
        }
        if let Some(resolution) = &witness.navigation_resolution {
            GuardedUrlPolicy::revalidate_resolution(resolution, resolver)
                .map_err(map_url_policy_error)?;
        }
        Ok(())
    }
}

const fn risk_class(action: BrowserAction) -> BrowserRiskClass {
    match action {
        BrowserAction::Navigate
        | BrowserAction::GoBack
        | BrowserAction::GoForward
        | BrowserAction::Inspect
        | BrowserAction::Extract => BrowserRiskClass::ReadOnly,
        BrowserAction::Click
        | BrowserAction::Fill
        | BrowserAction::Screenshot
        | BrowserAction::Evaluate => BrowserRiskClass::Effectful,
    }
}

fn canonical_origin(raw_url: &str) -> Result<String, BrowserActionPolicyError> {
    let url = Url::parse(raw_url).map_err(|_| BrowserActionPolicyError::UnsafeOrigin)?;
    if !matches!(url.scheme(), "http" | "https") || url.host().is_none() {
        return Err(BrowserActionPolicyError::UnsafeOrigin);
    }
    Ok(url.origin().ascii_serialization())
}

fn input_hash(input: &Value) -> Result<String, BrowserActionPolicyError> {
    let encoded = serde_json::to_vec(input).map_err(|_| BrowserActionPolicyError::InvalidInput)?;
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let digest = Sha256::digest(encoded);
    let mut result = String::with_capacity(7 + digest.len() * 2);
    result.push_str("sha256:");
    for byte in digest {
        result.push(char::from(HEX[usize::from(byte >> 4)]));
        result.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    Ok(result)
}

fn safe_target_summary(action: BrowserAction, input: &Value, input_hash: &str) -> String {
    match action {
        BrowserAction::Navigate => input
            .get("url")
            .and_then(Value::as_str)
            .and_then(|value| Url::parse(value).ok())
            .and_then(|url| url.host_str().map(str::to_owned))
            .map(|host| format!("origin:{host}"))
            .unwrap_or_else(|| "origin:invalid".to_owned()),
        BrowserAction::Click | BrowserAction::Fill | BrowserAction::Extract => {
            format!("selector:{}", &input_hash[7..23])
        }
        BrowserAction::Evaluate => format!("script:{}", &input_hash[7..23]),
        BrowserAction::Screenshot => format!("capture:{}", &input_hash[7..23]),
        _ => "active-page".to_owned(),
    }
}

fn map_url_policy_error(_error: GuardedUrlPolicyError) -> BrowserActionPolicyError {
    BrowserActionPolicyError::UnsafeOrigin
}

#[cfg(test)]
#[path = "action_policy_tests.rs"]
mod tests;
