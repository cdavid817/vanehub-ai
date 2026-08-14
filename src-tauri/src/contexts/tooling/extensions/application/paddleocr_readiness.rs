use super::{ExtensionApplicationError, ExtensionRepository, PADDLEOCR_INFERENCE_PROTOCOL_VERSION};
use crate::contexts::tooling::extensions::domain::ExtensionFrameworkId;
use std::sync::Arc;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PaddleOcrInferenceInspection {
    pub(crate) protocol_version: String,
    pub(crate) engine_version: String,
    pub(crate) interpreter_ready: bool,
    pub(crate) worker_ready: bool,
    pub(crate) renderer_ready: bool,
    pub(crate) renderer_version: Option<String>,
}

pub(crate) trait PaddleOcrInferenceInspectionPort: Send + Sync {
    fn inspect(
        &self,
        install_path: &str,
    ) -> Result<PaddleOcrInferenceInspection, ExtensionApplicationError>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PaddleOcrReadinessReason {
    Ready,
    NotInstalled,
    Disabled,
    InstallationIncomplete,
    PdfRendererUnavailable,
    ProtocolUnavailable,
    VersionMismatch,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PaddleOcrInferenceReadiness {
    pub(crate) ready: bool,
    pub(crate) reason: PaddleOcrReadinessReason,
    pub(crate) engine_version: Option<String>,
}

pub(crate) struct PaddleOcrReadinessService {
    repository: Arc<dyn ExtensionRepository>,
    inspection: Arc<dyn PaddleOcrInferenceInspectionPort>,
}

impl PaddleOcrReadinessService {
    pub(crate) fn new(
        repository: Arc<dyn ExtensionRepository>,
        inspection: Arc<dyn PaddleOcrInferenceInspectionPort>,
    ) -> Self {
        Self {
            repository,
            inspection,
        }
    }

    pub(crate) fn check(&self) -> Result<PaddleOcrInferenceReadiness, ExtensionApplicationError> {
        let state = self
            .repository
            .list_states()?
            .into_iter()
            .find(|state| state.framework_id == ExtensionFrameworkId::Paddleocr);
        let Some(state) = state else {
            return Ok(not_ready(PaddleOcrReadinessReason::NotInstalled));
        };
        if !state.installed {
            return Ok(not_ready(PaddleOcrReadinessReason::NotInstalled));
        }
        if !state.enabled {
            return Ok(not_ready(PaddleOcrReadinessReason::Disabled));
        }
        let Some(install_path) = state.install_path.as_deref() else {
            return Ok(not_ready(PaddleOcrReadinessReason::InstallationIncomplete));
        };
        let inspection = self.inspection.inspect(install_path)?;
        if !inspection.interpreter_ready || !inspection.worker_ready {
            return Ok(not_ready(PaddleOcrReadinessReason::InstallationIncomplete));
        }
        if !inspection.renderer_ready || inspection.renderer_version.is_none() {
            return Ok(not_ready(PaddleOcrReadinessReason::PdfRendererUnavailable));
        }
        if inspection.protocol_version != PADDLEOCR_INFERENCE_PROTOCOL_VERSION {
            return Ok(not_ready(PaddleOcrReadinessReason::ProtocolUnavailable));
        }
        if !reviewed_engine_version(&inspection.engine_version) {
            return Ok(not_ready(PaddleOcrReadinessReason::VersionMismatch));
        }
        Ok(PaddleOcrInferenceReadiness {
            ready: true,
            reason: PaddleOcrReadinessReason::Ready,
            engine_version: Some(inspection.engine_version),
        })
    }
}

fn not_ready(reason: PaddleOcrReadinessReason) -> PaddleOcrInferenceReadiness {
    PaddleOcrInferenceReadiness {
        ready: false,
        reason,
        engine_version: None,
    }
}

fn reviewed_engine_version(version: &str) -> bool {
    version
        .split_once('.')
        .is_some_and(|(major, rest)| major == "3" && !rest.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contexts::tooling::extensions::domain::{
        definition, ExtensionFrameworkState, ExtensionLifecycleStatus,
    };
    use std::sync::Mutex;

    struct Repository(ExtensionFrameworkState);

    impl ExtensionRepository for Repository {
        fn list_states(&self) -> Result<Vec<ExtensionFrameworkState>, ExtensionApplicationError> {
            Ok(vec![self.0.clone()])
        }

        fn record_transition(
            &self,
            _: ExtensionFrameworkId,
            _: ExtensionLifecycleStatus,
            _: &str,
            _: &str,
        ) -> Result<(), ExtensionApplicationError> {
            panic!("readiness must not mutate")
        }
        fn record_installation(
            &self,
            _: ExtensionFrameworkId,
            _: &super::super::InstalledExtension,
            _: &str,
        ) -> Result<(), ExtensionApplicationError> {
            panic!("readiness must not mutate")
        }
        fn record_removal(
            &self,
            _: ExtensionFrameworkId,
            _: &str,
        ) -> Result<(), ExtensionApplicationError> {
            panic!("readiness must not mutate")
        }
        fn apply_enablement(
            &self,
            _: &crate::contexts::tooling::extensions::domain::EnablementPlan,
            _: &str,
        ) -> Result<(), ExtensionApplicationError> {
            panic!("readiness must not mutate")
        }
        fn record_runtime_observation(
            &self,
            _: ExtensionFrameworkId,
            _: &crate::contexts::tooling::extensions::domain::ExtensionRuntimeObservation,
            _: &str,
        ) -> Result<(), ExtensionApplicationError> {
            panic!("readiness must not mutate")
        }
        fn record_self_test(
            &self,
            _: ExtensionFrameworkId,
            _: &str,
        ) -> Result<(), ExtensionApplicationError> {
            panic!("readiness must not mutate")
        }
        fn record_failure(
            &self,
            _: ExtensionFrameworkId,
            _: &str,
            _: &str,
        ) -> Result<(), ExtensionApplicationError> {
            panic!("readiness must not mutate")
        }
    }

    struct Inspection(Mutex<u32>, PaddleOcrInferenceInspection);

    impl PaddleOcrInferenceInspectionPort for Inspection {
        fn inspect(
            &self,
            _: &str,
        ) -> Result<PaddleOcrInferenceInspection, ExtensionApplicationError> {
            *self.0.lock().expect("calls") += 1;
            Ok(self.1.clone())
        }
    }

    fn state(enabled: bool) -> ExtensionFrameworkState {
        let mut state =
            ExtensionFrameworkState::seeded(definition(ExtensionFrameworkId::Paddleocr));
        state.installed = true;
        state.enabled = enabled;
        state.install_path = Some("C:/managed/paddleocr".to_owned());
        state.installed_version = Some("3.2.0".to_owned());
        state
    }

    fn inspection() -> PaddleOcrInferenceInspection {
        PaddleOcrInferenceInspection {
            protocol_version: PADDLEOCR_INFERENCE_PROTOCOL_VERSION.to_owned(),
            engine_version: "3.2.0".to_owned(),
            interpreter_ready: true,
            worker_ready: true,
            renderer_ready: true,
            renderer_version: Some("1.0.0".to_owned()),
        }
    }

    #[test]
    fn readiness_is_read_only_and_requires_enabled_compatible_inference() {
        let disabled_inspection = Arc::new(Inspection(Mutex::new(0), inspection()));
        let disabled = PaddleOcrReadinessService::new(
            Arc::new(Repository(state(false))),
            disabled_inspection.clone(),
        );
        assert_eq!(
            disabled.check().expect("disabled").reason,
            PaddleOcrReadinessReason::Disabled
        );
        assert_eq!(*disabled_inspection.0.lock().expect("calls"), 0);

        let ready = PaddleOcrReadinessService::new(
            Arc::new(Repository(state(true))),
            Arc::new(Inspection(Mutex::new(0), inspection())),
        );
        assert!(ready.check().expect("ready").ready);

        let mut incompatible = inspection();
        incompatible.protocol_version = "vanehub.paddleocr.inference.v2".to_owned();
        let mismatched = PaddleOcrReadinessService::new(
            Arc::new(Repository(state(true))),
            Arc::new(Inspection(Mutex::new(0), incompatible)),
        );
        let readiness = mismatched.check().expect("mismatch");
        assert!(!readiness.ready);
        assert_eq!(
            readiness.reason,
            PaddleOcrReadinessReason::ProtocolUnavailable
        );
    }
}
