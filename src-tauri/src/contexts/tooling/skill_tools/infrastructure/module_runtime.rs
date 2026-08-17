#[cfg(feature = "skill-tool-module-runtime")]
use super::wasm_execution::execute_module_entry;
#[cfg(feature = "skill-tool-module-runtime")]
use super::wasm_host_bridge::{install_host_call, ModuleStoreState};
use crate::contexts::tooling::skill_tools::application::{
    SkillToolApplicationError, SkillToolCompiledArtifactPort, SkillToolModuleHostCallPort,
    SkillToolModuleOutcome, SkillToolModuleRuntime, SkillToolPackageRef, SkillToolPackageSource,
};
#[cfg(feature = "skill-tool-module-runtime")]
use crate::contexts::tooling::skill_tools::domain::{content_hash_of, inspect_module};
use crate::contexts::tooling::skill_tools::domain::{
    ModuleImplementation, SkillToolKey, SkillToolLimits,
};
use serde_json::Value;
use std::collections::BTreeMap;
use std::collections::HashSet;
use std::sync::atomic::AtomicBool;
#[cfg(feature = "skill-tool-module-runtime")]
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::sync::Mutex;

pub(crate) struct NativeSkillToolModuleRuntime<'a> {
    source: &'a dyn SkillToolPackageSource,
    active_by_skill: Mutex<BTreeMap<String, u32>>,
    #[cfg(feature = "skill-tool-module-runtime")]
    engine: wasmtime::Engine,
    #[cfg(feature = "skill-tool-module-runtime")]
    compiled: Mutex<BTreeMap<String, wasmtime::Module>>,
}

impl<'a> NativeSkillToolModuleRuntime<'a> {
    pub(crate) fn new(
        source: &'a dyn SkillToolPackageSource,
    ) -> Result<Self, SkillToolApplicationError> {
        #[cfg(feature = "skill-tool-module-runtime")]
        let engine = {
            let mut config = wasmtime::Config::new();
            config.consume_fuel(true).epoch_interruption(true);
            wasmtime::Engine::new(&config)
                .map_err(|_| SkillToolApplicationError::ModuleRuntimeUnavailable)?
        };
        Ok(Self {
            source,
            active_by_skill: Mutex::new(BTreeMap::new()),
            #[cfg(feature = "skill-tool-module-runtime")]
            engine,
            #[cfg(feature = "skill-tool-module-runtime")]
            compiled: Mutex::new(BTreeMap::new()),
        })
    }
}

impl SkillToolModuleRuntime for NativeSkillToolModuleRuntime<'_> {
    fn is_available(&self) -> bool {
        cfg!(feature = "skill-tool-module-runtime")
    }

    fn invoke(
        &self,
        package: &SkillToolPackageRef,
        key: &SkillToolKey,
        module: &ModuleImplementation,
        export: &str,
        _input: &Value,
        limits: &SkillToolLimits,
        cancelled: &AtomicBool,
        host_calls: Arc<dyn SkillToolModuleHostCallPort>,
    ) -> Result<SkillToolModuleOutcome, SkillToolApplicationError> {
        #[cfg(not(feature = "skill-tool-module-runtime"))]
        {
            let _ = (package, key, module, export, limits, cancelled, host_calls);
            Err(SkillToolApplicationError::ModuleRuntimeUnavailable)
        }
        #[cfg(feature = "skill-tool-module-runtime")]
        {
            let _permit = self.enter_skill(key.owner.as_str(), limits.concurrency)?;
            self.invoke_native(
                package, key, module, export, _input, limits, cancelled, host_calls,
            )
        }
    }
}

impl SkillToolCompiledArtifactPort for NativeSkillToolModuleRuntime<'_> {
    fn retain_revisions(
        &self,
        revisions: &HashSet<crate::contexts::tooling::skill_tools::domain::SkillToolRevision>,
    ) {
        #[cfg(feature = "skill-tool-module-runtime")]
        if let Ok(mut compiled) = self.compiled.lock() {
            compiled.retain(|key, _| {
                revisions
                    .iter()
                    .any(|revision| key.starts_with(revision.as_str()))
            });
        }
        #[cfg(not(feature = "skill-tool-module-runtime"))]
        let _ = revisions;
    }
}

struct SkillConcurrencyPermit<'a> {
    active: &'a Mutex<BTreeMap<String, u32>>,
    owner: String,
}

impl Drop for SkillConcurrencyPermit<'_> {
    fn drop(&mut self) {
        if let Ok(mut active) = self.active.lock() {
            let count = active.entry(self.owner.clone()).or_default();
            *count = count.saturating_sub(1);
            if *count == 0 {
                active.remove(&self.owner);
            }
        }
    }
}

impl NativeSkillToolModuleRuntime<'_> {
    fn enter_skill(
        &self,
        owner: &str,
        maximum: u32,
    ) -> Result<SkillConcurrencyPermit<'_>, SkillToolApplicationError> {
        let mut active = self
            .active_by_skill
            .lock()
            .map_err(|_| SkillToolApplicationError::ResourceLimit("concurrency".to_string()))?;
        let count = active.entry(owner.to_string()).or_default();
        if *count >= maximum {
            return Err(SkillToolApplicationError::ResourceLimit(
                "concurrency".to_string(),
            ));
        }
        *count += 1;
        Ok(SkillConcurrencyPermit {
            active: &self.active_by_skill,
            owner: owner.to_string(),
        })
    }
}

#[cfg(feature = "skill-tool-module-runtime")]
impl NativeSkillToolModuleRuntime<'_> {
    #[allow(clippy::too_many_arguments)]
    fn invoke_native(
        &self,
        package: &SkillToolPackageRef,
        key: &SkillToolKey,
        implementation: &ModuleImplementation,
        export: &str,
        input: &Value,
        limits: &SkillToolLimits,
        cancelled: &AtomicBool,
        host_calls: Arc<dyn SkillToolModuleHostCallPort>,
    ) -> Result<SkillToolModuleOutcome, SkillToolApplicationError> {
        if cancelled.load(Ordering::Acquire) {
            return Ok(SkillToolModuleOutcome::Cancelled);
        }
        let input_bytes = input.to_string().into_bytes();
        if input_bytes.len() as u64 > limits.input_bytes {
            return Ok(limit_breached("input-bytes"));
        }
        let bytes = self
            .source
            .read_implementation(package, &implementation.path)?;
        if content_hash_of(&bytes) != implementation.content_hash {
            return Err(SkillToolApplicationError::IntegrityMismatch {
                path: implementation.path.clone(),
            });
        }
        inspect_module(&bytes, export, limits)?;
        let compiled = match self.compiled_module(key, &bytes) {
            Ok(compiled) => compiled,
            Err(error) => return Ok(trapped(error)),
        };
        let memory_limit = usize::try_from(limits.memory_bytes).unwrap_or(usize::MAX);
        let store_limits = wasmtime::StoreLimitsBuilder::new()
            .memory_size(memory_limit)
            .memories(1)
            .instances(1)
            .trap_on_grow_failure(true)
            .build();
        let maximum_payload_bytes =
            usize::try_from(limits.output_bytes.min(limits.input_bytes)).unwrap_or(usize::MAX);
        let mut store = wasmtime::Store::new(
            &self.engine,
            ModuleStoreState {
                limits: store_limits,
                host_calls,
                maximum_payload_bytes,
            },
        );
        store.limiter(|state| &mut state.limits);
        store
            .set_fuel(limits.fuel)
            .map_err(|_| SkillToolApplicationError::ModuleRuntimeUnavailable)?;
        store.set_epoch_deadline(1);
        store.epoch_deadline_trap();
        let mut linker = wasmtime::Linker::new(&self.engine);
        if let Err(error) = install_host_call(&mut linker) {
            return Ok(trapped(error));
        }
        let instance = match linker.instantiate(&mut store, &compiled) {
            Ok(instance) => instance,
            Err(error) => return Ok(trapped(error.to_string())),
        };
        Ok(execute_module_entry(
            &self.engine,
            &mut store,
            &instance,
            export,
            &input_bytes,
            limits,
            cancelled,
        ))
    }
}

#[cfg(feature = "skill-tool-module-runtime")]
const ENGINE_CONFIGURATION_WITNESS: &str = "wasmtime-47.0.3-core-cranelift-fuel-epoch-no-wasi-v1";

#[cfg(feature = "skill-tool-module-runtime")]
impl NativeSkillToolModuleRuntime<'_> {
    fn compiled_module(
        &self,
        key: &SkillToolKey,
        bytes: &[u8],
    ) -> Result<wasmtime::Module, String> {
        let cache_key = format!("{}:{ENGINE_CONFIGURATION_WITNESS}", key.revision.as_str());
        if let Some(module) = self
            .compiled
            .lock()
            .map_err(|_| "module-cache".to_string())?
            .get(&cache_key)
            .cloned()
        {
            return Ok(module);
        }
        let module =
            wasmtime::Module::new(&self.engine, bytes).map_err(|error| error.to_string())?;
        self.compiled
            .lock()
            .map_err(|_| "module-cache".to_string())?
            .insert(cache_key, module.clone());
        Ok(module)
    }
}

#[cfg(feature = "skill-tool-module-runtime")]
fn trapped(detail: String) -> SkillToolModuleOutcome {
    SkillToolModuleOutcome::Trapped {
        detail: detail.chars().take(256).collect(),
    }
}

#[cfg(feature = "skill-tool-module-runtime")]
fn limit_breached(limit: &str) -> SkillToolModuleOutcome {
    SkillToolModuleOutcome::LimitBreached {
        limit: limit.to_string(),
    }
}

#[cfg(test)]
#[path = "module_runtime_tests.rs"]
mod tests;
