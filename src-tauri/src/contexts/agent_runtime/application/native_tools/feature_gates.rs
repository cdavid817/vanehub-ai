#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct OnePieceToolFeatureGates {
    browser: bool,
    web: bool,
    code_execution: bool,
    ocr: bool,
    artifact_read: bool,
    artifact_publish: bool,
    artifact_download: bool,
    delegation_analyze: bool,
    delegation_edit: bool,
    delegation_apply: bool,
    subagent: bool,
}

impl OnePieceToolFeatureGates {
    pub(crate) fn from_environment() -> Self {
        Self::from_lookup(|name| std::env::var(name).ok())
    }

    #[cfg(test)]
    pub(crate) fn rollout_defaults() -> Self {
        Self::from_lookup(|_| None)
    }

    fn from_lookup(mut lookup: impl FnMut(&str) -> Option<String>) -> Self {
        Self {
            browser: enabled(&mut lookup, "VANEHUB_ONEPIECE_BROWSER_ENABLED", false),
            web: enabled(&mut lookup, "VANEHUB_ONEPIECE_WEB_ENABLED", false),
            code_execution: enabled(
                &mut lookup,
                "VANEHUB_ONEPIECE_CODE_EXECUTION_ENABLED",
                false,
            ),
            ocr: enabled(&mut lookup, "VANEHUB_ONEPIECE_OCR_ENABLED", false),
            artifact_read: enabled(&mut lookup, "VANEHUB_ONEPIECE_ARTIFACT_READ_ENABLED", true),
            artifact_publish: enabled(
                &mut lookup,
                "VANEHUB_ONEPIECE_ARTIFACT_PUBLISH_ENABLED",
                false,
            ),
            artifact_download: enabled(
                &mut lookup,
                "VANEHUB_ONEPIECE_ARTIFACT_DOWNLOAD_ENABLED",
                false,
            ),
            delegation_analyze: enabled(
                &mut lookup,
                "VANEHUB_ONEPIECE_DELEGATION_ANALYZE_ENABLED",
                false,
            ),
            delegation_edit: enabled(
                &mut lookup,
                "VANEHUB_ONEPIECE_DELEGATION_EDIT_ENABLED",
                false,
            ),
            delegation_apply: enabled(
                &mut lookup,
                "VANEHUB_ONEPIECE_DELEGATION_APPLY_ENABLED",
                false,
            ),
            subagent: enabled(&mut lookup, "VANEHUB_ONEPIECE_SUBAGENT_ENABLED", false),
        }
    }

    pub(crate) fn all_enabled() -> Self {
        Self {
            browser: true,
            web: true,
            code_execution: true,
            ocr: true,
            artifact_read: true,
            artifact_publish: true,
            artifact_download: true,
            delegation_analyze: true,
            delegation_edit: true,
            delegation_apply: true,
            subagent: true,
        }
    }

    pub(crate) fn enabled(&self, capability: &str, mode: &str) -> bool {
        match (capability, mode) {
            ("browser", "read" | "execute") => self.browser,
            ("web", "read") => self.web,
            ("code_execution", "execute") => self.code_execution,
            ("ocr", "read") => self.ocr,
            ("artifact", "read") => self.artifact_read,
            ("artifact", "publish") => self.artifact_publish,
            ("artifact", "download") => self.artifact_download,
            ("delegation", "read") => self.delegation_analyze,
            ("delegation", "write") => self.delegation_edit,
            ("delegation", "apply") => self.delegation_apply,
            ("subagent", "delegate") => self.subagent,
            _ => false,
        }
    }

    pub(crate) fn tool_enabled(&self, name: &str) -> bool {
        match name {
            "browser" => self.browser,
            "web_search" | "web_fetch" => self.web,
            "code_execution" => self.code_execution,
            "ocr" => self.ocr,
            "artifact" => self.artifact_read || self.artifact_publish,
            "delegate_cli" => self.delegation_analyze || self.delegation_edit,
            "apply_delegation_changes" => self.delegation_apply,
            "delegate_subagent" => self.subagent,
            _ => true,
        }
    }

    pub(crate) fn operation_enabled(&self, operation: super::NativeToolOperation) -> bool {
        use super::NativeToolOperation;
        match operation {
            NativeToolOperation::BrowserRead | NativeToolOperation::BrowserInteract => self.browser,
            NativeToolOperation::WebSearch | NativeToolOperation::WebFetch => self.web,
            NativeToolOperation::CodeExecute => self.code_execution,
            NativeToolOperation::OcrRead => self.ocr,
            NativeToolOperation::ArtifactRead => self.artifact_read,
            NativeToolOperation::ArtifactPublish => self.artifact_publish,
            NativeToolOperation::DelegationAnalyze => self.delegation_analyze,
            NativeToolOperation::DelegationEdit => self.delegation_edit,
            NativeToolOperation::DelegationApply => self.delegation_apply,
            NativeToolOperation::SubagentDelegate => self.subagent,
        }
    }
}

fn enabled(lookup: &mut impl FnMut(&str) -> Option<String>, name: &str, default: bool) -> bool {
    lookup(name).map_or(default, |value| value == "1")
}

#[cfg(test)]
#[path = "feature_gates_tests.rs"]
mod tests;
