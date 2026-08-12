#![cfg_attr(not(test), allow(dead_code))]

use super::{
    OverlayEffectivePackageSnapshot, OverlayEffectiveSnapshotPort, OverlayKey,
    OverlayManifestRepository, OverlayManifestSnapshot, OverlayPayloadRepository,
    OverlayValidationDiagnostic, OverlayValidationReason, OverlayValidationTarget,
    SkillApplicationError, SkillClockPort, SkillLoggingPort, SkillResourceDocument,
    MAX_RESOURCE_BYTES, MAX_RESOURCE_CHARACTERS,
};
use crate::contexts::tooling::skills::domain::{
    replay_overlay_scope_chain, EffectiveResourceSource, OverlayIntegrityFailure,
    OverlayMutationState, OverlayScopeReplay, OverlayScopeReplayInput, OverlayScopeReplayStatus,
    SkillId, DEFAULT_OVERLAY_LIMITS,
};
use std::sync::Arc;

const MAXIMUM_SHADOW_SUMMARIES: usize = 8;

#[derive(Clone)]
pub(crate) struct OverlayAppliedSkillSnapshot {
    pub(crate) base: OverlayEffectivePackageSnapshot,
    pub(crate) replay: OverlayScopeReplay,
}

pub(crate) trait OverlayAppliedSkillSnapshotPort: Send + Sync {
    fn read_overlay_applied_package(
        &self,
        canonical_skill_id: &SkillId,
        workspace_identity: Option<&str>,
    ) -> Result<OverlayAppliedSkillSnapshot, SkillApplicationError>;

    fn read_overlay_applied_resource(
        &self,
        canonical_skill_id: &SkillId,
        workspace_identity: Option<&str>,
        expected_revision: &str,
        logical_path: &str,
    ) -> Result<SkillResourceDocument, SkillApplicationError>;

    fn read_overlay_applied_resource_bytes(
        &self,
        canonical_skill_id: &SkillId,
        workspace_identity: Option<&str>,
        expected_revision: &str,
        logical_path: &str,
    ) -> Result<Vec<u8>, SkillApplicationError> {
        self.read_overlay_applied_resource(
            canonical_skill_id,
            workspace_identity,
            expected_revision,
            logical_path,
        )
        .map(|resource| resource.content.into_bytes())
    }
}

#[derive(Clone)]
pub(crate) struct OverlayAppliedSkillSnapshotResolver {
    base: Arc<dyn OverlayEffectiveSnapshotPort>,
    manifests: Arc<dyn OverlayManifestRepository>,
    payloads: Arc<dyn OverlayPayloadRepository>,
    diagnostics: Option<OverlayRuntimeDiagnostics>,
}

#[derive(Clone)]
struct OverlayRuntimeDiagnostics {
    logging: Arc<dyn SkillLoggingPort>,
    clock: Arc<dyn SkillClockPort>,
}

impl OverlayAppliedSkillSnapshotResolver {
    pub(crate) fn new(
        base: Arc<dyn OverlayEffectiveSnapshotPort>,
        manifests: Arc<dyn OverlayManifestRepository>,
        payloads: Arc<dyn OverlayPayloadRepository>,
    ) -> Self {
        Self {
            base,
            manifests,
            payloads,
            diagnostics: None,
        }
    }

    pub(crate) fn with_runtime_diagnostics(
        mut self,
        logging: Arc<dyn SkillLoggingPort>,
        clock: Arc<dyn SkillClockPort>,
    ) -> Self {
        self.diagnostics = Some(OverlayRuntimeDiagnostics { logging, clock });
        self
    }

    fn record_runtime_refusal(
        &self,
        canonical_skill_id: &SkillId,
        reason: OverlayValidationReason,
    ) {
        let Some(diagnostics) = &self.diagnostics else {
            return;
        };
        let diagnostic = OverlayValidationDiagnostic::refused(
            OverlayValidationTarget::Replay,
            canonical_skill_id.as_str(),
            None,
            None,
            reason,
            &[],
            &diagnostics.clock.now(),
        );
        let _ = diagnostics.logging.record_overlay_validation(&diagnostic);
    }

    fn record_replay_diagnostics(&self, canonical_skill_id: &SkillId, replay: &OverlayScopeReplay) {
        for result in replay.scope_results() {
            let reason = match result.status() {
                OverlayScopeReplayStatus::Applied => continue,
                OverlayScopeReplayStatus::Untrusted => OverlayValidationReason::Trust,
                OverlayScopeReplayStatus::NeedsReconciliation => {
                    OverlayValidationReason::StaleWitness
                }
                OverlayScopeReplayStatus::Conflict(_)
                | OverlayScopeReplayStatus::Blocked { .. } => {
                    OverlayValidationReason::ReplayConflict
                }
                OverlayScopeReplayStatus::IntegrityFailure(_) => OverlayValidationReason::Integrity,
            };
            self.record_runtime_refusal(canonical_skill_id, reason);
        }
    }

    fn replay_input<'a>(
        &self,
        snapshot: &'a OverlayManifestSnapshot,
        base: &OverlayEffectivePackageSnapshot,
    ) -> OverlayScopeReplayInput<'a> {
        if let Some(mutation_id) = self.payload_integrity_failure(snapshot) {
            return OverlayScopeReplayInput::integrity_failure(
                &snapshot.document,
                OverlayIntegrityFailure::PayloadHashMismatch { mutation_id },
            );
        }
        if snapshot.document.base_witness.instruction_hash != base.instruction_hash
            || snapshot.document.base_witness.package_hash != base.package_hash
            || snapshot.document.base_witness.base_identity != base.base_identity
        {
            OverlayScopeReplayInput::base_drift(&snapshot.document)
        } else {
            OverlayScopeReplayInput::verified(&snapshot.document)
        }
    }

    fn payload_integrity_failure(&self, snapshot: &OverlayManifestSnapshot) -> Option<String> {
        let key = OverlayKey {
            canonical_skill_id: snapshot.document.canonical_skill_id.clone(),
            scope: snapshot.document.scope(),
            workspace_identity: snapshot.document.workspace_identity().map(str::to_string),
        };
        snapshot
            .document
            .files
            .iter()
            .filter(|file| file.state() == OverlayMutationState::Active)
            .find_map(|file| {
                self.payloads
                    .read_verified(&key, &file.content_hash)
                    .err()
                    .map(|_| file.id.clone())
            })
    }
}

impl OverlayAppliedSkillSnapshotPort for OverlayAppliedSkillSnapshotResolver {
    fn read_overlay_applied_package(
        &self,
        canonical_skill_id: &SkillId,
        workspace_identity: Option<&str>,
    ) -> Result<OverlayAppliedSkillSnapshot, SkillApplicationError> {
        let base = self
            .base
            .read_effective_package(canonical_skill_id, workspace_identity)?;
        let manifests = match self
            .manifests
            .applicable(canonical_skill_id, workspace_identity)
        {
            Ok(manifests) => manifests,
            Err(error) => {
                self.record_runtime_refusal(canonical_skill_id, OverlayValidationReason::Integrity);
                return Err(error);
            }
        };
        let inputs = manifests
            .iter()
            .map(|snapshot| self.replay_input(snapshot, &base))
            .collect::<Vec<_>>();
        let replay = replay_overlay_scope_chain(
            &base.instructions,
            &base.resources,
            &inputs,
            workspace_identity,
            MAXIMUM_SHADOW_SUMMARIES,
        );
        self.record_replay_diagnostics(canonical_skill_id, &replay);
        Ok(OverlayAppliedSkillSnapshot { base, replay })
    }

    fn read_overlay_applied_resource(
        &self,
        canonical_skill_id: &SkillId,
        workspace_identity: Option<&str>,
        expected_revision: &str,
        logical_path: &str,
    ) -> Result<SkillResourceDocument, SkillApplicationError> {
        let snapshot = self.read_overlay_applied_package(canonical_skill_id, workspace_identity)?;
        let resource = effective_resource(&snapshot, expected_revision, logical_path)?;
        if resource.size_bytes > MAX_RESOURCE_BYTES {
            return Err(SkillApplicationError::OversizedResource);
        }
        if matches!(resource.source, EffectiveResourceSource::Overlay { .. })
            && !is_text_overlay_media_type(&resource.media_type)
        {
            return Err(SkillApplicationError::BinaryResource);
        }
        let bytes = self.read_overlay_applied_resource_bytes(
            canonical_skill_id,
            workspace_identity,
            expected_revision,
            logical_path,
        )?;
        let content =
            String::from_utf8(bytes).map_err(|_| SkillApplicationError::BinaryResource)?;
        if content.chars().count() > MAX_RESOURCE_CHARACTERS {
            return Err(SkillApplicationError::OversizedResource);
        }
        Ok(SkillResourceDocument {
            size_bytes: resource.size_bytes,
            content,
        })
    }

    fn read_overlay_applied_resource_bytes(
        &self,
        canonical_skill_id: &SkillId,
        workspace_identity: Option<&str>,
        expected_revision: &str,
        logical_path: &str,
    ) -> Result<Vec<u8>, SkillApplicationError> {
        let snapshot = self.read_overlay_applied_package(canonical_skill_id, workspace_identity)?;
        let resource = effective_resource(&snapshot, expected_revision, logical_path)?;
        if resource.size_bytes > DEFAULT_OVERLAY_LIMITS.maximum_supporting_file_bytes {
            return Err(SkillApplicationError::OversizedResource);
        }
        match &resource.source {
            EffectiveResourceSource::Base { .. } => self.base.read_effective_resource_bytes(
                canonical_skill_id,
                workspace_identity,
                logical_path,
            ),
            EffectiveResourceSource::Overlay {
                scope,
                workspace_identity: overlay_workspace,
                ..
            } => self.payloads.read_verified(
                &OverlayKey {
                    canonical_skill_id: canonical_skill_id.clone(),
                    scope: *scope,
                    workspace_identity: overlay_workspace.clone(),
                },
                &resource.content_hash,
            ),
        }
    }
}

fn effective_resource<'a>(
    snapshot: &'a OverlayAppliedSkillSnapshot,
    expected_revision: &str,
    logical_path: &str,
) -> Result<
    &'a crate::contexts::tooling::skills::domain::EffectiveSkillResource,
    SkillApplicationError,
> {
    if snapshot.replay.effective().effective_hash() != expected_revision {
        return Err(SkillApplicationError::ConcurrentModification(
            snapshot.base.canonical_skill_id.as_str().to_string(),
        ));
    }
    let resource = snapshot
        .replay
        .effective()
        .resources()
        .iter()
        .find(|resource| resource.logical_path == logical_path)
        .ok_or_else(|| SkillApplicationError::NotFound(logical_path.to_string()))?;
    Ok(resource)
}

fn is_text_overlay_media_type(media_type: &str) -> bool {
    let normalized = media_type
        .split_once(';')
        .map_or(media_type, |(value, _)| value)
        .trim()
        .to_ascii_lowercase();
    normalized.starts_with("text/")
        || matches!(
            normalized.as_str(),
            "application/json" | "application/yaml" | "application/toml"
        )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contexts::tooling::skills::application::{
        OverlayManifestSnapshot, SkillLogAction, SkillLogEvent, SkillLoggingPort,
        SkillResourceDocument,
    };
    use crate::contexts::tooling::skills::domain::{
        BaseSkillResource, OverlayBaseWitness, OverlayDocument, OverlayFile, OverlayPatch,
        OverlayScope, OverlayTrust, SkillLayer,
    };
    use std::collections::BTreeMap;
    use std::sync::Mutex;

    struct FixedBase(OverlayEffectivePackageSnapshot);

    impl OverlayEffectiveSnapshotPort for FixedBase {
        fn read_effective_package(
            &self,
            _canonical_skill_id: &SkillId,
            _workspace_identity: Option<&str>,
        ) -> Result<OverlayEffectivePackageSnapshot, SkillApplicationError> {
            Ok(self.0.clone())
        }

        fn read_effective_resource(
            &self,
            _canonical_skill_id: &SkillId,
            _workspace_identity: Option<&str>,
            logical_path: &str,
        ) -> Result<SkillResourceDocument, SkillApplicationError> {
            (logical_path == "references/base.md")
                .then(|| SkillResourceDocument {
                    content: "Base resource".to_string(),
                    size_bytes: 13,
                })
                .ok_or_else(|| SkillApplicationError::NotFound(logical_path.to_string()))
        }
    }

    struct FixedManifests(Vec<OverlayManifestSnapshot>);

    impl OverlayManifestRepository for FixedManifests {
        fn load(
            &self,
            key: &OverlayKey,
        ) -> Result<Option<OverlayManifestSnapshot>, SkillApplicationError> {
            Ok(self
                .0
                .iter()
                .find(|snapshot| {
                    snapshot.document.canonical_skill_id == key.canonical_skill_id
                        && snapshot.document.scope() == key.scope
                        && snapshot.document.workspace_identity()
                            == key.workspace_identity.as_deref()
                })
                .cloned())
        }

        fn applicable(
            &self,
            _canonical_skill_id: &SkillId,
            _workspace_identity: Option<&str>,
        ) -> Result<Vec<OverlayManifestSnapshot>, SkillApplicationError> {
            Ok(self.0.clone())
        }
    }

    struct EmptyPayloads;

    impl OverlayPayloadRepository for EmptyPayloads {
        fn read_verified(
            &self,
            _key: &OverlayKey,
            _content_hash: &str,
        ) -> Result<Vec<u8>, SkillApplicationError> {
            Ok(Vec::new())
        }

        fn referenced_content_hashes(
            &self,
            _key: &OverlayKey,
        ) -> Result<Vec<String>, SkillApplicationError> {
            Ok(Vec::new())
        }
    }

    struct RejectingPayloads;

    impl OverlayPayloadRepository for RejectingPayloads {
        fn read_verified(
            &self,
            _key: &OverlayKey,
            _content_hash: &str,
        ) -> Result<Vec<u8>, SkillApplicationError> {
            Err(SkillApplicationError::Filesystem(
                "D:/private-overlay-store/customer-payload.bin".to_string(),
            ))
        }

        fn referenced_content_hashes(
            &self,
            _key: &OverlayKey,
        ) -> Result<Vec<String>, SkillApplicationError> {
            Ok(Vec::new())
        }
    }

    #[derive(Default)]
    struct RecordedLogs(Mutex<Vec<SkillLogEvent>>);

    impl SkillLoggingPort for RecordedLogs {
        fn record(&self, event: &SkillLogEvent) -> Result<(), SkillApplicationError> {
            self.0.lock().expect("recorded logs").push(event.clone());
            Ok(())
        }
    }

    struct FixedClock;

    impl SkillClockPort for FixedClock {
        fn now(&self) -> String {
            "2026-08-11T00:00:00Z".to_string()
        }
    }

    struct FixedPayloads(BTreeMap<String, Vec<u8>>);

    impl OverlayPayloadRepository for FixedPayloads {
        fn read_verified(
            &self,
            _key: &OverlayKey,
            content_hash: &str,
        ) -> Result<Vec<u8>, SkillApplicationError> {
            self.0
                .get(content_hash)
                .cloned()
                .ok_or_else(|| SkillApplicationError::NotFound(content_hash.to_string()))
        }

        fn referenced_content_hashes(
            &self,
            _key: &OverlayKey,
        ) -> Result<Vec<String>, SkillApplicationError> {
            Ok(self.0.keys().cloned().collect())
        }
    }

    #[test]
    fn resolver_replays_discovered_scopes_after_the_selected_base_snapshot() {
        let skill_id = SkillId::parse("runtime-overlay").expect("Skill id");
        let base_replay = replay_overlay_scope_chain("Base", &[], &[], None, 0);
        let base = OverlayEffectivePackageSnapshot {
            canonical_skill_id: skill_id.clone(),
            base_identity: "project:runtime-overlay".to_string(),
            base_layer: SkillLayer::Project,
            instructions: "Base".to_string(),
            resources: Vec::new(),
            instruction_hash: base_replay.base().instruction_hash().to_string(),
            package_hash: base_replay.base().effective_hash().to_string(),
        };
        let witness = OverlayBaseWitness::new(
            &base.base_identity,
            &base.instruction_hash,
            &base.package_hash,
        )
        .expect("base witness");
        let manifests = vec![
            overlay(
                &skill_id,
                OverlayScope::System,
                None,
                witness.clone(),
                "Base",
                "System",
            ),
            overlay(
                &skill_id,
                OverlayScope::User,
                None,
                witness.clone(),
                "System",
                "User",
            ),
            overlay(
                &skill_id,
                OverlayScope::Project,
                Some("D:/work"),
                witness,
                "User",
                "Project",
            ),
        ];
        let resolver = OverlayAppliedSkillSnapshotResolver::new(
            Arc::new(FixedBase(base)),
            Arc::new(FixedManifests(manifests)),
            Arc::new(EmptyPayloads),
        );

        let workspace = resolver
            .read_overlay_applied_package(&skill_id, Some("D:/work"))
            .expect("workspace snapshot");
        assert_eq!(workspace.base.base_layer, SkillLayer::Project);
        assert_eq!(workspace.replay.effective().instructions(), "Project");
        assert_eq!(workspace.replay.scope_results().len(), 3);

        let global = resolver
            .read_overlay_applied_package(&skill_id, None)
            .expect("global snapshot");
        assert_eq!(global.replay.effective().instructions(), "User");
        assert_eq!(global.replay.scope_results().len(), 2);
    }

    #[test]
    fn resolver_reads_the_effective_resource_with_revision_and_media_guards() {
        let skill_id = SkillId::parse("runtime-resources").expect("Skill id");
        let base_resources = vec![BaseSkillResource {
            logical_path: "references/base.md".to_string(),
            media_type: "text/markdown".to_string(),
            size_bytes: 13,
            content_hash: "base-hash".to_string(),
            source_layer: SkillLayer::User,
        }];
        let base_replay = replay_overlay_scope_chain("Base", &base_resources, &[], None, 0);
        let base = OverlayEffectivePackageSnapshot {
            canonical_skill_id: skill_id.clone(),
            base_identity: "user:runtime-resources".to_string(),
            base_layer: SkillLayer::User,
            instructions: "Base".to_string(),
            resources: base_resources.clone(),
            instruction_hash: base_replay.base().instruction_hash().to_string(),
            package_hash: base_replay.base().effective_hash().to_string(),
        };
        let mut overlay = OverlayDocument::new(
            skill_id.clone(),
            OverlayScope::User,
            None,
            OverlayBaseWitness::new(
                &base.base_identity,
                &base.instruction_hash,
                &base.package_hash,
            )
            .expect("base witness"),
            OverlayTrust::trusted_local(1),
            "2026-08-11T00:00:00Z",
        )
        .expect("Overlay document");
        overlay.files.push(
            OverlayFile::new(
                "text-file",
                "references/overlay.md",
                "text/markdown; charset=utf-8",
                16,
                "text-hash",
                "payloads/text-hash",
                "2026-08-11T00:00:00Z",
            )
            .expect("text file"),
        );
        overlay.files.push(
            OverlayFile::new(
                "image-file",
                "assets/logo.png",
                "image/png",
                4,
                "image-hash",
                "payloads/image-hash",
                "2026-08-11T00:00:00Z",
            )
            .expect("image file"),
        );
        let resolver = OverlayAppliedSkillSnapshotResolver::new(
            Arc::new(FixedBase(base)),
            Arc::new(FixedManifests(vec![OverlayManifestSnapshot {
                document: overlay,
                document_hash: "manifest-hash".to_string(),
            }])),
            Arc::new(FixedPayloads(BTreeMap::from([
                ("text-hash".to_string(), b"Overlay resource".to_vec()),
                ("image-hash".to_string(), b"text".to_vec()),
            ]))),
        );
        let snapshot = resolver
            .read_overlay_applied_package(&skill_id, None)
            .expect("effective snapshot");
        let revision = snapshot.replay.effective().effective_hash();

        let base_resource = resolver
            .read_overlay_applied_resource(&skill_id, None, revision, "references/base.md")
            .expect("base resource");
        assert_eq!(base_resource.content, "Base resource");
        let overlay_resource = resolver
            .read_overlay_applied_resource(&skill_id, None, revision, "references/overlay.md")
            .expect("Overlay resource");
        assert_eq!(overlay_resource.content, "Overlay resource");
        assert!(matches!(
            resolver.read_overlay_applied_resource(
                &skill_id,
                None,
                "stale-revision",
                "references/overlay.md"
            ),
            Err(SkillApplicationError::ConcurrentModification(_))
        ));
        assert_eq!(
            resolver.read_overlay_applied_resource(&skill_id, None, revision, "assets/logo.png"),
            Err(SkillApplicationError::BinaryResource)
        );
    }

    #[test]
    fn unsafe_overlay_states_never_enter_effective_content_and_logs_remain_redacted() {
        let raw_skill_identity = "runtime-sensitive-skill";
        let skill_id = SkillId::parse(raw_skill_identity).expect("Skill id");
        let base = base_snapshot(&skill_id, "Base safe instructions");
        let witness = OverlayBaseWitness::new(
            &base.base_identity,
            &base.instruction_hash,
            &base.package_hash,
        )
        .expect("base witness");
        let logging = Arc::new(RecordedLogs::default());

        let mut untrusted = OverlayDocument::new(
            skill_id.clone(),
            OverlayScope::System,
            None,
            witness.clone(),
            OverlayTrust::untrusted_imported(Some(
                "D:/private-imports/customer-overlay.zip".to_string(),
            )),
            "2026-08-11T00:00:00Z",
        )
        .expect("untrusted Overlay");
        untrusted.patches.push(
            OverlayPatch::new(
                "untrusted-private-patch",
                "Base safe instructions",
                "UNTRUSTED_PRIVATE_INSTRUCTIONS",
                false,
                &base.instruction_hash,
                "2026-08-11T00:00:00Z",
            )
            .expect("untrusted patch"),
        );
        let untrusted_resolver = resolver_with_diagnostics(
            base.clone(),
            vec![OverlayManifestSnapshot {
                document: untrusted,
                document_hash: "untrusted-private-document-hash".to_string(),
            }],
            Arc::new(EmptyPayloads),
            logging.clone(),
        );
        let untrusted_snapshot = untrusted_resolver
            .read_overlay_applied_package(&skill_id, None)
            .expect("untrusted fallback");
        assert_eq!(
            untrusted_snapshot.replay.effective().instructions(),
            "Base safe instructions"
        );
        assert_eq!(
            untrusted_snapshot.replay.scope_results()[0].status(),
            &OverlayScopeReplayStatus::Untrusted
        );

        let mut conflicted = overlay(
            &skill_id,
            OverlayScope::System,
            None,
            witness.clone(),
            "MISSING_PRIVATE_TARGET",
            "CONFLICTED_PRIVATE_INSTRUCTIONS",
        );
        conflicted.document.files.push(
            OverlayFile::new(
                "conflicted-private-file",
                "references/conflicted-private.md",
                "text/markdown",
                24,
                "conflicted-private-content-hash",
                "payloads/conflicted-private-content-hash",
                "2026-08-11T00:00:00Z",
            )
            .expect("conflicted file"),
        );
        let blocked = overlay(
            &skill_id,
            OverlayScope::Project,
            Some("D:/private-workspace/customer-project"),
            witness.clone(),
            "Base safe instructions",
            "BLOCKED_PRIVATE_INSTRUCTIONS",
        );
        let conflict_resolver = resolver_with_diagnostics(
            base.clone(),
            vec![conflicted, blocked],
            Arc::new(EmptyPayloads),
            logging.clone(),
        );
        let conflict_snapshot = conflict_resolver
            .read_overlay_applied_package(&skill_id, Some("D:/private-workspace/customer-project"))
            .expect("conflict fallback");
        assert_eq!(
            conflict_snapshot.replay.effective().instructions(),
            "Base safe instructions"
        );
        assert!(conflict_snapshot.replay.effective().resources().is_empty());
        assert!(matches!(
            conflict_snapshot.replay.scope_results()[0].status(),
            OverlayScopeReplayStatus::Conflict(_)
        ));
        assert!(matches!(
            conflict_snapshot.replay.scope_results()[1].status(),
            OverlayScopeReplayStatus::Blocked { .. }
        ));

        let mut invalid = OverlayDocument::new(
            skill_id.clone(),
            OverlayScope::System,
            None,
            witness,
            OverlayTrust::trusted_local(1),
            "2026-08-11T00:00:00Z",
        )
        .expect("invalid Overlay fixture");
        invalid.files.push(
            OverlayFile::new(
                "invalid-private-file",
                "references/customer-secret.md",
                "text/markdown",
                18,
                "missing-private-payload-hash",
                "payloads/missing-private-payload-hash",
                "2026-08-11T00:00:00Z",
            )
            .expect("invalid payload fixture"),
        );
        let invalid_resolver = resolver_with_diagnostics(
            base,
            vec![OverlayManifestSnapshot {
                document: invalid,
                document_hash: "invalid-private-document-hash".to_string(),
            }],
            Arc::new(RejectingPayloads),
            logging.clone(),
        );
        let invalid_snapshot = invalid_resolver
            .read_overlay_applied_package(&skill_id, None)
            .expect("integrity fallback");
        assert_eq!(
            invalid_snapshot.replay.effective().instructions(),
            "Base safe instructions"
        );
        assert!(invalid_snapshot.replay.effective().resources().is_empty());
        assert!(matches!(
            invalid_snapshot.replay.scope_results()[0].status(),
            OverlayScopeReplayStatus::IntegrityFailure(_)
        ));

        let events = logging.0.lock().expect("recorded logs");
        assert_eq!(events.len(), 4);
        assert!(events
            .iter()
            .all(|event| event.action == SkillLogAction::OverlayValidation));
        let serialized = format!("{events:?}");
        for private_value in [
            raw_skill_identity,
            "UNTRUSTED_PRIVATE_INSTRUCTIONS",
            "D:/private-imports/customer-overlay.zip",
            "MISSING_PRIVATE_TARGET",
            "CONFLICTED_PRIVATE_INSTRUCTIONS",
            "references/conflicted-private.md",
            "BLOCKED_PRIVATE_INSTRUCTIONS",
            "D:/private-workspace/customer-project",
            "references/customer-secret.md",
            "missing-private-payload-hash",
            "D:/private-overlay-store/customer-payload.bin",
        ] {
            assert!(!serialized.contains(private_value));
        }
        assert!(events.iter().all(|event| {
            event.skill_id.is_none()
                && event.context.contains_key("identityHash")
                && event.context.get("target").map(String::as_str) == Some("replay")
                && !event.context.contains_key("pathHash")
                && !event.context.contains_key("contentHash")
        }));
        assert!(serialized.contains("trust"));
        assert!(serialized.contains("replay-conflict"));
        assert!(serialized.contains("integrity"));
    }

    fn base_snapshot(skill_id: &SkillId, instructions: &str) -> OverlayEffectivePackageSnapshot {
        let replay = replay_overlay_scope_chain(instructions, &[], &[], None, 0);
        OverlayEffectivePackageSnapshot {
            canonical_skill_id: skill_id.clone(),
            base_identity: format!("system:{}", skill_id.as_str()),
            base_layer: SkillLayer::System,
            instructions: instructions.to_string(),
            resources: Vec::new(),
            instruction_hash: replay.base().instruction_hash().to_string(),
            package_hash: replay.base().effective_hash().to_string(),
        }
    }

    fn resolver_with_diagnostics(
        base: OverlayEffectivePackageSnapshot,
        manifests: Vec<OverlayManifestSnapshot>,
        payloads: Arc<dyn OverlayPayloadRepository>,
        logging: Arc<RecordedLogs>,
    ) -> OverlayAppliedSkillSnapshotResolver {
        OverlayAppliedSkillSnapshotResolver::new(
            Arc::new(FixedBase(base)),
            Arc::new(FixedManifests(manifests)),
            payloads,
        )
        .with_runtime_diagnostics(logging, Arc::new(FixedClock))
    }

    fn overlay(
        skill_id: &SkillId,
        scope: OverlayScope,
        workspace: Option<&str>,
        witness: OverlayBaseWitness,
        old_string: &str,
        new_string: &str,
    ) -> OverlayManifestSnapshot {
        let mut document = OverlayDocument::new(
            skill_id.clone(),
            scope,
            workspace,
            witness,
            OverlayTrust::trusted_local(1),
            "2026-08-11T00:00:00Z",
        )
        .expect("Overlay document");
        let patch_id = format!("{}-patch", scope.as_str());
        document.patches.push(
            OverlayPatch::new(
                &patch_id,
                old_string,
                new_string,
                false,
                "creation-base",
                "2026-08-11T00:00:00Z",
            )
            .expect("Overlay patch"),
        );
        OverlayManifestSnapshot {
            document,
            document_hash: format!("{}-hash", scope.as_str()),
        }
    }
}
