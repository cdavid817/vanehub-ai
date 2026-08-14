use super::dto::{ConnectorView, PairingStartView, SessionBindingView, WeChatAuthorizationView};
use crate::contexts::communications::api::{
    ConnectorSummary, PairingStartResult, SessionBindingSnapshot, WeChatAuthorizationResult,
};

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

pub(super) fn pairing(result: PairingStartResult) -> PairingStartView {
    PairingStartView {
        connector: result.connector,
        session_id: result.session_id,
        code: result.code,
        expires_at: result.expires_at,
        replace_existing: result.replace_existing,
    }
}

pub(super) fn binding(snapshot: SessionBindingSnapshot) -> SessionBindingView {
    SessionBindingView {
        binding: snapshot.binding,
        pending_connector: snapshot.pending_connector,
    }
}
