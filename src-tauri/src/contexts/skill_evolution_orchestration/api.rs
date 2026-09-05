use chrono::Utc;
use std::{collections::BTreeMap, sync::Arc};

use crate::{
    contexts::{
        operations::api::{DiagnosticLog, DiagnosticLogPort, LogSeverity},
        skill_evolution_orchestration::{
            application::{
                AuthoritativeTriggerSourceV1, EvolutionTriggerIngressService,
                RelevantMutationKindV1,
            },
            infrastructure::OrchestrationRepository,
        },
    },
    platform::database::NativeDatabase,
};

pub(crate) struct SkillEvolutionOrchestrationApi {
    pub(super) database: NativeDatabase,
    pub(super) ingress: EvolutionTriggerIngressService,
    logging: Arc<dyn DiagnosticLogPort>,
}

impl SkillEvolutionOrchestrationApi {
    pub(crate) fn new(database: NativeDatabase, logging: Arc<dyn DiagnosticLogPort>) -> Self {
        Self {
            database: database.clone(),
            ingress: EvolutionTriggerIngressService::new(Arc::new(OrchestrationRepository::new(
                database,
            ))),
            logging,
        }
    }

    pub(crate) fn publish_feedback_change(
        &self,
        workspace_id: Option<&str>,
        message_id: &str,
        feedback_revision: u64,
        authorization_event_id: Option<&str>,
    ) {
        let Some(workspace_id) = workspace_id else {
            self.warn("workspace-unavailable");
            return;
        };
        let now_ms = Utc::now().timestamp_millis();
        let source = AuthoritativeTriggerSourceV1 {
            workspace_id: workspace_id.into(),
            source_id: message_id.into(),
            source_revision: feedback_revision,
            occurred_at_ms: now_ms,
        };
        if self
            .ingress
            .explicit_feedback_commit(source.clone(), now_ms)
            .is_err()
        {
            self.warn("feedback-trigger-unavailable");
        }
        if let Some(authorization_event_id) = authorization_event_id {
            let authorization_source = AuthoritativeTriggerSourceV1 {
                source_id: authorization_event_id.into(),
                source_revision: 1,
                ..source
            };
            if self
                .ingress
                .relevant_mutation(RelevantMutationKindV1::Policy, authorization_source, now_ms)
                .is_err()
            {
                self.warn("authorization-trigger-unavailable");
            }
        }
    }

    pub(crate) fn background_lifecycle(
        &self,
    ) -> crate::contexts::skill_evolution_orchestration::infrastructure::EvolutionBackgroundLifecycle
    {
        crate::contexts::skill_evolution_orchestration::infrastructure::EvolutionBackgroundLifecycle::new(
            self.database.clone(),
            self.ingress.clone(),
            self.logging.clone(),
        )
    }

    fn warn(&self, reason: &'static str) {
        let _ = self.logging.write_diagnostic(DiagnosticLog {
            severity: LogSeverity::Warn,
            category: "skill-evolution.orchestration.trigger".into(),
            message: "Skill evolution trigger was not queued".into(),
            context: BTreeMap::from([("reason".into(), reason.into())]),
        });
    }
}
