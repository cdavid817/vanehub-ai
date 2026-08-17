use super::*;
use crate::contexts::tooling::skill_tools::domain::{
    parse_manifest_bytes, SkillToolRevision, SkillToolSourceScope, DEFAULT_MANIFEST_LIMITS,
    DEFAULT_SKILL_TOOL_LIMITS,
};
use std::sync::atomic::{AtomicU32, Ordering};

struct Host {
    calls: AtomicU32,
    outcome: SkillToolDispatchOutcome,
}

impl SkillToolHostDispatchPort for Host {
    fn dispatch(
        &self,
        _principal: &SkillToolPrincipal,
        _capability: &SkillToolCapability,
        arguments: &Value,
    ) -> Result<SkillToolDispatchOutcome, SkillToolApplicationError> {
        self.calls.fetch_add(1, Ordering::AcqRel);
        assert_eq!(arguments, &serde_json::json!({"path": "src/lib.rs"}));
        Ok(self.outcome.clone())
    }
}

struct Modes(bool);

impl SkillToolCapabilityModePort for Modes {
    fn allows(&self, _mode: SkillToolExecutionMode, _capability: &SkillToolCapability) -> bool {
        self.0
    }
}

struct Budget(AtomicU32);

impl SkillToolInvocationBudgetPort for Budget {
    fn reserve_host_call(&self) -> Result<(), SkillToolApplicationError> {
        self.0.fetch_add(1, Ordering::AcqRel);
        Ok(())
    }

    fn consume_output(&self, _bytes: u64) -> Result<(), SkillToolApplicationError> {
        Ok(())
    }
}

fn fixture(
    mode_allowed: bool,
    cancelled: bool,
    chain: Vec<String>,
) -> (SkillToolModuleHostDispatcher, Arc<Host>, Arc<Budget>) {
    let manifest = parse_manifest_bytes(
        include_bytes!("../../../../../tests/fixtures/skill-tools/valid-declarative.json"),
        &DEFAULT_MANIFEST_LIMITS,
    )
    .expect("manifest");
    let declaration = manifest.tools[0].clone();
    let principal = SkillToolPrincipal {
        parent_agent_id: "agent".to_string(),
        key: crate::contexts::tooling::skill_tools::domain::SkillToolKey::new(
            manifest.owner,
            SkillToolSourceScope::global(),
            declaration.id,
            SkillToolRevision::parse(&"0".repeat(64)).expect("revision"),
        ),
        workspace_path: Some("/workspace".to_string()),
        session_id: Some("session".to_string()),
        generation_id: "generation".to_string(),
        delegation_chain: chain,
    };
    let host = Arc::new(Host {
        calls: AtomicU32::new(0),
        outcome: SkillToolDispatchOutcome::Completed(serde_json::json!({"ok": true})),
    });
    let budget = Arc::new(Budget(AtomicU32::new(0)));
    let dispatcher = SkillToolModuleHostDispatcher::new(
        principal,
        SkillToolExecutionMode::Execute,
        declaration.capabilities,
        DEFAULT_SKILL_TOOL_LIMITS,
        host.clone(),
        Arc::new(Modes(mode_allowed)),
        budget.clone(),
        Arc::new(AtomicBool::new(cancelled)),
    );
    (dispatcher, host, budget)
}

fn request(capability: &str) -> Value {
    serde_json::json!({
        "capability": capability,
        "arguments": {"path": "src/lib.rs"}
    })
}

#[test]
fn declared_allowed_call_reaches_existing_dispatch_once() {
    let (dispatcher, host, budget) = fixture(true, false, Vec::new());
    assert!(matches!(
        dispatcher.call(&request("tool:read_file")).expect("call"),
        SkillToolDispatchOutcome::Completed(_)
    ));
    assert_eq!(host.calls.load(Ordering::Acquire), 1);
    assert_eq!(budget.0.load(Ordering::Acquire), 1);
}

#[test]
fn capability_mode_cycle_depth_and_cancellation_fail_before_dispatch() {
    let cases = [
        fixture(false, false, Vec::new()),
        fixture(true, true, Vec::new()),
        fixture(true, false, vec!["tool:read_file".to_string()]),
        fixture(true, false, vec!["tool:other".to_string(); 4]),
    ];
    for (dispatcher, host, budget) in cases {
        assert!(matches!(
            dispatcher.call(&request("tool:read_file")).expect("denied"),
            SkillToolDispatchOutcome::Denied { .. } | SkillToolDispatchOutcome::Cancelled
        ));
        assert_eq!(host.calls.load(Ordering::Acquire), 0);
        assert_eq!(budget.0.load(Ordering::Acquire), 0);
    }
    let (dispatcher, host, budget) = fixture(true, false, Vec::new());
    assert!(matches!(
        dispatcher
            .call(&request("tool:write_file"))
            .expect("undeclared"),
        SkillToolDispatchOutcome::Denied { .. }
    ));
    assert_eq!(host.calls.load(Ordering::Acquire), 0);
    assert_eq!(budget.0.load(Ordering::Acquire), 0);

    let (dispatcher, host, budget) = fixture(true, false, Vec::new());
    assert!(dispatcher.call(&request("skill__forged__tool")).is_err());
    assert_eq!(host.calls.load(Ordering::Acquire), 0);
    assert_eq!(budget.0.load(Ordering::Acquire), 0);
}
