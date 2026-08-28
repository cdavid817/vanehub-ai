//! Local-media use cases.

mod canary;
mod cleanup;
mod execute;
mod ocr;
mod operation_store;
pub(crate) mod ports;
mod probe;
mod service;
mod stt;
#[cfg(test)]
mod test_doubles;
#[cfg(test)]
#[path = "service_tests.rs"]
mod tests;
mod tts;
pub(crate) mod worker_contract;

pub(crate) use service::{
    LocalMediaApplicationService, LocalMediaDependencies, PreparedLocalMediaOperation,
};
