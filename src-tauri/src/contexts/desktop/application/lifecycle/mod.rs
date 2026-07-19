mod error;
mod ports;
mod service;

pub(crate) use error::DesktopLifecycleApplicationError;
pub(crate) use ports::{DesktopLifecyclePort, DesktopShutdownPort};
pub(crate) use service::DesktopLifecycleApplicationService;

#[cfg(test)]
mod tests;
