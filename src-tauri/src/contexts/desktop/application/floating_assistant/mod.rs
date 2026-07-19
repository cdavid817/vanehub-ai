mod error;
mod ports;
mod service;

pub(crate) use error::FloatingAssistantApplicationError;
pub(crate) use ports::{FloatingAssistantRepository, FloatingAssistantWindowPort};
pub(crate) use service::FloatingAssistantApplicationService;

#[cfg(test)]
mod tests;
