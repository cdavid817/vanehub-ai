mod installation_adapter;
mod managed_paddleocr_worker;
mod managed_pdfium_renderer;
mod ocr_native_tool_adapter;
mod ocr_native_tool_support;
mod ocr_staging;
mod paddleocr_readiness;
mod pdfium_protocol;
mod process_adapter;
mod runtime_adapter;
mod runtime_support;
mod sqlite_repository;

pub(crate) use installation_adapter::{ManagedExtensionInstallation, SystemExtensionEnvironment};
pub(crate) use managed_paddleocr_worker::ManagedPaddleOcrWorker;
pub(crate) use managed_pdfium_renderer::ManagedPdfiumRenderer;
pub(crate) use ocr_native_tool_adapter::OcrNativeToolAdapter;
pub(crate) use ocr_staging::OwnedOcrStagingAdapter;
pub(crate) use paddleocr_readiness::ManagedPaddleOcrReadinessInspector;
pub(crate) use runtime_adapter::OwnedExtensionRuntime;
pub(crate) use runtime_support::{
    ExtensionOperationAdapter, SystemExtensionClock, UnifiedExtensionLoggingAdapter,
};
pub(crate) use sqlite_repository::{apply_schema, SqliteExtensionRepository};
