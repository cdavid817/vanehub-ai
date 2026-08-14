use super::*;
use std::sync::Mutex;

struct Port {
    witnesses: Vec<DelegationRestartWitness>,
    persisted: Mutex<Vec<DelegationRestartResolution>>,
}

impl DelegationRestartRecoveryPort for Port {
    fn list_interrupted(&self) -> Result<Vec<DelegationInterruptedApply>, ()> {
        Ok(self
            .witnesses
            .iter()
            .enumerate()
            .map(|(index, _)| DelegationInterruptedApply {
                apply_attempt_id: format!("apply-{index}"),
                capsule_reference: format!("recovery/apply-{index}"),
            })
            .collect())
    }

    fn inspect(&self, apply: &DelegationInterruptedApply) -> Result<DelegationRestartWitness, ()> {
        let index = apply
            .apply_attempt_id
            .strip_prefix("apply-")
            .and_then(|value| value.parse::<usize>().ok())
            .ok_or(())?;
        self.witnesses.get(index).copied().ok_or(())
    }

    fn persist_resolution(
        &self,
        _: &DelegationInterruptedApply,
        resolution: DelegationRestartResolution,
    ) -> Result<(), ()> {
        self.persisted.lock().map_err(|_| ())?.push(resolution);
        Ok(())
    }
}

#[test]
fn distinguishes_completed_rolled_back_and_manual_without_replay() {
    let port = Arc::new(Port {
        witnesses: vec![
            DelegationRestartWitness {
                post_apply_matches: true,
                pre_apply_matches: false,
                capsule_integral: true,
            },
            DelegationRestartWitness {
                post_apply_matches: false,
                pre_apply_matches: true,
                capsule_integral: true,
            },
            DelegationRestartWitness {
                post_apply_matches: false,
                pre_apply_matches: false,
                capsule_integral: true,
            },
        ],
        persisted: Mutex::new(Vec::new()),
    });

    let outcomes = DelegationRestartRecoveryService::new(port.clone())
        .reconcile()
        .expect("reconciled");

    assert_eq!(
        outcomes.iter().map(|item| item.1).collect::<Vec<_>>(),
        vec![
            DelegationRestartResolution::SafelyCompleted,
            DelegationRestartResolution::RolledBack,
            DelegationRestartResolution::ManualRecoveryRequired,
        ]
    );
    assert_eq!(port.persisted.lock().expect("persisted").len(), 3);
}

#[test]
fn ambiguous_or_non_integral_witness_fails_closed_to_manual_recovery() {
    for witness in [
        DelegationRestartWitness {
            post_apply_matches: true,
            pre_apply_matches: true,
            capsule_integral: true,
        },
        DelegationRestartWitness {
            post_apply_matches: false,
            pre_apply_matches: true,
            capsule_integral: false,
        },
    ] {
        assert_eq!(
            classify(witness),
            DelegationRestartResolution::ManualRecoveryRequired
        );
    }
}
