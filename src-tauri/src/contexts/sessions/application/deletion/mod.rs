//! Session deletion with optional, conservative worktree cleanup.

mod coordinator;
mod models;
mod policy;
mod ports;
#[cfg(test)]
mod tests;

pub(crate) use coordinator::{SessionDeletionCoordinator, SessionDeletionPorts};
pub(crate) use models::{
    error_code as deletion_error_code, DeletionChangeSummary, DeletionCheckCompleteness,
    DeletionGroupResult, DeletionGroupStatus, DeletionIgnoredSample, DeletionIgnoredSummary,
    DeletionOutcome, DeletionPhase, DeletionRuntimeEffect, ExecuteSessionDeletionRequest,
    PreviewSessionDeletionRequest, RetrySessionDeletionRequest, SessionDbEffect,
    SessionDeletionHandle, SessionDeletionOperation, SessionDeletionPreview,
    WorktreeDeletionPolicy, WorktreeEffect,
};
#[cfg(test)]
pub(crate) use ports::NewDeletionGroup;
pub(crate) use ports::{
    DeletionClockPort, DeletionEventPort, DeletionIdPort, DeletionJournalPort, DeletionOwner,
    DeletionOwnerPort, DeletionPreviewStore, DeletionReferencePort, DeletionWorkspacePort,
    GateOutcome, GateToken, GroupCompletion, GroupPatch, GroupSnapshot, JournalCreateOutcome,
    NewDeletionOperation, ObservationView, OperationOwnership, OperationPatch, QuiescenceReport,
    ReferenceInput, ReferenceScan, RemovalOutcomeView, RemovalReportView, ResolvedWorktree,
    SessionDeletionClaim, SessionDeletionRuntimePort, SessionExecutionAdmissionPort,
    SessionReference, StoredPreview, Tri, WorktreeAssessment, WorktreeIdentityView,
};

impl SessionExecutionAdmissionPort for SessionDeletionCoordinator {
    fn ensure_session_admits_execution(
        &self,
        session_id: &str,
    ) -> Result<(), crate::contexts::sessions::application::SessionsApplicationError> {
        SessionDeletionCoordinator::ensure_session_admits_execution(self, session_id)
    }
}
