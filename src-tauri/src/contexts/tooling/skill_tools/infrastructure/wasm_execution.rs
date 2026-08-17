use super::wasm_host_bridge::ModuleStoreState;
use crate::contexts::tooling::skill_tools::application::SkillToolModuleOutcome;
use crate::contexts::tooling::skill_tools::domain::SkillToolLimits;
use serde_json::Value;
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use wasmtime::{Engine, Instance, Store, TypedFunc};

enum ModuleEntry {
    Json(TypedFunc<(i32, i32), i64>),
    Unit(TypedFunc<(), ()>),
}

enum RawResult {
    Json(i64),
    Unit,
}

pub(crate) fn execute_module_entry(
    engine: &Engine,
    store: &mut Store<ModuleStoreState>,
    instance: &Instance,
    export: &str,
    input: &[u8],
    limits: &SkillToolLimits,
    cancelled: &AtomicBool,
) -> SkillToolModuleOutcome {
    let entry =
        if let Ok(function) = instance.get_typed_func::<(i32, i32), i64>(&mut *store, export) {
            let Some(memory) = instance.get_memory(&mut *store, "memory") else {
                return trapped("module-memory-export");
            };
            if memory.write(&mut *store, 0, input).is_err() {
                return limit("input-memory");
            }
            ModuleEntry::Json(function)
        } else if let Ok(function) = instance.get_typed_func::<(), ()>(&mut *store, export) {
            ModuleEntry::Unit(function)
        } else {
            return trapped("module-entry-signature");
        };
    let reason = Arc::new(AtomicU8::new(0));
    let call = std::thread::scope(|scope| {
        let (done_tx, done_rx) = std::sync::mpsc::channel();
        let engine = engine.clone();
        let timer_reason = Arc::clone(&reason);
        scope.spawn(move || {
            let deadline = Instant::now() + Duration::from_millis(limits.wall_time_milliseconds);
            loop {
                if done_rx.recv_timeout(Duration::from_millis(2)).is_ok() {
                    return;
                }
                if cancelled.load(Ordering::Acquire) {
                    timer_reason.store(1, Ordering::Release);
                    engine.increment_epoch();
                    return;
                }
                if Instant::now() >= deadline {
                    timer_reason.store(2, Ordering::Release);
                    engine.increment_epoch();
                    return;
                }
            }
        });
        let result = match entry {
            ModuleEntry::Json(function) => function
                .call(
                    &mut *store,
                    (0, i32::try_from(input.len()).unwrap_or(i32::MAX)),
                )
                .map(RawResult::Json),
            ModuleEntry::Unit(function) => function.call(&mut *store, ()).map(|()| RawResult::Unit),
        };
        let _ = done_tx.send(());
        result
    });
    match (call, reason.load(Ordering::Acquire)) {
        (_, 1) => SkillToolModuleOutcome::Cancelled,
        (_, 2) => limit("wall-time"),
        (Err(error), _)
            if error.downcast_ref::<wasmtime::Trap>() == Some(&wasmtime::Trap::OutOfFuel) =>
        {
            limit("fuel")
        }
        (Err(error), _) => trapped(&error.to_string()),
        (Ok(RawResult::Unit), _) => SkillToolModuleOutcome::Completed(Value::Null),
        (Ok(RawResult::Json(packed)), _) => decode_output(store, instance, packed, limits),
    }
}

fn decode_output(
    store: &mut Store<ModuleStoreState>,
    instance: &Instance,
    packed: i64,
    limits: &SkillToolLimits,
) -> SkillToolModuleOutcome {
    let pointer = usize::try_from((packed as u64 >> 32) as u32).unwrap_or(usize::MAX);
    let length = usize::try_from(packed as u32).unwrap_or(usize::MAX);
    if length as u64 > limits.output_bytes {
        return limit("output-bytes");
    }
    let Some(memory) = instance.get_memory(&mut *store, "memory") else {
        return trapped("module-memory-export");
    };
    let mut bytes = vec![0; length];
    if memory.read(&*store, pointer, &mut bytes).is_err() {
        return trapped("module-output-range");
    }
    match serde_json::from_slice(&bytes) {
        Ok(value) => SkillToolModuleOutcome::Completed(value),
        Err(_) => trapped("module-invalid-output"),
    }
}

fn limit(name: &str) -> SkillToolModuleOutcome {
    SkillToolModuleOutcome::LimitBreached {
        limit: name.to_string(),
    }
}

fn trapped(detail: &str) -> SkillToolModuleOutcome {
    SkillToolModuleOutcome::Trapped {
        detail: detail.chars().take(256).collect(),
    }
}
