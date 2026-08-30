use super::api::SkillEvolutionCurationApi;
use super::api_action_support::*;
use super::api_models::*;
use super::{application::*, domain::*, infrastructure::*};
use serde_json::to_value;

impl SkillEvolutionCurationApi {
    pub(crate) async fn save_draft(
        &self,
        input: CuratorDraftInput,
        now_ms: i64,
    ) -> CuratorApiResult {
        validate_key(&input.idempotency_key)?;
        let workspace = self.workspace(&input.candidate_id)?;
        let request = CuratorDraftRequestV1 {
            schema_version: input.schema_version,
            candidate_id: input.candidate_id.clone(),
            expected_candidate_revision: input.expected_candidate_revision,
            target_skill_id: input.target_skill_id,
            target_revision: input.target_revision,
            overlay_scope: input.overlay_scope,
            mutation: input.mutation,
            rationale: input.rationale,
            expected_effective_change: input.expected_effective_change,
            supporting_files: Vec::new(),
            requested_permissions: Vec::new(),
            commands: Vec::new(),
            direct_base_edit: false,
        };
        let draft = {
            let mut connection = self.connection()?;
            let mut repository = SqliteCuratorRepository::new(&mut connection);
            let overlay = SkillApiCuratorDraftValidator::new(&self.skills, Some(&workspace));
            CuratorDraftService::new(&mut repository, &overlay)
                .create_revision(&request, now_ms)
                .map_err(|error| self.action_error(&input.candidate_id, error.to_string()))?
        };
        let candidate_revision = input
            .expected_candidate_revision
            .checked_add(1)
            .ok_or_else(invalid)?;
        let mut connection = self.connection()?;
        let mut repository = SqliteCuratorRepository::new(&mut connection);
        let reviewer = AssessmentApiCuratorDraftReviewer::new(self.reviewer.clone());
        CuratorDraftReviewService::new(&mut repository, &reviewer)
            .review_current(
                CuratorDraftReviewRequest {
                    candidate_id: &input.candidate_id,
                    expected_candidate_revision: candidate_revision,
                    expected_draft_revision: draft.revision,
                },
                now_ms,
            )
            .await
            .map_err(|error| self.action_error(&input.candidate_id, error.to_string()))?;
        let result = action_receipt(
            &connection,
            &input.candidate_id,
            &input.idempotency_key,
            false,
        );
        self.dispatch_after(result, now_ms)
    }

    pub(crate) fn preview(&self, input: CuratorPreviewInput, now_ms: i64) -> CuratorApiResult {
        validate_key(&input.idempotency_key)?;
        let workspace = self.workspace(&input.candidate_id)?;
        let mut connection = self.connection()?;
        let mut repository = SqliteCuratorRepository::new(&mut connection);
        let overlay = SkillApiCuratorPreviewer::new(&self.skills, Some(&workspace));
        let preview = CuratorPreviewService::new(&mut repository, &overlay)
            .create(
                CuratorPreviewRequest {
                    candidate_id: &input.candidate_id,
                    expected_candidate_revision: input.expected_candidate_revision,
                    expected_draft_revision: input.expected_draft_revision,
                    expected_assessment_id: &input.expected_assessment_id,
                },
                now_ms,
            )
            .map_err(|error| self.action_error(&input.candidate_id, error.to_string()))?;
        to_value(preview).map_err(|_| storage())
    }

    pub(crate) fn reject(&self, input: CuratorRejectInput, now_ms: i64) -> CuratorApiResult {
        let mut connection = self.connection()?;
        let mut repository = SqliteCuratorRepository::new(&mut connection);
        let outcome = CuratorDecisionService::new(
            &mut repository,
            CuratorTrustedActor::local_interactive_user(now_ms),
        )
        .reject(CuratorRejectRequest {
            candidate_id: &input.candidate_id,
            expected_candidate_revision: input.expected_candidate_revision,
            idempotency_key: &input.idempotency_key,
            reason: input.reason,
            note: input.note.as_deref(),
        })
        .map_err(|error| self.action_error(&input.candidate_id, error.to_string()))?;
        let result = action_receipt(
            &connection,
            &input.candidate_id,
            &outcome.decision_id,
            outcome.duplicate,
        );
        self.dispatch_after(result, now_ms)
    }

    pub(crate) fn defer(&self, input: CuratorDeferInput, now_ms: i64) -> CuratorApiResult {
        let mut connection = self.connection()?;
        let mut repository = SqliteCuratorRepository::new(&mut connection);
        let outcome = CuratorDecisionService::new(
            &mut repository,
            CuratorTrustedActor::local_interactive_user(now_ms),
        )
        .defer(CuratorDeferRequest {
            candidate_id: &input.candidate_id,
            expected_candidate_revision: input.expected_candidate_revision,
            idempotency_key: &input.idempotency_key,
            reason: input.reason,
            note: input.note.as_deref(),
            review_after_ms: input.review_after_ms,
        })
        .map_err(|error| self.action_error(&input.candidate_id, error.to_string()))?;
        let result = action_receipt(
            &connection,
            &input.candidate_id,
            &outcome.decision_id,
            outcome.duplicate,
        );
        self.dispatch_after(result, now_ms)
    }

    pub(crate) fn resume(&self, input: CuratorResumeInput, now_ms: i64) -> CuratorApiResult {
        let mut connection = self.connection()?;
        let mut repository = SqliteCuratorRepository::new(&mut connection);
        let outcome = CuratorDecisionService::new(
            &mut repository,
            CuratorTrustedActor::local_interactive_user(now_ms),
        )
        .resume(CuratorResumeRequest {
            candidate_id: &input.candidate_id,
            expected_candidate_revision: input.expected_candidate_revision,
            expected_candidate_hash: &input.expected_candidate_hash,
            expected_policy_hash: &input.expected_policy_hash,
            expected_draft_revision: input.expected_draft_revision,
            expected_assessment_id: input.expected_assessment_id.as_deref(),
            idempotency_key: &input.idempotency_key,
        })
        .map_err(|error| self.action_error(&input.candidate_id, error.to_string()))?;
        action_receipt(
            &connection,
            &input.candidate_id,
            &outcome.decision_id,
            outcome.duplicate,
        )
    }

    pub(crate) fn approve(&self, input: CuratorApproveInput, now_ms: i64) -> CuratorApiResult {
        let workspace = self.workspace(&input.candidate_id)?;
        let mut connection = self.connection()?;
        let mut repository = SqliteCuratorRepository::new(&mut connection);
        let overlay = SkillApiCuratorApplication::new(&self.skills, Some(&workspace));
        let outcome = CuratorApplicationService::new(
            &mut repository,
            &overlay,
            CuratorTrustedActor::local_interactive_user(now_ms),
        )
        .approve(CuratorApprovalRequest {
            candidate_id: &input.candidate_id,
            expected_candidate_revision: input.expected_candidate_revision,
            confirmed_preview_hash: &input.confirmed_preview_hash,
            confirmed_effective_diff_hash: &input.confirmed_effective_diff_hash,
            idempotency_key: &input.idempotency_key,
        })
        .map_err(|error| self.action_error(&input.candidate_id, error.to_string()))?;
        let application = match outcome {
            CuratorApplicationOutcome::Applied(value)
            | CuratorApplicationOutcome::Failed(value) => value,
        };
        let result = application_result(&connection, application);
        self.dispatch_after(result, now_ms)
    }

    pub(crate) fn retry(&self, input: CuratorRetryInput, now_ms: i64) -> CuratorApiResult {
        validate_key(&input.idempotency_key)?;
        let workspace = self.workspace(&input.candidate_id)?;
        let mut connection = self.connection()?;
        let mut repository = SqliteCuratorRepository::new(&mut connection);
        let overlay = SkillApiCuratorApplication::new(&self.skills, Some(&workspace));
        CuratorApplicationService::new(
            &mut repository,
            &overlay,
            CuratorTrustedActor::local_interactive_user(now_ms),
        )
        .prepare_retry(&input.candidate_id, input.expected_candidate_revision)
        .map_err(|error| self.action_error(&input.candidate_id, error.to_string()))?;
        action_receipt(
            &connection,
            &input.candidate_id,
            &input.idempotency_key,
            false,
        )
    }

    pub(crate) fn update_policy(&self, input: CuratorPolicyInput, now_ms: i64) -> CuratorApiResult {
        let mut connection = self.connection()?;
        let mut repository = SqliteCuratorRepository::new(&mut connection);
        let outcome = repository
            .update_policy(
                &input.workspace_id,
                input.expected_revision,
                input.policy.into(),
                now_ms,
            )
            .map_err(policy_error)?;
        to_value(outcome.policy).map_err(|_| storage())
    }
}
