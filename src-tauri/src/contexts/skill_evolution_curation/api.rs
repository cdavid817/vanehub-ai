//! Published Curator facade used by runtime adapters.

use super::api_models::*;
use super::api_queries;
use crate::contexts::agent_runtime::api::AgentRuntimeApi;
use crate::contexts::operations::application::DiagnosticLogPort;
use crate::contexts::skill_evolution_assessment::api::DraftQualityReviewApi;
use crate::contexts::skill_evolution_curation::application::CuratorNotificationPort;
use crate::contexts::skill_evolution_curation::infrastructure::SqliteCuratorRepository;
use crate::contexts::tooling::skills::api::SkillApi;
use crate::platform::database::NativeDatabase;
use rusqlite::OptionalExtension;
use serde_json::{to_value, Value};
use std::sync::Arc;

pub(crate) use super::api_models::{
    CuratorApiError, CuratorApproveInput, CuratorAuditQuery, CuratorDeferInput, CuratorDraftInput,
    CuratorPolicyInput, CuratorPreviewInput, CuratorQueueQuery, CuratorRejectInput,
    CuratorResumeInput, CuratorRetryInput,
};
pub(crate) use super::api_system_policy::{
    CuratorSystemPolicyApplyInput, CuratorSystemPolicyApplyReceipt,
};
pub(crate) use super::rollback_candidate::{
    CuratorRollbackCandidateInput, CuratorRollbackCandidateReceipt,
};

#[derive(Clone)]
pub(crate) struct SkillEvolutionCurationApi {
    pub(super) database: NativeDatabase,
    pub(super) skills: SkillApi,
    pub(super) reviewer: DraftQualityReviewApi,
    pub(super) notifications: Arc<dyn CuratorNotificationPort>,
    pub(super) logging: Arc<dyn DiagnosticLogPort>,
}

impl SkillEvolutionCurationApi {
    pub(crate) fn new(
        database: NativeDatabase,
        skills: SkillApi,
        runtime: AgentRuntimeApi,
        notifications: Arc<dyn CuratorNotificationPort>,
        logging: Arc<dyn DiagnosticLogPort>,
    ) -> Self {
        Self {
            reviewer: DraftQualityReviewApi::with_model(database.clone(), runtime),
            database,
            skills,
            notifications,
            logging,
        }
    }

    pub(crate) fn queue(&self, query: CuratorQueueQuery) -> Result<Value, CuratorApiError> {
        let connection = self.database.connection().map_err(|_| storage())?;
        api_queries::queue(&connection, query)
    }

    pub(crate) fn detail(&self, candidate_id: &str) -> Result<Value, CuratorApiError> {
        let connection = self.database.connection().map_err(|_| storage())?;
        api_queries::detail(&connection, candidate_id)
    }

    pub(crate) fn audit(&self, query: CuratorAuditQuery) -> Result<Value, CuratorApiError> {
        let connection = self.database.connection().map_err(|_| storage())?;
        api_queries::audit(&connection, query)
    }

    pub(crate) fn policy(&self, workspace_id: &str) -> Result<Value, CuratorApiError> {
        let mut connection = self.database.connection().map_err(|_| storage())?;
        let repository = SqliteCuratorRepository::new(&mut connection);
        let policy = repository
            .load_policy(workspace_id)
            .map_err(super::api_action_support::policy_error)?;
        to_value(policy).map_err(|_| storage())
    }

    pub(crate) fn application_recovery_status(
        &self,
        application_id: &str,
    ) -> Result<Option<CuratorApplicationRecoveryStatus>, CuratorApiError> {
        if application_id.trim().is_empty() {
            return Err(CuratorApiError::new("invalid_request"));
        }
        let connection = self.database.connection().map_err(|_| storage())?;
        connection
            .query_row(
                "SELECT application_id,status,overlay_history_id
                 FROM evolution_curator_applications WHERE application_id=?1",
                [application_id],
                |row| {
                    Ok(CuratorApplicationRecoveryStatus {
                        application_id: row.get(0)?,
                        status: row.get(1)?,
                        overlay_history_id: row.get(2)?,
                    })
                },
            )
            .optional()
            .map_err(|_| storage())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CuratorApplicationRecoveryStatus {
    pub(crate) application_id: String,
    pub(crate) status: String,
    pub(crate) overlay_history_id: Option<String>,
}

fn storage() -> CuratorApiError {
    CuratorApiError::new("storage_unavailable")
}
