use super::ocr_media::validate_image_dimensions;
use crate::contexts::artifacts::api::{ArtifactDescriptor, ArtifactService};
use std::path::{Path, PathBuf};
use std::sync::Arc;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct OcrAdmissionLimits {
    pub(crate) input_bytes: u64,
    pub(crate) image_width: u32,
    pub(crate) image_height: u32,
    pub(crate) image_pixels: u64,
    pub(crate) pdf_pages: u32,
    pub(crate) rendered_page_bytes: u64,
}

impl OcrAdmissionLimits {
    pub(crate) const HARD_CEILING: Self = Self {
        input_bytes: 64 * 1024 * 1024,
        image_width: 16_384,
        image_height: 16_384,
        image_pixels: 80_000_000,
        pdf_pages: 32,
        rendered_page_bytes: 32 * 1024 * 1024,
    };
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct OcrArtifactRequest {
    pub(crate) artifact_id: String,
    pub(crate) pages: Vec<u32>,
    pub(crate) limits: OcrAdmissionLimits,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AdmittedOcrArtifact {
    pub(crate) artifact_id: String,
    pub(crate) content_hash: String,
    pub(crate) media_type: String,
    pub(crate) staged_path: PathBuf,
    pub(crate) pages: Vec<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum OcrAdmissionError {
    InvalidRequest,
    ArtifactUnavailable,
    UnsupportedMediaType,
    IntegrityFailure,
    LimitExceeded,
    UnsafeWorkspace,
}

pub(crate) trait OcrArtifactSourcePort: Send + Sync {
    fn resolve_verified(
        &self,
        artifact_id: &str,
    ) -> Result<(ArtifactDescriptor, Vec<u8>), OcrAdmissionError>;
}

pub(crate) trait OcrStagingPort: Send + Sync {
    fn stage_read_only(
        &self,
        staging_root: &Path,
        file_name: &str,
        bytes: &[u8],
    ) -> Result<PathBuf, OcrAdmissionError>;
}

pub(crate) struct OcrArtifactAdmissionService {
    source: Arc<dyn OcrArtifactSourcePort>,
    staging: Arc<dyn OcrStagingPort>,
}

impl OcrArtifactAdmissionService {
    pub(crate) fn new(
        source: Arc<dyn OcrArtifactSourcePort>,
        staging: Arc<dyn OcrStagingPort>,
    ) -> Self {
        Self { source, staging }
    }

    pub(crate) fn admit(
        &self,
        request: OcrArtifactRequest,
        staging_root: &Path,
    ) -> Result<AdmittedOcrArtifact, OcrAdmissionError> {
        validate_request(&request)?;
        let (artifact, bytes) = self.source.resolve_verified(&request.artifact_id)?;
        if artifact.size_bytes != bytes.len() as u64
            || artifact.size_bytes > request.limits.input_bytes
        {
            return Err(OcrAdmissionError::LimitExceeded);
        }
        let extension = admitted_extension(&artifact.media_type, &bytes)?;
        validate_image_dimensions(&artifact.media_type, &bytes, request.limits)?;
        if artifact.media_type == "application/pdf" && request.pages.is_empty() {
            return Err(OcrAdmissionError::InvalidRequest);
        }
        let staged_path =
            self.staging
                .stage_read_only(staging_root, &format!("source.{extension}"), &bytes)?;
        let content_hash = artifact
            .content_hash
            .strip_prefix("sha256:")
            .filter(|hash| hash.len() == 64 && hash.bytes().all(|byte| byte.is_ascii_hexdigit()))
            .ok_or(OcrAdmissionError::IntegrityFailure)?
            .to_ascii_lowercase();
        Ok(AdmittedOcrArtifact {
            artifact_id: artifact.id,
            content_hash,
            media_type: artifact.media_type,
            staged_path,
            pages: request.pages,
        })
    }
}

pub(crate) struct ArtifactOcrSourceAdapter {
    artifacts: Arc<ArtifactService>,
}

impl ArtifactOcrSourceAdapter {
    pub(crate) fn new(artifacts: Arc<ArtifactService>) -> Self {
        Self { artifacts }
    }
}

impl OcrArtifactSourcePort for ArtifactOcrSourceAdapter {
    fn resolve_verified(
        &self,
        artifact_id: &str,
    ) -> Result<(ArtifactDescriptor, Vec<u8>), OcrAdmissionError> {
        let artifact = self
            .artifacts
            .metadata(artifact_id)
            .map_err(|_| OcrAdmissionError::ArtifactUnavailable)?;
        let capacity =
            usize::try_from(artifact.size_bytes).map_err(|_| OcrAdmissionError::LimitExceeded)?;
        let mut admitted = Vec::with_capacity(capacity);
        let mut offset = 0_u64;
        loop {
            let chunk = self
                .artifacts
                .download_chunk(artifact_id, offset, 1_048_576)
                .map_err(|_| OcrAdmissionError::IntegrityFailure)?;
            if chunk.offset != offset || chunk.content_hash != artifact.content_hash {
                return Err(OcrAdmissionError::IntegrityFailure);
            }
            admitted.extend_from_slice(&chunk.bytes);
            let Some(next) = chunk.next_offset else {
                break;
            };
            if next <= offset || next > artifact.size_bytes {
                return Err(OcrAdmissionError::IntegrityFailure);
            }
            offset = next;
        }
        Ok((artifact, admitted))
    }
}

fn validate_request(request: &OcrArtifactRequest) -> Result<(), OcrAdmissionError> {
    let ceiling = OcrAdmissionLimits::HARD_CEILING;
    let limits = request.limits;
    if request.artifact_id.is_empty()
        || request.artifact_id.len() > 128
        || request.pages.len() > ceiling.pdf_pages as usize
        || request.pages.contains(&0)
        || request.pages.windows(2).any(|pages| pages[0] >= pages[1])
        || limits.input_bytes == 0
        || limits.input_bytes > ceiling.input_bytes
        || limits.image_width == 0
        || limits.image_width > ceiling.image_width
        || limits.image_height == 0
        || limits.image_height > ceiling.image_height
        || limits.image_pixels == 0
        || limits.image_pixels > ceiling.image_pixels
        || limits.pdf_pages == 0
        || limits.pdf_pages > ceiling.pdf_pages
        || limits.rendered_page_bytes == 0
        || limits.rendered_page_bytes > ceiling.rendered_page_bytes
    {
        Err(OcrAdmissionError::InvalidRequest)
    } else {
        Ok(())
    }
}

fn admitted_extension(media_type: &str, bytes: &[u8]) -> Result<&'static str, OcrAdmissionError> {
    match media_type {
        "image/png" if bytes.starts_with(b"\x89PNG\r\n\x1a\n") => Ok("png"),
        "image/jpeg" if bytes.starts_with(&[0xff, 0xd8, 0xff]) => Ok("jpg"),
        "application/pdf" if bytes.starts_with(b"%PDF-") => Ok("pdf"),
        "image/png" | "image/jpeg" | "application/pdf" => Err(OcrAdmissionError::IntegrityFailure),
        _ => Err(OcrAdmissionError::UnsupportedMediaType),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contexts::artifacts::api::{
        ArtifactCreator, ArtifactEvidenceKind, ArtifactVisibility,
    };

    struct Source(ArtifactDescriptor, Vec<u8>);

    struct Staging;

    impl OcrStagingPort for Staging {
        fn stage_read_only(
            &self,
            staging_root: &Path,
            file_name: &str,
            _bytes: &[u8],
        ) -> Result<PathBuf, OcrAdmissionError> {
            Ok(staging_root.join(file_name))
        }
    }

    impl OcrArtifactSourcePort for Source {
        fn resolve_verified(
            &self,
            _: &str,
        ) -> Result<(ArtifactDescriptor, Vec<u8>), OcrAdmissionError> {
            Ok((self.0.clone(), self.1.clone()))
        }
    }

    fn descriptor(media_type: &str, size_bytes: u64) -> ArtifactDescriptor {
        ArtifactDescriptor {
            contract_version: 1,
            id: "artifact-source".to_owned(),
            content_hash: format!("sha256:{}", "a".repeat(64)),
            size_bytes,
            media_type: media_type.to_owned(),
            display_name: "source".to_owned(),
            creator: ArtifactCreator {
                kind: "user".to_owned(),
                id: "user".to_owned(),
            },
            evidence_kind: ArtifactEvidenceKind::HostVerified,
            visibility: ArtifactVisibility::Private,
            source_operation_id: "upload".to_owned(),
            source_artifact_ids: Vec::new(),
            created_at: "now".to_owned(),
            expires_at: None,
        }
    }

    #[test]
    fn admitted_image_is_materialized_read_only_from_verified_artifact_bytes() {
        let mut bytes = b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR".to_vec();
        bytes.extend_from_slice(&16_u32.to_be_bytes());
        bytes.extend_from_slice(&8_u32.to_be_bytes());
        let service = OcrArtifactAdmissionService::new(
            Arc::new(Source(descriptor("image/png", bytes.len() as u64), bytes)),
            Arc::new(Staging),
        );
        let admitted = service
            .admit(
                OcrArtifactRequest {
                    artifact_id: "artifact-source".to_owned(),
                    pages: Vec::new(),
                    limits: OcrAdmissionLimits::HARD_CEILING,
                },
                Path::new("owned-staging"),
            )
            .expect("admit");
        assert_eq!(admitted.staged_path, Path::new("owned-staging/source.png"));
    }

    #[test]
    fn media_mismatch_and_unordered_pdf_pages_fail_closed() {
        let service = OcrArtifactAdmissionService::new(
            Arc::new(Source(descriptor("image/png", 4), b"nope".to_vec())),
            Arc::new(Staging),
        );
        let error = service.admit(
            OcrArtifactRequest {
                artifact_id: "artifact-source".to_owned(),
                pages: Vec::new(),
                limits: OcrAdmissionLimits::HARD_CEILING,
            },
            Path::new("owned-staging"),
        );
        assert_eq!(error, Err(OcrAdmissionError::IntegrityFailure));
        let mut invalid = OcrArtifactRequest {
            artifact_id: "artifact-source".to_owned(),
            pages: vec![2, 1],
            limits: OcrAdmissionLimits::HARD_CEILING,
        };
        invalid.limits.pdf_pages = 2;
        assert_eq!(
            validate_request(&invalid),
            Err(OcrAdmissionError::InvalidRequest)
        );
    }
}
