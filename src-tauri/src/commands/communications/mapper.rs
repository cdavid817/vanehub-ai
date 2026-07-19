use super::dto::{ConnectorView, WeChatAuthorizationView};
use crate::contexts::communications::api::{ConnectorSummary, WeChatAuthorizationResult};

pub(super) fn connector(summary: ConnectorSummary) -> ConnectorView {
    ConnectorView {
        descriptor: summary.descriptor,
        config: summary.configuration,
        health: summary.health,
        has_credentials: summary.has_credentials,
    }
}

pub(super) fn authorization(result: WeChatAuthorizationResult) -> WeChatAuthorizationView {
    WeChatAuthorizationView {
        status: result.status,
        image_data_url: result.image_data_url,
        expires_at: result.expires_at,
        safe_error_code: result.safe_error_code,
    }
}
