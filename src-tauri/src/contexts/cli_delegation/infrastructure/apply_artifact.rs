use crate::contexts::artifacts::application::ArtifactService;
use crate::contexts::cli_delegation::application::{
    DelegationApplyArtifactEvidence, DelegationApplyArtifactPort, DelegationChangeFile,
    DelegationChangeSetCapture,
};
use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::sync::Arc;

const MAX_CHANGESET_BYTES: usize = 48 * 1024 * 1024;
const CHUNK_BYTES: usize = 1024 * 1024;

pub(super) struct ApplyArtifactAdapter {
    artifacts: Arc<ArtifactService>,
}

impl ApplyArtifactAdapter {
    pub(super) fn new(artifacts: Arc<ArtifactService>) -> Self {
        Self { artifacts }
    }

    fn load_bytes(&self, id: &str) -> Result<(String, Vec<u8>), ()> {
        let metadata = self.artifacts.metadata(id).map_err(|_| ())?;
        let size = usize::try_from(metadata.size_bytes).map_err(|_| ())?;
        if metadata.media_type != "application/json" || size > MAX_CHANGESET_BYTES {
            return Err(());
        }
        let mut bytes = Vec::with_capacity(size);
        let mut offset = 0_u64;
        loop {
            let chunk = self
                .artifacts
                .download_chunk(id, offset, CHUNK_BYTES)
                .map_err(|_| ())?;
            if chunk.content_hash != metadata.content_hash || chunk.offset != offset {
                return Err(());
            }
            bytes.extend_from_slice(&chunk.bytes);
            match chunk.next_offset {
                Some(next) if next > offset && bytes.len() <= MAX_CHANGESET_BYTES => offset = next,
                Some(_) => return Err(()),
                None => break,
            }
        }
        if bytes.len() != size || sha256(&bytes) != metadata.content_hash {
            return Err(());
        }
        Ok((metadata.content_hash, bytes))
    }
}

impl DelegationApplyArtifactPort for ApplyArtifactAdapter {
    fn load_apply_evidence(
        &self,
        artifact_id: &str,
    ) -> Result<DelegationApplyArtifactEvidence, ()> {
        let (content_hash, bytes) = self.load_bytes(artifact_id)?;
        let manifest: ApplyManifest = serde_json::from_slice(&bytes).map_err(|_| ())?;
        if manifest.schema_version != 1 || manifest.repository_identity.trim().is_empty() {
            return Err(());
        }
        let patch = STANDARD.decode(manifest.patch_base64).map_err(|_| ())?;
        if sha256(&patch) != manifest.diff_hash {
            return Err(());
        }
        Ok(DelegationApplyArtifactEvidence {
            artifact_id: artifact_id.to_owned(),
            content_hash,
            repository_identity: manifest.repository_identity,
            capture: DelegationChangeSetCapture {
                base_commit: manifest.base_commit,
                files: manifest.files,
                canonical_patch: patch,
                diff_hash: manifest.diff_hash,
            },
            applyable: manifest.applyable,
            integrity_verified: true,
        })
    }
}

#[derive(Deserialize)]
struct ApplyManifest {
    schema_version: u16,
    repository_identity: String,
    base_commit: String,
    files: Vec<DelegationChangeFile>,
    patch_base64: String,
    diff_hash: String,
    applyable: bool,
}

fn sha256(bytes: &[u8]) -> String {
    let hex = Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    format!("sha256:{hex}")
}
