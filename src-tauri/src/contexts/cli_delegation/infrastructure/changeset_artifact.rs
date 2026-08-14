use crate::contexts::artifacts::application::{
    ArtifactCreateRequest, ArtifactCreator, ArtifactEvidenceKind, ArtifactService,
    ArtifactVisibility,
};
use crate::contexts::cli_delegation::application::DelegationChangeSetArtifactPort;
use serde_json::Value;
use std::sync::Arc;

pub(crate) struct ArtifactChangeSetAdapter {
    artifacts: Arc<ArtifactService>,
}

impl ArtifactChangeSetAdapter {
    pub(crate) fn new(artifacts: Arc<ArtifactService>) -> Self {
        Self { artifacts }
    }
}

impl DelegationChangeSetArtifactPort for ArtifactChangeSetAdapter {
    fn seal_json(
        &self,
        operation_id: &str,
        attempt_id: &str,
        created_at: &str,
        value: &Value,
    ) -> Result<(String, String), ()> {
        let artifact = self
            .artifacts
            .create_json(
                ArtifactCreateRequest {
                    operation_id: operation_id.to_string(),
                    display_name: format!("delegation-changeset-{attempt_id}.json"),
                    media_type: "application/json".to_string(),
                    creator: ArtifactCreator {
                        kind: "delegation-attempt".to_string(),
                        id: attempt_id.to_string(),
                    },
                    evidence_kind: ArtifactEvidenceKind::HostVerified,
                    visibility: ArtifactVisibility::Private,
                    source_artifact_ids: Vec::new(),
                    created_at: created_at.to_string(),
                    expires_at: None,
                },
                value,
            )
            .map_err(|_| ())?;
        Ok((artifact.id, artifact.content_hash))
    }
}
