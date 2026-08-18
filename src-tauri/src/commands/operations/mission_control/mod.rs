pub(crate) mod get_overview;
pub(crate) mod get_run;
pub(crate) mod perform_action;

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct MissionControlActionInput {
    pub(crate) run_id: String,
    pub(crate) version: u64,
    pub(crate) action: String,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct MissionControlActionReceipt {
    pub(crate) run: crate::contexts::operations::application::MissionControlRunSummary,
    pub(crate) operation_id: Option<String>,
}

pub(crate) fn receipt(
    run: crate::contexts::operations::api::AgentRun,
    action: &str,
) -> MissionControlActionReceipt {
    let operation_id = matches!(action, "retry" | "verify").then(|| run.id.clone());
    MissionControlActionReceipt {
        run: crate::contexts::operations::application::project_mission_control_run(run),
        operation_id,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contexts::operations::api::{AgentRun, RunOwner, RunRecoveryPolicy, RunState};

    #[test]
    fn action_input_and_receipt_keep_the_camel_case_transport_contract() {
        let input: MissionControlActionInput = serde_json::from_value(serde_json::json!({
            "runId": "run-1", "version": 3, "action": "verify"
        }))
        .expect("input contract");
        assert_eq!(
            (input.run_id.as_str(), input.version, input.action.as_str()),
            ("run-1", 3, "verify")
        );

        let run = AgentRun {
            id: "run-1".into(),
            owner: RunOwner {
                owner_type: "plan_run".into(),
                owner_id: "plan-1".into(),
            },
            links: Vec::new(),
            parent_run_id: None,
            state: RunState::Failed,
            recovery_policy: RunRecoveryPolicy::OwnerReconciles,
            runner: None,
            retry_count: 0,
            max_retries: 1,
            reason_code: Some("verification_failed".into()),
            created_at: "2026-08-17T00:00:00Z".into(),
            updated_at: "2026-08-17T00:01:00Z".into(),
            version: 3,
            last_witness: "failed".into(),
        };
        let value = serde_json::to_value(receipt(run, "verify")).expect("receipt contract");
        assert_eq!(value["operationId"], "run-1");
        assert_eq!(value["run"]["runId"], "run-1");
        assert!(value.get("operation_id").is_none());
    }
}
