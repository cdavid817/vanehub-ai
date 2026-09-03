pub(crate) mod begin_im_pairing;
pub(crate) mod begin_wechat_authorization;
pub(crate) mod cancel_im_pairing;
pub(crate) mod cancel_wechat_authorization;
pub(crate) mod clear_im_connector;
mod dto;
#[cfg(feature = "desktop-e2e")]
pub(crate) mod fixture_feishu_im;
pub(crate) mod get_im_routing;
pub(crate) mod get_im_session_binding;
pub(crate) mod list_im_connectors;
mod mapper;
pub(crate) mod poll_wechat_authorization;
pub(crate) mod remove_im_session_binding;
pub(crate) mod reset_im_bindings;
pub(crate) mod restart_im_connector;
pub(crate) mod save_im_connector;
pub(crate) mod save_im_routing;
pub(crate) mod set_im_binding_paused;
pub(crate) mod set_im_completion_notifications;
pub(crate) mod set_im_connector_enabled;
pub(crate) mod set_im_session_access;
pub(crate) mod test_im_connector;

#[cfg(test)]
mod tests;

#[cfg(test)]
#[path = "fixture_feishu_im_boundary_tests.rs"]
mod fixture_feishu_im_boundary_tests;
