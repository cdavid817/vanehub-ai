use super::*;
use crate::contexts::tooling::skill_tools::application::{
    SkillToolDispatchOutcome, SkillToolFileEntry,
};
#[cfg(feature = "skill-tool-module-runtime")]
use crate::contexts::tooling::skill_tools::domain::{
    parse_manifest_bytes, SkillToolRevision, DEFAULT_MANIFEST_LIMITS, DEFAULT_SKILL_TOOL_LIMITS,
};
#[cfg(feature = "skill-tool-module-runtime")]
use std::sync::atomic::{AtomicU32, Ordering};

struct Source {
    bytes: Vec<u8>,
}

#[cfg(feature = "skill-tool-module-runtime")]
struct MutableSource(Mutex<Vec<u8>>);

#[cfg(feature = "skill-tool-module-runtime")]
impl SkillToolPackageSource for MutableSource {
    fn read_manifest(
        &self,
        _package: &SkillToolPackageRef,
    ) -> Result<Option<Vec<u8>>, SkillToolApplicationError> {
        Ok(None)
    }

    fn list_tool_files(
        &self,
        _package: &SkillToolPackageRef,
    ) -> Result<Vec<SkillToolFileEntry>, SkillToolApplicationError> {
        Ok(Vec::new())
    }

    fn read_implementation(
        &self,
        _package: &SkillToolPackageRef,
        _relative_path: &str,
    ) -> Result<Vec<u8>, SkillToolApplicationError> {
        Ok(self.0.lock().expect("bytes").clone())
    }
}

struct NoHostCalls;

impl SkillToolModuleHostCallPort for NoHostCalls {
    fn call(
        &self,
        _request: &Value,
    ) -> Result<SkillToolDispatchOutcome, SkillToolApplicationError> {
        Ok(SkillToolDispatchOutcome::Denied {
            reason: "not-used".to_string(),
        })
    }
}

#[cfg(feature = "skill-tool-module-runtime")]
struct RecordingHostCalls(AtomicU32);

#[cfg(feature = "skill-tool-module-runtime")]
impl SkillToolModuleHostCallPort for RecordingHostCalls {
    fn call(&self, request: &Value) -> Result<SkillToolDispatchOutcome, SkillToolApplicationError> {
        assert_eq!(request["capability"], "tool:read_file");
        assert_eq!(request["arguments"]["path"], "src/lib.rs");
        self.0.fetch_add(1, Ordering::AcqRel);
        Ok(SkillToolDispatchOutcome::Completed(
            serde_json::json!({"ok": true}),
        ))
    }
}

impl SkillToolPackageSource for Source {
    fn read_manifest(
        &self,
        _package: &SkillToolPackageRef,
    ) -> Result<Option<Vec<u8>>, SkillToolApplicationError> {
        Ok(None)
    }

    fn list_tool_files(
        &self,
        _package: &SkillToolPackageRef,
    ) -> Result<Vec<SkillToolFileEntry>, SkillToolApplicationError> {
        Ok(Vec::new())
    }

    fn read_implementation(
        &self,
        _package: &SkillToolPackageRef,
        _relative_path: &str,
    ) -> Result<Vec<u8>, SkillToolApplicationError> {
        Ok(self.bytes.clone())
    }
}

#[test]
fn availability_exactly_tracks_the_optional_runtime_feature() {
    let source = Source { bytes: Vec::new() };
    let runtime = NativeSkillToolModuleRuntime::new(&source).expect("runtime construction");
    assert_eq!(
        runtime.is_available(),
        cfg!(feature = "skill-tool-module-runtime")
    );
}

#[test]
fn per_skill_concurrency_is_bounded_and_permits_release() {
    let source = Source { bytes: Vec::new() };
    let runtime = NativeSkillToolModuleRuntime::new(&source).expect("runtime");
    let first = runtime.enter_skill("review", 1).expect("first permit");
    assert!(matches!(
        runtime.enter_skill("review", 1),
        Err(SkillToolApplicationError::ResourceLimit(_))
    ));
    assert!(runtime.enter_skill("other", 1).is_ok());
    drop(first);
    assert!(runtime.enter_skill("review", 1).is_ok());
}

#[test]
fn concurrent_pressure_admits_only_the_per_skill_ceiling() {
    let source = Source { bytes: Vec::new() };
    let runtime = NativeSkillToolModuleRuntime::new(&source).expect("runtime");
    let barrier = Arc::new(std::sync::Barrier::new(9));
    let admitted = std::thread::scope(|scope| {
        let workers = (0..8)
            .map(|_| {
                let barrier = barrier.clone();
                let runtime = &runtime;
                scope.spawn(move || {
                    let permit = runtime.enter_skill("review", 2).ok();
                    barrier.wait();
                    permit.is_some()
                })
            })
            .collect::<Vec<_>>();
        barrier.wait();
        workers
            .into_iter()
            .map(|worker| worker.join().expect("worker"))
            .filter(|admitted| *admitted)
            .count()
    });
    assert_eq!(admitted, 2);
}

#[cfg(feature = "skill-tool-module-runtime")]
#[test]
fn native_runtime_executes_core_wasm_without_inherited_host_imports() {
    let bytes = empty_run_module();
    let source = Source {
        bytes: bytes.clone(),
    };
    let runtime = NativeSkillToolModuleRuntime::new(&source).expect("runtime");
    let (package, key) = fixture_identity();
    let implementation = ModuleImplementation {
        path: "scripts/modules/runtime.wasm".to_string(),
        export: "run".to_string(),
        content_hash: content_hash_of(&bytes),
    };
    assert_eq!(
        runtime
            .invoke(
                &package,
                &key,
                &implementation,
                "run",
                &Value::Null,
                &DEFAULT_SKILL_TOOL_LIMITS,
                &AtomicBool::new(false),
                Arc::new(NoHostCalls),
            )
            .expect("invoke"),
        SkillToolModuleOutcome::Completed(Value::Null)
    );
    assert_eq!(
        runtime
            .invoke(
                &package,
                &key,
                &implementation,
                "run",
                &Value::Null,
                &DEFAULT_SKILL_TOOL_LIMITS,
                &AtomicBool::new(false),
                Arc::new(NoHostCalls),
            )
            .expect("second fresh invocation"),
        SkillToolModuleOutcome::Completed(Value::Null)
    );
    assert_eq!(runtime.compiled.lock().expect("cache").len(), 1);
}

#[cfg(feature = "skill-tool-module-runtime")]
#[test]
fn cancellation_input_and_fuel_limits_fail_closed() {
    let bytes = infinite_run_module();
    let source = Source {
        bytes: bytes.clone(),
    };
    let runtime = NativeSkillToolModuleRuntime::new(&source).expect("runtime");
    let (package, key) = fixture_identity();
    let implementation = ModuleImplementation {
        path: "scripts/modules/runtime.wasm".to_string(),
        export: "run".to_string(),
        content_hash: content_hash_of(&bytes),
    };
    let cancelled = AtomicBool::new(true);
    assert_eq!(
        runtime
            .invoke(
                &package,
                &key,
                &implementation,
                "run",
                &Value::Null,
                &DEFAULT_SKILL_TOOL_LIMITS,
                &cancelled,
                Arc::new(NoHostCalls),
            )
            .expect("cancelled"),
        SkillToolModuleOutcome::Cancelled
    );
    let mut limits = DEFAULT_SKILL_TOOL_LIMITS;
    limits.input_bytes = 1;
    assert_eq!(
        runtime
            .invoke(
                &package,
                &key,
                &implementation,
                "run",
                &serde_json::json!({"large": true}),
                &limits,
                &AtomicBool::new(false),
                Arc::new(NoHostCalls),
            )
            .expect("input limit"),
        SkillToolModuleOutcome::LimitBreached {
            limit: "input-bytes".to_string()
        }
    );
    limits.input_bytes = DEFAULT_SKILL_TOOL_LIMITS.input_bytes;
    limits.fuel = 100;
    assert_eq!(
        runtime
            .invoke(
                &package,
                &key,
                &implementation,
                "run",
                &Value::Null,
                &limits,
                &AtomicBool::new(false),
                Arc::new(NoHostCalls),
            )
            .expect("fuel limit"),
        SkillToolModuleOutcome::LimitBreached {
            limit: "fuel".to_string()
        }
    );
}

#[cfg(feature = "skill-tool-module-runtime")]
#[test]
fn module_can_reach_only_the_structured_host_call_bridge() {
    let request = br#"{"capability":"tool:read_file","arguments":{"path":"src/lib.rs"}}"#;
    let bytes = host_call_module(request);
    let source = Source {
        bytes: bytes.clone(),
    };
    let runtime = NativeSkillToolModuleRuntime::new(&source).expect("runtime");
    wasmtime::Module::validate(&runtime.engine, &bytes).expect("valid host-call fixture");
    let (package, key) = fixture_identity();
    let implementation = ModuleImplementation {
        path: "scripts/modules/runtime.wasm".to_string(),
        export: "run".to_string(),
        content_hash: content_hash_of(&bytes),
    };
    let host = Arc::new(RecordingHostCalls(AtomicU32::new(0)));
    assert_eq!(
        runtime
            .invoke(
                &package,
                &key,
                &implementation,
                "run",
                &Value::Null,
                &DEFAULT_SKILL_TOOL_LIMITS,
                &AtomicBool::new(false),
                host.clone(),
            )
            .expect("invoke"),
        SkillToolModuleOutcome::Completed(Value::Null)
    );
    assert_eq!(host.0.load(Ordering::Acquire), 1);
}

#[cfg(feature = "skill-tool-module-runtime")]
#[test]
fn output_buffer_invalid_json_trap_memory_growth_timeout_and_late_cancel_are_bounded() {
    let (package, key) = fixture_identity();
    let run = |bytes: Vec<u8>, limits: SkillToolLimits, cancelled: &AtomicBool| {
        let source = Source {
            bytes: bytes.clone(),
        };
        let runtime = NativeSkillToolModuleRuntime::new(&source).expect("runtime");
        let implementation = ModuleImplementation {
            path: "scripts/modules/runtime.wasm".to_string(),
            export: "run".to_string(),
            content_hash: content_hash_of(&bytes),
        };
        runtime
            .invoke(
                &package,
                &key,
                &implementation,
                "run",
                &Value::Null,
                &limits,
                cancelled,
                Arc::new(NoHostCalls),
            )
            .expect("invoke")
    };

    let mut output_limits = DEFAULT_SKILL_TOOL_LIMITS;
    output_limits.output_bytes = 4;
    assert_eq!(
        run(
            json_output_module(128, b"12345"),
            output_limits,
            &AtomicBool::new(false)
        ),
        SkillToolModuleOutcome::LimitBreached {
            limit: "output-bytes".to_string()
        }
    );
    assert!(matches!(
        run(
            json_output_module(128, b"not-json"),
            DEFAULT_SKILL_TOOL_LIMITS,
            &AtomicBool::new(false)
        ),
        SkillToolModuleOutcome::Trapped { detail } if detail == "module-invalid-output"
    ));
    assert!(matches!(
        run(
            trap_module(),
            DEFAULT_SKILL_TOOL_LIMITS,
            &AtomicBool::new(false)
        ),
        SkillToolModuleOutcome::Trapped { .. }
    ));
    let mut memory_limits = DEFAULT_SKILL_TOOL_LIMITS;
    memory_limits.memory_bytes = 64 * 1024;
    assert!(matches!(
        run(
            memory_growth_module(),
            memory_limits,
            &AtomicBool::new(false)
        ),
        SkillToolModuleOutcome::Trapped { .. }
    ));
    let mut timeout_limits = DEFAULT_SKILL_TOOL_LIMITS;
    timeout_limits.wall_time_milliseconds = 5;
    assert_eq!(
        run(
            infinite_run_module(),
            timeout_limits,
            &AtomicBool::new(false)
        ),
        SkillToolModuleOutcome::LimitBreached {
            limit: "wall-time".to_string()
        }
    );

    let cancelled = AtomicBool::new(false);
    std::thread::scope(|scope| {
        scope.spawn(|| {
            std::thread::sleep(std::time::Duration::from_millis(5));
            cancelled.store(true, Ordering::Release);
        });
        assert_eq!(
            run(infinite_run_module(), DEFAULT_SKILL_TOOL_LIMITS, &cancelled),
            SkillToolModuleOutcome::Cancelled
        );
    });
}

#[cfg(feature = "skill-tool-module-runtime")]
#[test]
fn a_trapped_invocation_does_not_poison_an_unrelated_revision() {
    let trap = trap_module();
    let source = MutableSource(Mutex::new(trap.clone()));
    let runtime = NativeSkillToolModuleRuntime::new(&source).expect("runtime");
    let (package, trapped_key) = fixture_identity();
    let trapped_implementation = ModuleImplementation {
        path: "scripts/modules/runtime.wasm".to_string(),
        export: "run".to_string(),
        content_hash: content_hash_of(&trap),
    };
    assert!(matches!(
        runtime
            .invoke(
                &package,
                &trapped_key,
                &trapped_implementation,
                "run",
                &Value::Null,
                &DEFAULT_SKILL_TOOL_LIMITS,
                &AtomicBool::new(false),
                Arc::new(NoHostCalls),
            )
            .expect("trapped invocation"),
        SkillToolModuleOutcome::Trapped { .. }
    ));

    let healthy = empty_run_module();
    *source.0.lock().expect("bytes") = healthy.clone();
    let mut healthy_key = trapped_key;
    healthy_key.revision = SkillToolRevision::parse(&"1".repeat(64)).expect("revision");
    let healthy_implementation = ModuleImplementation {
        path: "scripts/modules/runtime.wasm".to_string(),
        export: "run".to_string(),
        content_hash: content_hash_of(&healthy),
    };
    assert_eq!(
        runtime
            .invoke(
                &package,
                &healthy_key,
                &healthy_implementation,
                "run",
                &Value::Null,
                &DEFAULT_SKILL_TOOL_LIMITS,
                &AtomicBool::new(false),
                Arc::new(NoHostCalls),
            )
            .expect("healthy invocation"),
        SkillToolModuleOutcome::Completed(Value::Null)
    );
}

#[cfg(feature = "skill-tool-module-runtime")]
fn fixture_identity() -> (SkillToolPackageRef, SkillToolKey) {
    let manifest = parse_manifest_bytes(
        include_bytes!("../../../../../tests/fixtures/skill-tools/valid-module.json"),
        &DEFAULT_MANIFEST_LIMITS,
    )
    .expect("manifest");
    let declaration = manifest.tools[0].clone();
    let package = SkillToolPackageRef {
        owner: manifest.owner.clone(),
        source: crate::contexts::tooling::skill_tools::domain::SkillToolSourceScope::global(),
        base_revision: "base".to_string(),
        root_path: "/fixture".to_string(),
    };
    let key = SkillToolKey::new(
        manifest.owner,
        package.source.clone(),
        declaration.id,
        crate::contexts::tooling::skill_tools::domain::SkillToolRevision::parse(&"0".repeat(64))
            .expect("revision"),
    );
    (package, key)
}

#[cfg(feature = "skill-tool-module-runtime")]
fn empty_run_module() -> Vec<u8> {
    vec![
        0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00, 0x01, 0x04, 0x01, 0x60, 0x00, 0x00, 0x03,
        0x02, 0x01, 0x00, 0x07, 0x07, 0x01, 0x03, b'r', b'u', b'n', 0x00, 0x00, 0x0a, 0x04, 0x01,
        0x02, 0x00, 0x0b,
    ]
}

#[cfg(feature = "skill-tool-module-runtime")]
fn infinite_run_module() -> Vec<u8> {
    vec![
        0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00, 0x01, 0x04, 0x01, 0x60, 0x00, 0x00, 0x03,
        0x02, 0x01, 0x00, 0x07, 0x07, 0x01, 0x03, b'r', b'u', b'n', 0x00, 0x00, 0x0a, 0x09, 0x01,
        0x07, 0x00, 0x03, 0x40, 0x0c, 0x00, 0x0b, 0x0b,
    ]
}

#[cfg(feature = "skill-tool-module-runtime")]
fn trap_module() -> Vec<u8> {
    unit_module(&[0x00, 0x00, 0x0b], false)
}

#[cfg(feature = "skill-tool-module-runtime")]
fn memory_growth_module() -> Vec<u8> {
    unit_module(&[0x00, 0x41, 0x01, 0x40, 0x00, 0x1a, 0x0b], true)
}

#[cfg(feature = "skill-tool-module-runtime")]
fn unit_module(body: &[u8], exports_memory: bool) -> Vec<u8> {
    let mut module = vec![0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00];
    push_section(&mut module, 1, &[0x01, 0x60, 0x00, 0x00]);
    push_section(&mut module, 3, &[0x01, 0x00]);
    if exports_memory {
        push_section(&mut module, 5, &[0x01, 0x00, 0x01]);
        push_section(
            &mut module,
            7,
            &[
                0x02, 0x03, b'r', b'u', b'n', 0x00, 0x00, 0x06, b'm', b'e', b'm', b'o', b'r', b'y',
                0x02, 0x00,
            ],
        );
    } else {
        push_section(&mut module, 7, &[0x01, 0x03, b'r', b'u', b'n', 0x00, 0x00]);
    }
    let mut code = vec![0x01, u8::try_from(body.len()).expect("small body")];
    code.extend_from_slice(body);
    push_section(&mut module, 10, &code);
    module
}

#[cfg(feature = "skill-tool-module-runtime")]
fn json_output_module(pointer: u32, output: &[u8]) -> Vec<u8> {
    let mut module = vec![0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00];
    push_section(&mut module, 1, &[0x01, 0x60, 0x02, 0x7f, 0x7f, 0x01, 0x7e]);
    push_section(&mut module, 3, &[0x01, 0x00]);
    push_section(&mut module, 5, &[0x01, 0x01, 0x01, 0x01]);
    push_section(
        &mut module,
        7,
        &[
            0x02, 0x06, b'm', b'e', b'm', b'o', b'r', b'y', 0x02, 0x00, 0x03, b'r', b'u', b'n',
            0x00, 0x00,
        ],
    );
    let packed = (u64::from(pointer) << 32) | output.len() as u64;
    let mut body = vec![0x00, 0x42];
    push_signed_i64(&mut body, packed as i64);
    body.push(0x0b);
    let mut code = vec![0x01, u8::try_from(body.len()).expect("small body")];
    code.extend_from_slice(&body);
    push_section(&mut module, 10, &code);
    let mut data = vec![0x01, 0x00, 0x41];
    push_signed_i32(&mut data, pointer as i32);
    data.push(0x0b);
    data.push(u8::try_from(output.len()).expect("small output"));
    data.extend_from_slice(output);
    push_section(&mut module, 11, &data);
    module
}

#[cfg(feature = "skill-tool-module-runtime")]
fn host_call_module(request: &[u8]) -> Vec<u8> {
    let mut module = vec![0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00];
    push_section(
        &mut module,
        1,
        &[
            0x02, 0x60, 0x04, 0x7f, 0x7f, 0x7f, 0x7f, 0x01, 0x7f, 0x60, 0x00, 0x00,
        ],
    );
    let mut imports = vec![0x01, 0x07];
    imports.extend_from_slice(b"vanehub");
    imports.push(0x09);
    imports.extend_from_slice(b"host_call");
    imports.extend_from_slice(&[0x00, 0x00]);
    push_section(&mut module, 2, &imports);
    push_section(&mut module, 3, &[0x01, 0x01]);
    push_section(&mut module, 5, &[0x01, 0x01, 0x01, 0x01]);
    push_section(
        &mut module,
        7,
        &[
            0x02, 0x06, b'm', b'e', b'm', b'o', b'r', b'y', 0x02, 0x00, 0x03, b'r', b'u', b'n',
            0x00, 0x01,
        ],
    );
    let request_length = u8::try_from(request.len()).expect("small request");
    let mut body = vec![0x00, 0x41, 0x00, 0x41];
    push_signed_i32(&mut body, i32::from(request_length));
    body.extend_from_slice(&[0x41, 0x80, 0x02, 0x41, 0x80, 0x04, 0x10, 0x00, 0x1a, 0x0b]);
    let mut code = vec![0x01, u8::try_from(body.len()).expect("small function")];
    code.extend_from_slice(&body);
    push_section(&mut module, 10, &code);
    let mut data = vec![0x01, 0x00, 0x41, 0x00, 0x0b, request_length];
    data.extend_from_slice(request);
    push_section(&mut module, 11, &data);
    module
}

#[cfg(feature = "skill-tool-module-runtime")]
fn push_section(module: &mut Vec<u8>, id: u8, body: &[u8]) {
    module.push(id);
    module.push(u8::try_from(body.len()).expect("small fixture section"));
    module.extend_from_slice(body);
}

#[cfg(feature = "skill-tool-module-runtime")]
fn push_signed_i32(bytes: &mut Vec<u8>, mut value: i32) {
    loop {
        let byte = (value & 0x7f) as u8;
        value >>= 7;
        let done = (value == 0 && byte & 0x40 == 0) || (value == -1 && byte & 0x40 != 0);
        bytes.push(if done { byte } else { byte | 0x80 });
        if done {
            return;
        }
    }
}

#[cfg(feature = "skill-tool-module-runtime")]
fn push_signed_i64(bytes: &mut Vec<u8>, mut value: i64) {
    loop {
        let byte = (value & 0x7f) as u8;
        value >>= 7;
        let done = (value == 0 && byte & 0x40 == 0) || (value == -1 && byte & 0x40 != 0);
        bytes.push(if done { byte } else { byte | 0x80 });
        if done {
            return;
        }
    }
}
