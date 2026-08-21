use crate::contexts::artifacts::api::ArtifactService;
use crate::contexts::cli_delegation::application::{
    DelegationChangeSetPayload, DelegationChangeSetReviewPort,
};
use std::sync::Arc;

const READ_CHUNK_BYTES: usize = 1024 * 1024;

pub(crate) struct ArtifactChangeSetReviewAdapter {
    artifacts: Arc<ArtifactService>,
}

impl ArtifactChangeSetReviewAdapter {
    pub(crate) fn new(artifacts: Arc<ArtifactService>) -> Self {
        Self { artifacts }
    }
}

impl DelegationChangeSetReviewPort for ArtifactChangeSetReviewAdapter {
    fn load(&self, artifact_id: &str, max_bytes: usize) -> Result<DelegationChangeSetPayload, ()> {
        let metadata = self.artifacts.metadata(artifact_id).map_err(|_| ())?;
        if metadata.media_type != "application/json"
            || usize::try_from(metadata.size_bytes).map_err(|_| ())? > max_bytes
        {
            return Err(());
        }
        let capacity = usize::try_from(metadata.size_bytes).map_err(|_| ())?;
        let mut bytes = Vec::with_capacity(capacity);
        let mut offset = 0_u64;
        loop {
            let chunk = self
                .artifacts
                .download_chunk(artifact_id, offset, READ_CHUNK_BYTES)
                .map_err(|_| ())?;
            if chunk.content_hash != metadata.content_hash || chunk.offset != offset {
                return Err(());
            }
            bytes.extend_from_slice(&chunk.bytes);
            match chunk.next_offset {
                Some(next) if next > offset && bytes.len() <= max_bytes => offset = next,
                Some(_) => return Err(()),
                None => break,
            }
        }
        if bytes.len() != capacity {
            return Err(());
        }
        Ok(DelegationChangeSetPayload {
            content_hash: metadata.content_hash,
            bytes,
        })
    }
}
