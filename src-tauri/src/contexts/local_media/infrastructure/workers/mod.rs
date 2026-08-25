pub(super) mod environment;
mod process;
mod protocol;
mod supervisor;

#[cfg(feature = "desktop-e2e")]
pub(crate) use environment::WorkerEnvironmentOverlay;
#[cfg(test)]
pub(crate) use protocol::LOCAL_MEDIA_WORKER_PROTOCOL;
pub(crate) use supervisor::build_supervisor;
#[cfg(feature = "desktop-e2e")]
pub(crate) use supervisor::build_supervisor_with_overlay;
