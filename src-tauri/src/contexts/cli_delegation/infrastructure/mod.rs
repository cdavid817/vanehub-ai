#![allow(unused_imports)]
// Infrastructure is staged behind the same fail-closed delegation promotion gate.
#![allow(dead_code)]

mod apply_artifact;
mod apply_backend;
mod apply_preflight_backend;
mod change_set_apply_adapter;
mod changeset_artifact;
mod changeset_capture;
mod changeset_review;
mod child_network;
mod logging;
mod materialization_fs;
mod native_tool_adapter;
mod native_tool_execution;
mod native_tool_persistence;
mod native_tool_support;
mod passive_probe;
mod process_launcher;
mod repository_preflight;
mod workspace;

pub(crate) use change_set_apply_adapter::NativeChangeSetApplyAdapter;
pub(crate) use changeset_artifact::ArtifactChangeSetAdapter;
pub(crate) use changeset_capture::GitDelegationChangeSetCapture;
pub(crate) use changeset_review::ArtifactChangeSetReviewAdapter;
pub(crate) use child_network::SandboxChildNetworkAdapter;
pub(crate) use logging::{DelegationLogEvent, DelegationLogger};
pub(crate) use materialization_fs::SystemDelegationMaterializationAdapter;
pub(crate) use native_tool_adapter::ClaudeDelegationNativeToolAdapter;
pub(crate) use passive_probe::{PassiveDelegationProbe, PassiveDelegationProbeRunner};
pub(crate) use process_launcher::ManagedDelegationProcessLauncher;
pub(crate) use workspace::IndependentGitWorkspaceAdapter;
