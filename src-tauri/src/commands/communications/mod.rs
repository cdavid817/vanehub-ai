pub(crate) mod begin_wechat_authorization;
pub(crate) mod cancel_wechat_authorization;
pub(crate) mod clear_im_connector;
mod dto;
pub(crate) mod get_im_routing;
pub(crate) mod list_im_connectors;
mod mapper;
pub(crate) mod poll_wechat_authorization;
pub(crate) mod reset_im_bindings;
pub(crate) mod restart_im_connector;
pub(crate) mod save_im_connector;
pub(crate) mod save_im_routing;
pub(crate) mod set_im_connector_enabled;
pub(crate) mod test_im_connector;

#[cfg(test)]
mod tests;
