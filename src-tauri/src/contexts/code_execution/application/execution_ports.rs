use super::{CodeOutputArtifact, CodeRuntime, CodeServiceError, RuntimeVersion};
use std::path::PathBuf;

pub(crate) trait CodeRuntimePort: Send + Sync {
    fn resolve_reviewed(
        &self,
        runtime: CodeRuntime,
    ) -> Result<(PathBuf, RuntimeVersion), CodeServiceError>;
}

pub(crate) trait CodeOutputArtifactPort: Send + Sync {
    fn seal_output(
        &self,
        execution_id: &str,
        relative_name: &str,
        media_type: &str,
        bytes: &[u8],
    ) -> Result<CodeOutputArtifact, CodeServiceError>;
}
