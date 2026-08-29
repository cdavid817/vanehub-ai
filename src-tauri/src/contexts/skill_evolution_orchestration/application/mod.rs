mod automatic_application;
mod budget_meter;
mod continuation_policy;
mod draft_pipeline;
mod idle_gate;
mod idle_wakeup;
mod recovery_coordinator;
mod recovery_registry;
mod scheduler_concurrency;
mod shutdown_coordinator;
mod stage_engine;
mod trigger_ingress;
mod trigger_projector;

pub(crate) use automatic_application::*;
pub(crate) use budget_meter::*;
pub(crate) use continuation_policy::*;
pub(crate) use draft_pipeline::*;
pub(crate) use idle_gate::*;
pub(crate) use idle_wakeup::*;
pub(crate) use recovery_coordinator::*;
pub(crate) use recovery_registry::*;
pub(crate) use scheduler_concurrency::*;
pub(crate) use shutdown_coordinator::*;
pub(crate) use stage_engine::*;
pub(crate) use trigger_ingress::*;
pub(crate) use trigger_projector::*;

#[cfg(test)]
mod automatic_application_tests;
#[cfg(test)]
mod budget_meter_tests;
#[cfg(test)]
mod continuation_policy_tests;
#[cfg(test)]
mod draft_pipeline_tests;
#[cfg(test)]
mod idle_gate_tests;
#[cfg(test)]
mod idle_wakeup_tests;
#[cfg(test)]
mod recovery_coordinator_tests;
#[cfg(test)]
mod recovery_registry_tests;
#[cfg(test)]
mod scheduler_concurrency_tests;
#[cfg(test)]
mod shutdown_coordinator_tests;
#[cfg(test)]
mod stage_engine_tests;
#[cfg(test)]
mod trigger_ingress_tests;
#[cfg(test)]
mod trigger_projector_tests;
