use std::collections::BTreeMap;
use std::sync::Arc;

use zeroize::Zeroizing;

use super::application::{EvidencePipeline, PipelineHealthSnapshot, RuntimeEvidenceProjector};
use super::domain::canonical_workspace_scope;
use super::domain::FeedbackState;
use super::infrastructure::{
    EvidenceIngestionProcessor, EvidenceOverview, EvidencePageRequest, EvidencePurgeOutcome,
    EvidencePurgeRequest, EvidencePurgeScope, EvidenceQueryScope, EvidenceRepositoryError,
    EvidenceSeedLineage, FeedbackTransitionError, SaveFeedbackRequest, SavedFeedback,
    SqliteEvolutionEvidenceRepository,
};
use crate::contexts::operations::api::DiagnosticLogPort;
use crate::platform::credentials::OsCredentialStore;
use crate::platform::database::NativeDatabase;

const CREDENTIAL_SERVICE: &str = "io.vanehub.ai";
const CREDENTIAL_ACCOUNT: &str = "skill-evolution-evidence-key-v1";

pub(crate) struct SkillEvolutionEvidenceApi {
    repository: SqliteEvolutionEvidenceRepository,
    installation_key: Option<Zeroizing<Vec<u8>>>,
    pipeline: Option<Arc<EvidencePipeline>>,
}

impl SkillEvolutionEvidenceApi {
    pub(crate) fn new(database: NativeDatabase, logging: Arc<dyn DiagnosticLogPort>) -> Self {
        let repository = SqliteEvolutionEvidenceRepository::new(database);
        let installation_key = load_or_create_key().ok();
        let pipeline = installation_key.as_deref().and_then(|key| {
            EvidenceIngestionProcessor::new(repository.clone(), key)
                .ok()
                .map(|processor| Arc::new(EvidencePipeline::start(Arc::new(processor), logging)))
        });
        Self {
            repository,
            installation_key,
            pipeline,
        }
    }

    pub(crate) fn projector(&self) -> RuntimeEvidenceProjector {
        match (self.pipeline.as_ref(), self.installation_key.as_deref()) {
            (Some(pipeline), Some(key)) => RuntimeEvidenceProjector::enabled(pipeline.clone(), key),
            _ => RuntimeEvidenceProjector::disabled(),
        }
    }

    pub(crate) fn save_feedback(
        &self,
        request: &SaveFeedbackRequest,
    ) -> Result<SavedFeedback, FeedbackTransitionError> {
        let key = self
            .installation_key
            .as_deref()
            .ok_or(FeedbackTransitionError::Storage)?;
        self.repository.save_feedback(request, key)
    }

    pub(crate) fn feedback_for_messages(
        &self,
        message_ids: &[String],
    ) -> Result<BTreeMap<String, FeedbackSummary>, EvidenceRepositoryError> {
        let connection = self
            .repository
            .database
            .connection()
            .map_err(|_| EvidenceRepositoryError::Storage)?;
        let mut summaries = BTreeMap::new();
        for message_id in message_ids {
            let result = connection.query_row(
                "SELECT feedback_state, feedback_revision, sanitized_note FROM evolution_feedback_current WHERE message_id = ?1",
                [message_id],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?, row.get::<_, Option<String>>(2)?)),
            );
            match result {
                Ok((state, revision, note)) => {
                    if let Some(state) = parse_state(&state) {
                        summaries.insert(
                            message_id.clone(),
                            FeedbackSummary {
                                state: Some(state),
                                revision: u64::try_from(revision).unwrap_or_default(),
                                sanitized_note: note,
                            },
                        );
                    }
                }
                Err(rusqlite::Error::QueryReturnedNoRows) => {}
                Err(_) => return Err(EvidenceRepositoryError::Storage),
            }
            if !summaries.contains_key(message_id) {
                let revision: i64 = connection
                    .query_row(
                        "SELECT COALESCE(MAX(feedback_revision), 0) FROM evolution_feedback_events WHERE message_id = ?1",
                        [message_id],
                        |row| row.get(0),
                    )
                    .map_err(|_| EvidenceRepositoryError::Storage)?;
                if revision > 0 {
                    summaries.insert(
                        message_id.clone(),
                        FeedbackSummary {
                            state: None,
                            revision: u64::try_from(revision).unwrap_or_default(),
                            sanitized_note: None,
                        },
                    );
                }
            }
        }
        Ok(summaries)
    }

    pub(crate) fn query_overview(
        &self,
        request: &EvidencePageRequest,
    ) -> Result<EvidenceOverview, EvidenceRepositoryError> {
        let request = EvidencePageRequest {
            scope: self.canonical_scope(&request.scope)?,
            limit: request.limit,
            cursor: request.cursor.clone(),
        };
        let mut overview = self.repository.query_overview(&request)?;
        match self.pipeline.as_ref() {
            Some(pipeline) => apply_health(&mut overview, pipeline.health()),
            None => {
                overview.pipeline.collection_enabled = false;
                overview.pipeline.status = "disabled".to_string();
            }
        }
        Ok(overview)
    }

    pub(crate) fn query_seed_lineage(
        &self,
        seed_id: &str,
        scope: &EvidenceQueryScope,
    ) -> Result<Option<EvidenceSeedLineage>, EvidenceRepositoryError> {
        self.repository
            .query_seed_lineage(seed_id, &self.canonical_scope(scope)?)
    }

    pub(crate) fn purge(
        &self,
        request: &EvidencePurgeRequest,
    ) -> Result<EvidencePurgeOutcome, EvidenceRepositoryError> {
        let request = EvidencePurgeRequest {
            operation_id: request.operation_id.clone(),
            scope: self.canonical_purge_scope(&request.scope)?,
        };
        self.repository.purge(&request)
    }

    pub(crate) fn purge_skill_scope(
        &self,
        operation_id: String,
        workspace: Option<String>,
        skill_id: String,
        confirmed: bool,
    ) -> Result<EvidencePurgeOutcome, PurgeSkillEvidenceError> {
        if !confirmed || operation_id.trim().is_empty() || skill_id.trim().is_empty() {
            return Err(PurgeSkillEvidenceError::InvalidRequest);
        }
        let scope = workspace.map_or_else(
            || EvidencePurgeScope::Skill(skill_id.clone()),
            |workspace| EvidencePurgeScope::WorkspaceSkill {
                workspace,
                skill_id: skill_id.clone(),
            },
        );
        self.purge(&EvidencePurgeRequest {
            operation_id,
            scope,
        })
        .map_err(|_| PurgeSkillEvidenceError::Storage)
    }

    fn canonical_scope(
        &self,
        scope: &EvidenceQueryScope,
    ) -> Result<EvidenceQueryScope, EvidenceRepositoryError> {
        Ok(EvidenceQueryScope {
            workspace: scope
                .workspace
                .as_deref()
                .map(|workspace| self.canonical_workspace(workspace))
                .transpose()?,
            skill_id: scope.skill_id.clone(),
        })
    }

    fn canonical_purge_scope(
        &self,
        scope: &EvidencePurgeScope,
    ) -> Result<EvidencePurgeScope, EvidenceRepositoryError> {
        match scope {
            EvidencePurgeScope::Workspace(workspace) => self
                .canonical_workspace(workspace)
                .map(EvidencePurgeScope::Workspace),
            EvidencePurgeScope::WorkspaceSkill {
                workspace,
                skill_id,
            } => Ok(EvidencePurgeScope::WorkspaceSkill {
                workspace: self.canonical_workspace(workspace)?,
                skill_id: skill_id.clone(),
            }),
            value => Ok(value.clone()),
        }
    }

    fn canonical_workspace(&self, workspace: &str) -> Result<String, EvidenceRepositoryError> {
        let key = self
            .installation_key
            .as_deref()
            .ok_or(EvidenceRepositoryError::Storage)?;
        canonical_workspace_scope(key, workspace).map_err(|_| EvidenceRepositoryError::Storage)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PurgeSkillEvidenceError {
    InvalidRequest,
    Storage,
}

impl PurgeSkillEvidenceError {
    pub(crate) fn code(self) -> &'static str {
        match self {
            Self::InvalidRequest => "evidence-purge-invalid-request",
            Self::Storage => "evidence-purge-failed",
        }
    }
}

fn apply_health(overview: &mut EvidenceOverview, health: PipelineHealthSnapshot) {
    let failures = health.failures.into_iter().sum::<u64>() as usize;
    let queue_drops = health.dropped_by_priority.into_iter().sum::<u64>() as usize;
    overview.pipeline.queue_depth = health.queue_depth;
    overview.pipeline.failure_count = failures;
    overview.pipeline.status = if failures > 0 || queue_drops > 0 {
        "degraded".to_string()
    } else {
        "healthy".to_string()
    };
    overview.dropped_count = overview.dropped_count.saturating_add(queue_drops);
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FeedbackSummary {
    pub(crate) state: Option<FeedbackState>,
    pub(crate) revision: u64,
    pub(crate) sanitized_note: Option<String>,
}

fn load_or_create_key() -> Result<Zeroizing<Vec<u8>>, ()> {
    let store = OsCredentialStore::new(CREDENTIAL_SERVICE);
    if let Some(encoded) = store.get(CREDENTIAL_ACCOUNT).map_err(|_| ())? {
        return decode_key(&Zeroizing::new(encoded));
    }
    let key = Zeroizing::new(std::array::from_fn::<_, 32, _>(|_| rand::random::<u8>()));
    let encoded = Zeroizing::new(
        key.iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>(),
    );
    store.set(CREDENTIAL_ACCOUNT, &encoded).map_err(|_| ())?;
    Ok(Zeroizing::new(key.to_vec()))
}

fn decode_key(encoded: &str) -> Result<Zeroizing<Vec<u8>>, ()> {
    if encoded.len() != 64 {
        return Err(());
    }
    let bytes = (0..encoded.len())
        .step_by(2)
        .map(|index| u8::from_str_radix(&encoded[index..index + 2], 16).map_err(|_| ()))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(Zeroizing::new(bytes))
}

fn parse_state(value: &str) -> Option<FeedbackState> {
    match value {
        "helpful" => Some(FeedbackState::Helpful),
        "unhelpful" => Some(FeedbackState::Unhelpful),
        "corrected" => Some(FeedbackState::Corrected),
        _ => None,
    }
}
