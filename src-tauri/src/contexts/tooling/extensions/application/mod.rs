mod error;
mod models;
mod ocr_admission;
mod ocr_execution;
mod ocr_media;
mod ocr_result;
mod paddleocr_inference;
mod paddleocr_readiness;
mod ports;
mod service;

#[cfg(test)]
mod tests;

pub(crate) use error::ExtensionApplicationError;
pub(crate) use models::{
    ExtensionExecutionLog, ExtensionInstallPreview, ExtensionLogEvent, ExtensionLogLevel,
    ExtensionOperationRequest, ExtensionOperationResult, ExtensionOverview, InstalledExtension,
    PreparedExtensionOperation, StartedExtensionOperation,
};
#[allow(unused_imports)]
pub(crate) use ocr_admission::{
    AdmittedOcrArtifact, ArtifactOcrSourceAdapter, OcrAdmissionError, OcrAdmissionLimits,
    OcrArtifactAdmissionService, OcrArtifactRequest, OcrArtifactSourcePort, OcrStagingPort,
};
#[allow(unused_imports)]
pub(crate) use ocr_execution::{
    ManagedOcrExecutionService, ManagedPdfRendererPort, OcrExecutionError, PaddleOcrWorkerPort,
    RenderedOcrPage,
};
#[allow(unused_imports)]
pub(crate) use ocr_result::{
    normalize_ocr_result, NormalizedOcrResult, OcrResultBlock, OCR_RESULT_CONTRACT_VERSION,
};
#[allow(unused_imports)]
pub(crate) use paddleocr_inference::{
    PaddleOcrEngineBlock, PaddleOcrEngineResult, PaddleOcrInferenceInput, PaddleOcrInferenceLimits,
    PaddleOcrInferenceRequest, PaddleOcrInputKind, PaddleOcrPoint, PaddleOcrProtocolError,
    PADDLEOCR_INFERENCE_PROTOCOL_VERSION,
};
#[allow(unused_imports)]
pub(crate) use paddleocr_readiness::{
    PaddleOcrInferenceInspection, PaddleOcrInferenceInspectionPort, PaddleOcrInferenceReadiness,
    PaddleOcrReadinessReason, PaddleOcrReadinessService,
};
pub(crate) use ports::{
    ExtensionClockPort, ExtensionEnvironmentPort, ExtensionInstallationPort, ExtensionLoggingPort,
    ExtensionMutationPort, ExtensionOperationPort, ExtensionRepository, ExtensionRuntimePort,
    InstallationInspection,
};
pub(crate) use service::ExtensionApplicationService;
