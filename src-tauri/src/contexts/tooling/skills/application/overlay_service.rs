#![cfg_attr(not(test), allow(dead_code))]

use super::{
    build_bounded_diff, build_overlay_diff, prepare_overlay_mutation,
    prepare_overlay_reconciliation, replay_conflicts, rescan_overlay_for_promotion,
    validate_overlay_promotion, OverlayActor, OverlayApplicationError, OverlayBoundedText,
    OverlayConflictSummary, OverlayContentScannerPort, OverlayDetail, OverlayDiff,
    OverlayEffectivePackageSnapshot, OverlayEffectiveSnapshotPort, OverlayGovernedMutationOutcome,
    OverlayGovernedMutationRequest, OverlayHistoryAction, OverlayHistoryEntry, OverlayHistoryPage,
    OverlayHistoryQuery, OverlayHistoryRepository, OverlayImportParserPort, OverlayImportRequest,
    OverlayImportReview, OverlayKey, OverlayManifestRepository, OverlayManifestSnapshot,
    OverlayMutation, OverlayMutationKind, OverlayMutationOutcome, OverlayMutationRequest,
    OverlayMutationSummary, OverlayPayloadRepository, OverlayPinStatePort, OverlayPreparationInput,
    OverlayPreparationSnapshots, OverlayPreview, OverlayPromotionRequest,
    OverlayPromotionValidationInput, OverlayReconciliationInput, OverlayReconciliationPreview,
    OverlayReconciliationRequest, OverlayResourceShadow, OverlayResourceSummary,
    OverlayRuntimeCacheInvalidationPort, OverlayScanResult, OverlayScopeDiff, OverlayScopeStatus,
    OverlayScopeSummary, OverlayStatus, OverlaySummary, OverlayTransactionExecutor,
    OverlayTransactionPlan, OverlayUsageDelta, OverlayUsageStatePort, SkillApplicationError,
    SkillClockPort,
};
use crate::contexts::tooling::skills::domain::{
    replay_overlay_scope_chain, EffectiveResourceSource, OverlayConflictState, OverlayDocument,
    OverlayMutationState, OverlayOrigin, OverlayScope, OverlayScopeReplay, OverlayScopeReplayInput,
    OverlayScopeReplayStatus, OverlayTrustState, SkillId, DEFAULT_OVERLAY_LIMITS,
    OVERLAY_TEXT_SCANNER_VERSION,
};
use sha2::{Digest, Sha256};
use std::sync::Arc;

const MAXIMUM_DETAIL_TEXT_CHARACTERS: usize = 12_000;
const MAXIMUM_DETAIL_ENTRIES: usize = 100;
const MAXIMUM_SHADOW_SUMMARIES: usize = 8;

#[derive(Clone)]
pub(crate) struct SkillOverlayApplicationService {
    manifests: Arc<dyn OverlayManifestRepository>,
    effective: Arc<dyn OverlayEffectiveSnapshotPort>,
    pins: Arc<dyn OverlayPinStatePort>,
    clock: Arc<dyn SkillClockPort>,
    history: Option<Arc<dyn OverlayHistoryRepository>>,
    usage: Option<Arc<dyn OverlayUsageStatePort>>,
    transactions: Option<Arc<dyn OverlayTransactionExecutor>>,
    payloads: Option<Arc<dyn OverlayPayloadRepository>>,
    scanner: Option<Arc<dyn OverlayContentScannerPort>>,
    imports: Option<Arc<dyn OverlayImportParserPort>>,
    runtime_cache: Option<Arc<dyn OverlayRuntimeCacheInvalidationPort>>,
}

impl SkillOverlayApplicationService {
    pub(crate) fn new(
        manifests: Arc<dyn OverlayManifestRepository>,
        effective: Arc<dyn OverlayEffectiveSnapshotPort>,
        pins: Arc<dyn OverlayPinStatePort>,
        clock: Arc<dyn SkillClockPort>,
    ) -> Self {
        Self {
            manifests,
            effective,
            pins,
            clock,
            history: None,
            usage: None,
            transactions: None,
            payloads: None,
            scanner: None,
            imports: None,
            runtime_cache: None,
        }
    }

    pub(crate) fn with_mutation_ports(
        mut self,
        history: Arc<dyn OverlayHistoryRepository>,
        usage: Arc<dyn OverlayUsageStatePort>,
        transactions: Arc<dyn OverlayTransactionExecutor>,
    ) -> Self {
        self.history = Some(history);
        self.usage = Some(usage);
        self.transactions = Some(transactions);
        self
    }

    pub(crate) fn with_promotion_ports(
        mut self,
        payloads: Arc<dyn OverlayPayloadRepository>,
        scanner: Arc<dyn OverlayContentScannerPort>,
    ) -> Self {
        self.payloads = Some(payloads);
        self.scanner = Some(scanner);
        self
    }

    pub(crate) fn with_import_parser(mut self, imports: Arc<dyn OverlayImportParserPort>) -> Self {
        self.imports = Some(imports);
        self
    }

    pub(crate) fn with_runtime_cache(
        mut self,
        runtime_cache: Arc<dyn OverlayRuntimeCacheInvalidationPort>,
    ) -> Self {
        self.runtime_cache = Some(runtime_cache);
        self
    }

    fn invalidate_runtime_cache(&self, key: &OverlayKey) {
        if let Some(runtime_cache) = &self.runtime_cache {
            runtime_cache.invalidate(key);
        }
    }

    pub(crate) fn query(
        &self,
        canonical_skill_id: &SkillId,
        active_workspace: Option<&str>,
    ) -> Result<OverlayDetail, SkillApplicationError> {
        let base = self
            .effective
            .read_effective_package(canonical_skill_id, active_workspace)?;
        validate_base_identity(canonical_skill_id, &base)?;
        let applicable = self
            .manifests
            .applicable(canonical_skill_id, active_workspace)?;
        let pin = self
            .pins
            .pin_snapshot(canonical_skill_id, active_workspace)?;
        let replay = replay_applicable(&base, &applicable, active_workspace);
        Ok(build_detail(&base, &applicable, pin.pinned, &replay))
    }

    pub(crate) fn effective_diff(
        &self,
        canonical_skill_id: &SkillId,
        active_workspace: Option<&str>,
    ) -> Result<OverlayDiff, SkillApplicationError> {
        Ok(self.query(canonical_skill_id, active_workspace)?.diff)
    }

    pub(crate) fn history(
        &self,
        key: &OverlayKey,
        active_workspace: Option<&str>,
        query: &OverlayHistoryQuery,
    ) -> Result<OverlayHistoryPage, SkillApplicationError> {
        validate_import_review_context(key, active_workspace)?;
        self.history
            .as_ref()
            .ok_or_else(mutation_ports_unavailable)?
            .read_verified_page(key, query)
    }

    pub(crate) fn history_by_application(
        &self,
        key: &OverlayKey,
        application_id: &str,
    ) -> Result<Option<OverlayHistoryEntry>, SkillApplicationError> {
        self.transactions
            .as_ref()
            .ok_or_else(mutation_ports_unavailable)?
            .recover(key)?;
        self.history
            .as_ref()
            .ok_or_else(mutation_ports_unavailable)?
            .find_curator_application(key, application_id)
    }

    pub(crate) fn import_overlay(
        &self,
        request: &OverlayImportRequest,
        active_workspace: Option<&str>,
    ) -> Result<OverlayImportReview, SkillApplicationError> {
        let history = self
            .history
            .as_ref()
            .ok_or_else(mutation_ports_unavailable)?;
        let usage = self.usage.as_ref().ok_or_else(mutation_ports_unavailable)?;
        let transactions = self
            .transactions
            .as_ref()
            .ok_or_else(mutation_ports_unavailable)?;
        let imports = self.imports.as_ref().ok_or_else(import_ports_unavailable)?;
        let key = OverlayKey {
            canonical_skill_id: request.canonical_skill_id.clone(),
            scope: request.scope,
            workspace_identity: request.workspace_identity.clone(),
        };
        validate_import_review_context(&key, active_workspace)?;
        if self.manifests.load(&key)?.is_some() {
            return Err(SkillApplicationError::Conflict(
                request.canonical_skill_id.as_str().to_string(),
            ));
        }
        let base = self
            .effective
            .read_effective_package(&request.canonical_skill_id, active_workspace)?;
        validate_base_identity(&request.canonical_skill_id, &base)?;
        let pin = self
            .pins
            .pin_snapshot(&request.canonical_skill_id, active_workspace)?;
        if pin.pinned {
            return Err(OverlayApplicationError::PinnedRefusal {
                skill_id: request.canonical_skill_id.as_str().to_string(),
            }
            .into());
        }
        if request.witnesses.expected_overlay_revision.is_some()
            || request.witnesses.expected_base_instruction_hash != base.instruction_hash
            || request.witnesses.expected_base_package_hash != base.package_hash
            || request.witnesses.expected_pinned != pin.pinned
        {
            return Err(OverlayApplicationError::StaleWitnesses {
                expected_revision: request.witnesses.expected_overlay_revision,
                current_revision: None,
                base_changed: request.witnesses.expected_base_instruction_hash
                    != base.instruction_hash
                    || request.witnesses.expected_base_package_hash != base.package_hash,
                payload_changed: false,
                pin_changed: request.witnesses.expected_pinned != pin.pinned,
            }
            .into());
        }
        let prepared = imports.parse(request)?;
        if prepared.document.canonical_skill_id != request.canonical_skill_id
            || prepared.document.scope() != request.scope
            || prepared.document.workspace_identity() != request.workspace_identity.as_deref()
        {
            return Err(OverlayApplicationError::ImportRejected {
                code: "import-target-mismatch".to_string(),
            }
            .into());
        }
        let next_manifest = transactions.manifest_snapshot(prepared.document)?;
        let usage_snapshot = usage.usage_snapshot(&key)?;
        let prior_event_hash = history.verified_tail_hash(&key)?;
        let timestamp = self.clock.now();
        let history_event = OverlayHistoryEntry {
            event_id: format!("event-import-{}", next_manifest.document.revision()),
            canonical_skill_id: request.canonical_skill_id.clone(),
            scope: request.scope,
            prior_revision: None,
            next_revision: next_manifest.document.revision(),
            actor: OverlayActor::User,
            action: OverlayHistoryAction::Import,
            timestamp: timestamp.clone(),
            prior_document_hash: None,
            next_document_hash: next_manifest.document_hash.clone(),
            scanner_version: prepared.scanner_version,
            safe_outcome: "overlay-imported-untrusted".to_string(),
            curator_application_id: None,
            committed_effective_diff_hash: None,
            prior_event_hash,
            event_hash: String::new(),
        };
        transactions.execute(OverlayTransactionPlan {
            key: key.clone(),
            expected_revision: None,
            expected_document_hash: None,
            next_manifest,
            payload_additions: prepared.payloads,
            history_event,
            usage_delta: OverlayUsageDelta {
                patch_count_delta: 0,
                overlay_mutation_count_delta: 1,
                timestamp,
                expected_revision_witness: usage_snapshot.revision_witness,
            },
        })?;
        self.invalidate_runtime_cache(&key);
        self.query_untrusted_import(&key, active_workspace)
    }

    pub(crate) fn query_untrusted_import(
        &self,
        key: &OverlayKey,
        active_workspace: Option<&str>,
    ) -> Result<OverlayImportReview, SkillApplicationError> {
        validate_import_review_context(key, active_workspace)?;
        let base = self
            .effective
            .read_effective_package(&key.canonical_skill_id, active_workspace)?;
        validate_base_identity(&key.canonical_skill_id, &base)?;
        let imported = self.manifests.load(key)?.ok_or_else(|| {
            SkillApplicationError::NotFound(key.canonical_skill_id.as_str().to_string())
        })?;
        validate_untrusted_import(&imported)?;
        let applicable = self
            .manifests
            .applicable(&key.canonical_skill_id, active_workspace)?;
        let replay = replay_import_review(&base, &applicable, &imported, active_workspace)?;
        Ok(build_import_review(&base, &imported, &replay))
    }

    pub(crate) fn promote_import(
        &self,
        request: &OverlayPromotionRequest,
        active_workspace: Option<&str>,
    ) -> Result<OverlayMutationOutcome, SkillApplicationError> {
        let history = self
            .history
            .as_ref()
            .ok_or_else(mutation_ports_unavailable)?;
        let usage = self.usage.as_ref().ok_or_else(mutation_ports_unavailable)?;
        let transactions = self
            .transactions
            .as_ref()
            .ok_or_else(mutation_ports_unavailable)?;
        let payloads = self
            .payloads
            .as_ref()
            .ok_or_else(promotion_ports_unavailable)?;
        let scanner = self
            .scanner
            .as_ref()
            .ok_or_else(promotion_ports_unavailable)?;
        let key = OverlayKey {
            canonical_skill_id: request.canonical_skill_id.clone(),
            scope: request.scope,
            workspace_identity: request.workspace_identity.clone(),
        };
        validate_import_review_context(&key, active_workspace)?;
        let base = self
            .effective
            .read_effective_package(&request.canonical_skill_id, active_workspace)?;
        validate_base_identity(&request.canonical_skill_id, &base)?;
        let current = self.manifests.load(&key)?.ok_or_else(|| {
            SkillApplicationError::NotFound(request.canonical_skill_id.as_str().to_string())
        })?;
        validate_untrusted_import(&current)?;
        let pin = self
            .pins
            .pin_snapshot(&request.canonical_skill_id, active_workspace)?;
        if pin.pinned {
            return Err(OverlayApplicationError::PinnedRefusal {
                skill_id: request.canonical_skill_id.as_str().to_string(),
            }
            .into());
        }
        let live_scan = rescan_overlay_for_promotion(
            &current.document,
            &key,
            payloads.as_ref(),
            scanner.as_ref(),
        )?;
        validate_overlay_promotion(&OverlayPromotionValidationInput {
            request,
            current: &current,
            base: &base,
            pin: &pin,
            live_scan: &live_scan,
        })?;
        let applicable = self
            .manifests
            .applicable(&request.canonical_skill_id, active_workspace)?;
        let review_replay = replay_import_review(&base, &applicable, &current, active_workspace)?;
        let conflicts = replay_conflicts(&review_replay);
        if !conflicts.is_empty() {
            return Err(OverlayApplicationError::NeedsReconciliation {
                conflict_count: conflicts.len(),
            }
            .into());
        }

        let timestamp = self.clock.now();
        let mut next_document = current.document.clone();
        next_document.promote_import(
            request.reviewed_revision,
            &request.reviewed_document_hash,
            &timestamp,
        )?;
        let next_manifest = transactions.manifest_snapshot(next_document)?;
        let usage_snapshot = usage.usage_snapshot(&key)?;
        let prior_event_hash = history.verified_tail_hash(&key)?;
        let history_event = promotion_history_event(
            &key,
            &current,
            &next_manifest,
            &live_scan,
            &timestamp,
            prior_event_hash,
        );
        let outcome = transactions.execute(OverlayTransactionPlan {
            key: key.clone(),
            expected_revision: Some(current.document.revision()),
            expected_document_hash: Some(current.document_hash.clone()),
            next_manifest: next_manifest.clone(),
            payload_additions: Vec::new(),
            history_event,
            usage_delta: OverlayUsageDelta {
                patch_count_delta: 0,
                overlay_mutation_count_delta: 1,
                timestamp,
                expected_revision_witness: usage_snapshot.revision_witness,
            },
        })?;
        self.invalidate_runtime_cache(&key);
        let mut next_applicable = applicable
            .into_iter()
            .filter(|snapshot| {
                snapshot.document.scope() != key.scope
                    || snapshot.document.workspace_identity() != key.workspace_identity.as_deref()
            })
            .collect::<Vec<_>>();
        next_applicable.push(next_manifest);
        next_applicable.sort_by_key(|snapshot| snapshot.document.scope());
        let replay = replay_applicable(&base, &next_applicable, active_workspace);
        Ok(OverlayMutationOutcome {
            summary: build_summary(&base, &next_applicable, pin.pinned, &replay),
            committed_revision: outcome.committed_revision,
            diff: build_overlay_diff(
                &base,
                &replay,
                DEFAULT_OVERLAY_LIMITS.maximum_instruction_characters,
            ),
        })
    }

    pub(crate) fn preview(
        &self,
        request: &OverlayMutationRequest,
        active_workspace: Option<&str>,
    ) -> Result<OverlayPreview, SkillApplicationError> {
        let base = self
            .effective
            .read_effective_package(&request.canonical_skill_id, active_workspace)?;
        validate_base_identity(&request.canonical_skill_id, &base)?;
        let key = OverlayKey {
            canonical_skill_id: request.canonical_skill_id.clone(),
            scope: request.scope,
            workspace_identity: request.workspace_identity.clone(),
        };
        let current = self.manifests.load(&key)?;
        let applicable = self
            .manifests
            .applicable(&request.canonical_skill_id, active_workspace)?;
        let pin = self
            .pins
            .pin_snapshot(&request.canonical_skill_id, active_workspace)?;
        let timestamp = self.clock.now();
        let mutation_id = preview_mutation_id(request);
        let prepared = prepare_overlay_mutation(&OverlayPreparationInput::with_default_limits(
            request,
            OverlayPreparationSnapshots {
                base: &base,
                current: current.as_ref(),
                applicable: &applicable,
                active_workspace,
                pin: &pin,
            },
            &timestamp,
            &mutation_id,
        ))?;
        Ok(prepared.preview)
    }

    pub(crate) fn preview_reconciliation(
        &self,
        request: &OverlayReconciliationRequest,
        active_workspace: Option<&str>,
    ) -> Result<OverlayReconciliationPreview, SkillApplicationError> {
        let (base, current, applicable, pin) =
            self.reconciliation_snapshots(request, active_workspace)?;
        let timestamp = self.clock.now();
        Ok(prepare_overlay_reconciliation(&OverlayReconciliationInput {
            request,
            base: &base,
            current: &current,
            applicable: &applicable,
            active_workspace,
            pin: &pin,
            timestamp: &timestamp,
        })?
        .preview)
    }

    pub(crate) fn reconcile(
        &self,
        request: &OverlayReconciliationRequest,
        active_workspace: Option<&str>,
    ) -> Result<OverlayMutationOutcome, SkillApplicationError> {
        let history = self
            .history
            .as_ref()
            .ok_or_else(mutation_ports_unavailable)?;
        let usage = self.usage.as_ref().ok_or_else(mutation_ports_unavailable)?;
        let transactions = self
            .transactions
            .as_ref()
            .ok_or_else(mutation_ports_unavailable)?;
        let (base, current, applicable, pin) =
            self.reconciliation_snapshots(request, active_workspace)?;
        let timestamp = self.clock.now();
        let prepared = prepare_overlay_reconciliation(&OverlayReconciliationInput {
            request,
            base: &base,
            current: &current,
            applicable: &applicable,
            active_workspace,
            pin: &pin,
            timestamp: &timestamp,
        })?;
        if !prepared.preview.can_commit {
            return Err(OverlayApplicationError::NeedsReconciliation {
                conflict_count: prepared.preview.conflict_choices.len(),
            }
            .into());
        }
        let key = OverlayKey {
            canonical_skill_id: request.canonical_skill_id.clone(),
            scope: request.scope,
            workspace_identity: request.workspace_identity.clone(),
        };
        let usage_snapshot = usage.usage_snapshot(&key)?;
        let prior_event_hash = history.verified_tail_hash(&key)?;
        let next_manifest = transactions.manifest_snapshot(prepared.next_document.clone())?;
        let history_event = reconciliation_history_event(
            &key,
            &current,
            &next_manifest,
            &timestamp,
            prior_event_hash,
        );
        let outcome = transactions.execute(OverlayTransactionPlan {
            key: key.clone(),
            expected_revision: Some(current.document.revision()),
            expected_document_hash: Some(current.document_hash.clone()),
            next_manifest: next_manifest.clone(),
            payload_additions: Vec::new(),
            history_event,
            usage_delta: OverlayUsageDelta {
                patch_count_delta: u64::from(prepared.edited_patch),
                overlay_mutation_count_delta: 1,
                timestamp,
                expected_revision_witness: usage_snapshot.revision_witness,
            },
        })?;
        self.invalidate_runtime_cache(&key);
        let mut next_applicable = applicable
            .into_iter()
            .filter(|snapshot| {
                snapshot.document.scope() != key.scope
                    || snapshot.document.workspace_identity() != key.workspace_identity.as_deref()
            })
            .collect::<Vec<_>>();
        next_applicable.push(next_manifest);
        next_applicable.sort_by_key(|snapshot| snapshot.document.scope());
        Ok(OverlayMutationOutcome {
            summary: build_summary(&base, &next_applicable, pin.pinned, &prepared.replay),
            committed_revision: outcome.committed_revision,
            diff: prepared.preview.final_diff,
        })
    }

    fn reconciliation_snapshots(
        &self,
        request: &OverlayReconciliationRequest,
        active_workspace: Option<&str>,
    ) -> Result<
        (
            OverlayEffectivePackageSnapshot,
            OverlayManifestSnapshot,
            Vec<OverlayManifestSnapshot>,
            super::OverlayPinSnapshot,
        ),
        SkillApplicationError,
    > {
        let base = self
            .effective
            .read_effective_package(&request.canonical_skill_id, active_workspace)?;
        validate_base_identity(&request.canonical_skill_id, &base)?;
        let key = OverlayKey {
            canonical_skill_id: request.canonical_skill_id.clone(),
            scope: request.scope,
            workspace_identity: request.workspace_identity.clone(),
        };
        let current = self.manifests.load(&key)?.ok_or_else(|| {
            SkillApplicationError::NotFound(request.canonical_skill_id.as_str().to_string())
        })?;
        let applicable = self
            .manifests
            .applicable(&request.canonical_skill_id, active_workspace)?;
        let pin = self
            .pins
            .pin_snapshot(&request.canonical_skill_id, active_workspace)?;
        Ok((base, current, applicable, pin))
    }

    pub(crate) fn create_exact_patch(
        &self,
        request: &OverlayMutationRequest,
        active_workspace: Option<&str>,
    ) -> Result<OverlayMutationOutcome, SkillApplicationError> {
        self.commit_manual_mutation(request, active_workspace, ManualMutationAction::CreatePatch)
    }

    pub(crate) fn commit_governed(
        &self,
        request: &OverlayGovernedMutationRequest,
        active_workspace: Option<&str>,
    ) -> Result<OverlayGovernedMutationOutcome, SkillApplicationError> {
        let action = match &request.mutation.mutation {
            OverlayMutation::ExactPatch { .. } => ManualMutationAction::CreatePatch,
            OverlayMutation::LearnedGuidance { .. } => ManualMutationAction::CreateGuidance,
            _ => {
                return Err(OverlayApplicationError::InvalidRequest {
                    code: "curator-mutation-kind-prohibited".to_string(),
                }
                .into())
            }
        };
        validate_governed_request(request)?;
        validate_manual_action(&request.mutation, action)?;
        let history = self
            .history
            .as_ref()
            .ok_or_else(mutation_ports_unavailable)?;
        let key = OverlayKey {
            canonical_skill_id: request.mutation.canonical_skill_id.clone(),
            scope: request.mutation.scope,
            workspace_identity: request.mutation.workspace_identity.clone(),
        };
        validate_import_review_context(&key, active_workspace)?;
        let transactions = self
            .transactions
            .as_ref()
            .ok_or_else(mutation_ports_unavailable)?;
        transactions.recover(&key)?;
        if let Some(existing) = history.find_curator_application(&key, &request.application_id)? {
            if existing.committed_effective_diff_hash.as_deref()
                != Some(request.expected_effective_diff_hash.as_str())
            {
                return Err(OverlayApplicationError::StaleWitnesses {
                    expected_revision: request.mutation.witnesses.expected_overlay_revision,
                    current_revision: Some(existing.next_revision),
                    base_changed: false,
                    payload_changed: true,
                    pin_changed: false,
                }
                .into());
            }
            return Ok(OverlayGovernedMutationOutcome {
                committed_revision: existing.next_revision,
                history_event_hash: existing.event_hash,
                effective_diff_hash: request.expected_effective_diff_hash.clone(),
                duplicate: true,
            });
        }
        let usage = self.usage.as_ref().ok_or_else(mutation_ports_unavailable)?;
        let base = self
            .effective
            .read_effective_package(&request.mutation.canonical_skill_id, active_workspace)?;
        validate_base_identity(&request.mutation.canonical_skill_id, &base)?;
        let current = self.manifests.load(&key)?;
        let applicable = self
            .manifests
            .applicable(&request.mutation.canonical_skill_id, active_workspace)?;
        let pin = self
            .pins
            .pin_snapshot(&request.mutation.canonical_skill_id, active_workspace)?;
        let timestamp = self.clock.now();
        let prepared = prepare_overlay_mutation(&OverlayPreparationInput::with_default_limits(
            &request.mutation,
            OverlayPreparationSnapshots {
                base: &base,
                current: current.as_ref(),
                applicable: &applicable,
                active_workspace,
                pin: &pin,
            },
            &timestamp,
            &request.application_id,
        ))?;
        validate_manual_target(current.as_ref(), &request.mutation, action)?;
        let effective_diff_hash = &prepared.preview.base_to_proposed.effective_hash;
        if !prepared.preview.can_commit
            || effective_diff_hash != &request.expected_effective_diff_hash
        {
            return Err(OverlayApplicationError::StaleWitnesses {
                expected_revision: request.mutation.witnesses.expected_overlay_revision,
                current_revision: current
                    .as_ref()
                    .map(|snapshot| snapshot.document.revision()),
                base_changed: prepared.preview.witnesses.expected_base_package_hash
                    != request.mutation.witnesses.expected_base_package_hash,
                payload_changed: true,
                pin_changed: pin.pinned != request.mutation.witnesses.expected_pinned,
            }
            .into());
        }
        let next_manifest = transactions.manifest_snapshot(prepared.next_document.clone())?;
        let usage_snapshot = usage.usage_snapshot(&key)?;
        let prior_event_hash = history.verified_tail_hash(&key)?;
        let mut history_event = history_event(
            &key,
            current.as_ref(),
            &next_manifest,
            action,
            &timestamp,
            &request.application_id,
            prior_event_hash,
        );
        history_event.curator_application_id = Some(request.application_id.clone());
        history_event.committed_effective_diff_hash = Some(effective_diff_hash.clone());
        let outcome = transactions.execute(OverlayTransactionPlan {
            key: key.clone(),
            expected_revision: request.mutation.witnesses.expected_overlay_revision,
            expected_document_hash: current
                .as_ref()
                .map(|snapshot| snapshot.document_hash.clone()),
            next_manifest,
            payload_additions: prepared.payload_additions,
            history_event,
            usage_delta: OverlayUsageDelta {
                patch_count_delta: u64::from(action == ManualMutationAction::CreatePatch),
                overlay_mutation_count_delta: 1,
                timestamp,
                expected_revision_witness: usage_snapshot.revision_witness,
            },
        })?;
        self.invalidate_runtime_cache(&key);
        Ok(OverlayGovernedMutationOutcome {
            committed_revision: outcome.committed_revision,
            history_event_hash: outcome.history_event_hash,
            effective_diff_hash: effective_diff_hash.clone(),
            duplicate: false,
        })
    }

    pub(crate) fn disable_exact_patch(
        &self,
        request: &OverlayMutationRequest,
        active_workspace: Option<&str>,
    ) -> Result<OverlayMutationOutcome, SkillApplicationError> {
        self.commit_manual_mutation(
            request,
            active_workspace,
            ManualMutationAction::DisablePatch,
        )
    }

    pub(crate) fn revert_exact_patch(
        &self,
        request: &OverlayMutationRequest,
        active_workspace: Option<&str>,
    ) -> Result<OverlayMutationOutcome, SkillApplicationError> {
        self.commit_manual_mutation(request, active_workspace, ManualMutationAction::RevertPatch)
    }

    pub(crate) fn create_learned_guidance(
        &self,
        request: &OverlayMutationRequest,
        active_workspace: Option<&str>,
    ) -> Result<OverlayMutationOutcome, SkillApplicationError> {
        self.commit_manual_mutation(
            request,
            active_workspace,
            ManualMutationAction::CreateGuidance,
        )
    }

    pub(crate) fn disable_learned_guidance(
        &self,
        request: &OverlayMutationRequest,
        active_workspace: Option<&str>,
    ) -> Result<OverlayMutationOutcome, SkillApplicationError> {
        self.commit_manual_mutation(
            request,
            active_workspace,
            ManualMutationAction::DisableGuidance,
        )
    }

    pub(crate) fn revert_learned_guidance(
        &self,
        request: &OverlayMutationRequest,
        active_workspace: Option<&str>,
    ) -> Result<OverlayMutationOutcome, SkillApplicationError> {
        self.commit_manual_mutation(
            request,
            active_workspace,
            ManualMutationAction::RevertGuidance,
        )
    }

    pub(crate) fn add_supporting_file(
        &self,
        request: &OverlayMutationRequest,
        active_workspace: Option<&str>,
    ) -> Result<OverlayMutationOutcome, SkillApplicationError> {
        self.commit_manual_mutation(request, active_workspace, ManualMutationAction::AddFile)
    }

    pub(crate) fn replace_supporting_file(
        &self,
        request: &OverlayMutationRequest,
        active_workspace: Option<&str>,
    ) -> Result<OverlayMutationOutcome, SkillApplicationError> {
        self.commit_manual_mutation(request, active_workspace, ManualMutationAction::ReplaceFile)
    }

    pub(crate) fn disable_supporting_file(
        &self,
        request: &OverlayMutationRequest,
        active_workspace: Option<&str>,
    ) -> Result<OverlayMutationOutcome, SkillApplicationError> {
        self.commit_manual_mutation(request, active_workspace, ManualMutationAction::DisableFile)
    }

    pub(crate) fn revert_supporting_file(
        &self,
        request: &OverlayMutationRequest,
        active_workspace: Option<&str>,
    ) -> Result<OverlayMutationOutcome, SkillApplicationError> {
        self.commit_manual_mutation(request, active_workspace, ManualMutationAction::RevertFile)
    }

    fn commit_manual_mutation(
        &self,
        request: &OverlayMutationRequest,
        active_workspace: Option<&str>,
        action: ManualMutationAction,
    ) -> Result<OverlayMutationOutcome, SkillApplicationError> {
        validate_manual_action(request, action)?;
        let history = self
            .history
            .as_ref()
            .ok_or_else(mutation_ports_unavailable)?;
        let usage = self.usage.as_ref().ok_or_else(mutation_ports_unavailable)?;
        let transactions = self
            .transactions
            .as_ref()
            .ok_or_else(mutation_ports_unavailable)?;
        let base = self
            .effective
            .read_effective_package(&request.canonical_skill_id, active_workspace)?;
        validate_base_identity(&request.canonical_skill_id, &base)?;
        let key = OverlayKey {
            canonical_skill_id: request.canonical_skill_id.clone(),
            scope: request.scope,
            workspace_identity: request.workspace_identity.clone(),
        };
        let current = self.manifests.load(&key)?;
        let applicable = self
            .manifests
            .applicable(&request.canonical_skill_id, active_workspace)?;
        let pin = self
            .pins
            .pin_snapshot(&request.canonical_skill_id, active_workspace)?;
        let timestamp = self.clock.now();
        let mutation_id = preview_mutation_id(request);
        let prepared = prepare_overlay_mutation(&OverlayPreparationInput::with_default_limits(
            request,
            OverlayPreparationSnapshots {
                base: &base,
                current: current.as_ref(),
                applicable: &applicable,
                active_workspace,
                pin: &pin,
            },
            &timestamp,
            &mutation_id,
        ))?;
        validate_manual_target(current.as_ref(), request, action)?;
        if !prepared.preview.can_commit {
            return Err(OverlayApplicationError::NeedsReconciliation {
                conflict_count: prepared.preview.conflicts.len(),
            }
            .into());
        }
        let usage_snapshot = usage.usage_snapshot(&key)?;
        let prior_event_hash = history.verified_tail_hash(&key)?;
        let next_manifest = transactions.manifest_snapshot(prepared.next_document.clone())?;
        let history_event = history_event(
            &key,
            current.as_ref(),
            &next_manifest,
            action,
            &timestamp,
            &mutation_id,
            prior_event_hash,
        );
        let plan = OverlayTransactionPlan {
            key: key.clone(),
            expected_revision: request.witnesses.expected_overlay_revision,
            expected_document_hash: current
                .as_ref()
                .map(|snapshot| snapshot.document_hash.clone()),
            next_manifest: next_manifest.clone(),
            payload_additions: prepared.payload_additions,
            history_event,
            usage_delta: OverlayUsageDelta {
                patch_count_delta: u64::from(action == ManualMutationAction::CreatePatch),
                overlay_mutation_count_delta: 1,
                timestamp,
                expected_revision_witness: usage_snapshot.revision_witness,
            },
        };
        let outcome = transactions.execute(plan)?;
        self.invalidate_runtime_cache(&key);
        let mut next_applicable = applicable
            .into_iter()
            .filter(|snapshot| {
                snapshot.document.scope() != key.scope
                    || snapshot.document.workspace_identity() != key.workspace_identity.as_deref()
            })
            .collect::<Vec<_>>();
        next_applicable.push(next_manifest);
        next_applicable.sort_by_key(|snapshot| snapshot.document.scope());
        Ok(OverlayMutationOutcome {
            summary: build_summary(&base, &next_applicable, pin.pinned, &prepared.replay),
            committed_revision: outcome.committed_revision,
            diff: prepared.preview.diff,
        })
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ManualMutationAction {
    CreatePatch,
    DisablePatch,
    RevertPatch,
    CreateGuidance,
    DisableGuidance,
    RevertGuidance,
    AddFile,
    ReplaceFile,
    DisableFile,
    RevertFile,
}

fn validate_governed_request(
    request: &OverlayGovernedMutationRequest,
) -> Result<(), SkillApplicationError> {
    let valid_id = !request.application_id.is_empty()
        && request.application_id.len() <= 160
        && request
            .application_id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-' | b'_' | b':'));
    if !valid_id
        || request.expected_effective_diff_hash.trim().is_empty()
        || request.expected_effective_diff_hash.len() > 160
    {
        return Err(OverlayApplicationError::InvalidRequest {
            code: "curator-application-witness-invalid".to_string(),
        }
        .into());
    }
    Ok(())
}

fn validate_manual_action(
    request: &OverlayMutationRequest,
    expected: ManualMutationAction,
) -> Result<(), SkillApplicationError> {
    let matches = matches!(
        (&request.mutation, expected),
        (
            OverlayMutation::ExactPatch { .. },
            ManualMutationAction::CreatePatch
        ) | (
            OverlayMutation::LearnedGuidance { .. },
            ManualMutationAction::CreateGuidance
        ) | (
            OverlayMutation::Disable { .. },
            ManualMutationAction::DisablePatch
        ) | (
            OverlayMutation::Revert { .. },
            ManualMutationAction::RevertPatch
        ) | (
            OverlayMutation::Disable { .. },
            ManualMutationAction::DisableGuidance
        ) | (
            OverlayMutation::Revert { .. },
            ManualMutationAction::RevertGuidance
        ) | (
            OverlayMutation::SupportingFile { .. },
            ManualMutationAction::AddFile
        ) | (
            OverlayMutation::SupportingFile { .. },
            ManualMutationAction::ReplaceFile
        ) | (
            OverlayMutation::Disable { .. },
            ManualMutationAction::DisableFile
        ) | (
            OverlayMutation::Revert { .. },
            ManualMutationAction::RevertFile
        )
    );
    if matches {
        Ok(())
    } else {
        Err(OverlayApplicationError::InvalidRequest {
            code: "manual-overlay-operation-mismatch".to_string(),
        }
        .into())
    }
}

fn validate_manual_target(
    current: Option<&OverlayManifestSnapshot>,
    request: &OverlayMutationRequest,
    action: ManualMutationAction,
) -> Result<(), SkillApplicationError> {
    match action {
        ManualMutationAction::CreatePatch | ManualMutationAction::CreateGuidance => return Ok(()),
        ManualMutationAction::AddFile | ManualMutationAction::ReplaceFile => {
            return validate_supporting_file_write(current, request, action);
        }
        _ => {}
    }
    let mutation_id = match &request.mutation {
        OverlayMutation::Disable { mutation_id } | OverlayMutation::Revert { mutation_id } => {
            mutation_id
        }
        _ => {
            return Err(OverlayApplicationError::InvalidRequest {
                code: "manual-overlay-operation-mismatch".to_string(),
            }
            .into())
        }
    };
    let target_matches = current.is_some_and(|snapshot| match action {
        ManualMutationAction::DisablePatch | ManualMutationAction::RevertPatch => snapshot
            .document
            .patches
            .iter()
            .any(|patch| patch.id == *mutation_id),
        ManualMutationAction::DisableGuidance | ManualMutationAction::RevertGuidance => snapshot
            .document
            .learn_blocks
            .iter()
            .any(|block| block.id == *mutation_id),
        ManualMutationAction::DisableFile | ManualMutationAction::RevertFile => snapshot
            .document
            .files
            .iter()
            .any(|file| file.id == *mutation_id),
        ManualMutationAction::CreatePatch
        | ManualMutationAction::CreateGuidance
        | ManualMutationAction::AddFile
        | ManualMutationAction::ReplaceFile => false,
    });
    if target_matches {
        Ok(())
    } else {
        Err(OverlayApplicationError::InvalidRequest {
            code: "manual-overlay-target-mismatch".to_string(),
        }
        .into())
    }
}

fn validate_supporting_file_write(
    current: Option<&OverlayManifestSnapshot>,
    request: &OverlayMutationRequest,
    action: ManualMutationAction,
) -> Result<(), SkillApplicationError> {
    let logical_path = match &request.mutation {
        OverlayMutation::SupportingFile { logical_path, .. } => logical_path,
        _ => {
            return Err(OverlayApplicationError::InvalidRequest {
                code: "manual-overlay-operation-mismatch".to_string(),
            }
            .into())
        }
    };
    let has_active_file = current.is_some_and(|snapshot| {
        snapshot.document.files.iter().rev().any(|file| {
            file.logical_path == *logical_path && file.state() == OverlayMutationState::Active
        })
    });
    let valid = match action {
        ManualMutationAction::AddFile => {
            !has_active_file && request.witnesses.expected_payload_hash.is_none()
        }
        ManualMutationAction::ReplaceFile => {
            has_active_file && request.witnesses.expected_payload_hash.is_some()
        }
        _ => false,
    };
    if valid {
        Ok(())
    } else {
        let code = match action {
            ManualMutationAction::AddFile => "supporting-file-add-requires-empty-path",
            ManualMutationAction::ReplaceFile => "supporting-file-replace-requires-active-path",
            _ => "manual-overlay-operation-mismatch",
        };
        Err(OverlayApplicationError::InvalidRequest {
            code: code.to_string(),
        }
        .into())
    }
}

fn history_event(
    key: &OverlayKey,
    current: Option<&OverlayManifestSnapshot>,
    next: &OverlayManifestSnapshot,
    action: ManualMutationAction,
    timestamp: &str,
    mutation_id: &str,
    prior_event_hash: Option<String>,
) -> OverlayHistoryEntry {
    let (history_action, safe_outcome) = match action {
        ManualMutationAction::CreatePatch => (OverlayHistoryAction::Patch, "exact-patch-created"),
        ManualMutationAction::CreateGuidance => {
            (OverlayHistoryAction::Learn, "learned-guidance-created")
        }
        ManualMutationAction::DisablePatch => {
            (OverlayHistoryAction::Disable, "exact-patch-disabled")
        }
        ManualMutationAction::RevertPatch => (OverlayHistoryAction::Revert, "exact-patch-reverted"),
        ManualMutationAction::DisableGuidance => {
            (OverlayHistoryAction::Disable, "learned-guidance-disabled")
        }
        ManualMutationAction::RevertGuidance => {
            (OverlayHistoryAction::Revert, "learned-guidance-reverted")
        }
        ManualMutationAction::AddFile => (OverlayHistoryAction::File, "supporting-file-added"),
        ManualMutationAction::ReplaceFile => {
            (OverlayHistoryAction::File, "supporting-file-replaced")
        }
        ManualMutationAction::DisableFile => {
            (OverlayHistoryAction::Disable, "supporting-file-disabled")
        }
        ManualMutationAction::RevertFile => {
            (OverlayHistoryAction::Revert, "supporting-file-reverted")
        }
    };
    OverlayHistoryEntry {
        event_id: format!("event-{mutation_id}-{}", next.document.revision()),
        canonical_skill_id: key.canonical_skill_id.clone(),
        scope: key.scope,
        prior_revision: current.map(|snapshot| snapshot.document.revision()),
        next_revision: next.document.revision(),
        actor: OverlayActor::User,
        action: history_action,
        timestamp: timestamp.to_string(),
        prior_document_hash: current.map(|snapshot| snapshot.document_hash.clone()),
        next_document_hash: next.document_hash.clone(),
        scanner_version: OVERLAY_TEXT_SCANNER_VERSION.to_string(),
        safe_outcome: safe_outcome.to_string(),
        curator_application_id: None,
        committed_effective_diff_hash: None,
        prior_event_hash,
        event_hash: String::new(),
    }
}

fn promotion_history_event(
    key: &OverlayKey,
    current: &OverlayManifestSnapshot,
    next: &OverlayManifestSnapshot,
    scan: &OverlayScanResult,
    timestamp: &str,
    prior_event_hash: Option<String>,
) -> OverlayHistoryEntry {
    OverlayHistoryEntry {
        event_id: format!("event-promote-{}", current.document.revision()),
        canonical_skill_id: key.canonical_skill_id.clone(),
        scope: key.scope,
        prior_revision: Some(current.document.revision()),
        next_revision: next.document.revision(),
        actor: OverlayActor::User,
        action: OverlayHistoryAction::Promote,
        timestamp: timestamp.to_string(),
        prior_document_hash: Some(current.document_hash.clone()),
        next_document_hash: next.document_hash.clone(),
        scanner_version: scan.scanner_version.clone(),
        safe_outcome: "import-trust-promoted".to_string(),
        curator_application_id: None,
        committed_effective_diff_hash: None,
        prior_event_hash,
        event_hash: String::new(),
    }
}

fn reconciliation_history_event(
    key: &OverlayKey,
    current: &OverlayManifestSnapshot,
    next: &OverlayManifestSnapshot,
    timestamp: &str,
    prior_event_hash: Option<String>,
) -> OverlayHistoryEntry {
    let resolved_conflicts = next
        .document
        .conflicts
        .iter()
        .filter(|conflict| {
            conflict.state() == OverlayConflictState::Resolved
                && conflict.resolution_revision() == Some(next.document.revision())
        })
        .count();
    let ignored_conflicts = next
        .document
        .conflicts
        .iter()
        .filter(|conflict| {
            conflict.state() == OverlayConflictState::Ignored
                && conflict.resolution_revision() == Some(next.document.revision())
        })
        .count();
    OverlayHistoryEntry {
        event_id: format!("event-reconcile-{}", next.document.revision()),
        canonical_skill_id: key.canonical_skill_id.clone(),
        scope: key.scope,
        prior_revision: Some(current.document.revision()),
        next_revision: next.document.revision(),
        actor: OverlayActor::User,
        action: OverlayHistoryAction::Reconcile,
        timestamp: timestamp.to_string(),
        prior_document_hash: Some(current.document_hash.clone()),
        next_document_hash: next.document_hash.clone(),
        scanner_version: OVERLAY_TEXT_SCANNER_VERSION.to_string(),
        safe_outcome: format!(
            "overlay-reconciled:resolved={resolved_conflicts}:ignored={ignored_conflicts}"
        ),
        curator_application_id: None,
        committed_effective_diff_hash: None,
        prior_event_hash,
        event_hash: String::new(),
    }
}

fn mutation_ports_unavailable() -> SkillApplicationError {
    SkillApplicationError::Repository("Overlay mutation ports are unavailable".to_string())
}

fn promotion_ports_unavailable() -> SkillApplicationError {
    SkillApplicationError::Repository("Overlay promotion ports are unavailable".to_string())
}

fn import_ports_unavailable() -> SkillApplicationError {
    SkillApplicationError::Repository("Overlay import parser is unavailable".to_string())
}

fn validate_base_identity(
    requested: &SkillId,
    base: &OverlayEffectivePackageSnapshot,
) -> Result<(), SkillApplicationError> {
    if &base.canonical_skill_id == requested {
        Ok(())
    } else {
        Err(SkillApplicationError::NotFound(
            requested.as_str().to_string(),
        ))
    }
}

fn validate_import_review_context(
    key: &OverlayKey,
    active_workspace: Option<&str>,
) -> Result<(), SkillApplicationError> {
    let valid = match key.scope {
        OverlayScope::Project => {
            key.workspace_identity.as_deref().is_some()
                && key.workspace_identity.as_deref() == active_workspace
        }
        OverlayScope::System | OverlayScope::User => key.workspace_identity.is_none(),
    };
    if valid {
        Ok(())
    } else {
        Err(OverlayApplicationError::InvalidRequest {
            code: "overlay-import-review-workspace-mismatch".to_string(),
        }
        .into())
    }
}

fn validate_untrusted_import(
    imported: &OverlayManifestSnapshot,
) -> Result<(), SkillApplicationError> {
    let trust = imported.document.trust();
    if trust.origin() == OverlayOrigin::Imported && trust.state() == OverlayTrustState::Untrusted {
        Ok(())
    } else {
        Err(OverlayApplicationError::TrustRequired {
            revision: imported.document.revision(),
        }
        .into())
    }
}

fn replay_import_review(
    base: &OverlayEffectivePackageSnapshot,
    applicable: &[OverlayManifestSnapshot],
    imported: &OverlayManifestSnapshot,
    active_workspace: Option<&str>,
) -> Result<OverlayScopeReplay, SkillApplicationError> {
    let mut inputs = applicable
        .iter()
        .filter(|snapshot| {
            snapshot.document.scope() < imported.document.scope()
                && snapshot
                    .document
                    .trust()
                    .is_trusted_for_revision(snapshot.document.revision())
        })
        .map(|snapshot| OverlayScopeReplayInput::verified(&snapshot.document))
        .collect::<Vec<_>>();
    inputs.push(
        OverlayScopeReplayInput::untrusted_import_review(&imported.document).ok_or_else(|| {
            OverlayApplicationError::TrustRequired {
                revision: imported.document.revision(),
            }
        })?,
    );
    Ok(replay_overlay_scope_chain(
        &base.instructions,
        &base.resources,
        &inputs,
        active_workspace,
        MAXIMUM_SHADOW_SUMMARIES,
    ))
}

fn build_import_review(
    base: &OverlayEffectivePackageSnapshot,
    imported: &OverlayManifestSnapshot,
    replay: &OverlayScopeReplay,
) -> OverlayImportReview {
    let document = &imported.document;
    let mut mutations = mutation_summaries(std::slice::from_ref(imported));
    let mutations_truncated = mutations.len() > MAXIMUM_DETAIL_ENTRIES;
    mutations.truncate(MAXIMUM_DETAIL_ENTRIES);
    let mut resources = import_resource_summaries(document, replay);
    let resources_truncated = resources.len() > MAXIMUM_DETAIL_ENTRIES;
    resources.truncate(MAXIMUM_DETAIL_ENTRIES);
    let mut conflicts = conflict_summaries(std::slice::from_ref(imported));
    conflicts.extend(replay_conflicts(replay));
    conflicts.sort_by(|left, right| {
        (&left.mutation_id, &left.safe_reason, &left.id).cmp(&(
            &right.mutation_id,
            &right.safe_reason,
            &right.id,
        ))
    });
    conflicts.dedup_by(|left, right| {
        left.mutation_id == right.mutation_id && left.safe_reason == right.safe_reason
    });
    let conflicts_truncated = conflicts.len() > MAXIMUM_DETAIL_ENTRIES;
    conflicts.truncate(MAXIMUM_DETAIL_ENTRIES);
    OverlayImportReview {
        source_summary: safe_source_summary(document.trust().source_summary()),
        revision: document.revision(),
        document_hash: imported.document_hash.clone(),
        scan: OverlayScanResult {
            scanner_version: OVERLAY_TEXT_SCANNER_VERSION.to_string(),
            passed: true,
            safe_rule_ids: Vec::new(),
            rule_ids_truncated: false,
        },
        diff: build_overlay_diff(
            base,
            replay,
            DEFAULT_OVERLAY_LIMITS.maximum_instruction_characters,
        ),
        mutations,
        mutations_truncated,
        resources,
        resources_truncated,
        conflicts,
        conflicts_truncated,
    }
}

fn safe_source_summary(source: Option<&str>) -> String {
    let basename = source
        .unwrap_or_default()
        .rsplit(['/', '\\'])
        .find(|segment| !segment.trim().is_empty())
        .unwrap_or("imported-overlay-package");
    let summary = basename
        .chars()
        .filter(|character| !character.is_control())
        .take(160)
        .collect::<String>();
    if summary.trim().is_empty() {
        "imported-overlay-package".to_string()
    } else {
        summary
    }
}

fn import_resource_summaries(
    document: &OverlayDocument,
    replay: &OverlayScopeReplay,
) -> Vec<OverlayResourceSummary> {
    document
        .files
        .iter()
        .map(|file| {
            let effective = replay.effective().resources().iter().find(|resource| {
                matches!(
                    &resource.source,
                    EffectiveResourceSource::Overlay { mutation_id, .. } if mutation_id == &file.id
                )
            });
            let shadowed = effective
                .into_iter()
                .flat_map(|resource| resource.shadowed.iter())
                .map(|shadow| match &shadow.source {
                    EffectiveResourceSource::Base { layer } => OverlayResourceShadow {
                        scope: None,
                        base_layer: Some(*layer),
                        content_hash: shadow.content_hash.clone(),
                    },
                    EffectiveResourceSource::Overlay { scope, .. } => OverlayResourceShadow {
                        scope: Some(*scope),
                        base_layer: None,
                        content_hash: shadow.content_hash.clone(),
                    },
                })
                .collect();
            OverlayResourceSummary {
                mutation_id: file.id.clone(),
                logical_path: file.logical_path.clone(),
                media_type: file.media_type.clone(),
                size_bytes: file.size,
                content_hash: file.content_hash.clone(),
                effective_scope: document.scope(),
                state: file.state(),
                shadowed,
                shadowed_truncated: effective.is_some_and(|resource| resource.shadowed_truncated),
            }
        })
        .collect()
}

fn replay_applicable(
    base: &OverlayEffectivePackageSnapshot,
    applicable: &[OverlayManifestSnapshot],
    active_workspace: Option<&str>,
) -> OverlayScopeReplay {
    let inputs = applicable
        .iter()
        .filter(|snapshot| {
            snapshot
                .document
                .trust()
                .is_trusted_for_revision(snapshot.document.revision())
        })
        .map(|snapshot| {
            if base_hash_changed(&snapshot.document, base) {
                OverlayScopeReplayInput::base_drift(&snapshot.document)
            } else {
                OverlayScopeReplayInput::verified(&snapshot.document)
            }
        })
        .collect::<Vec<_>>();
    replay_overlay_scope_chain(
        &base.instructions,
        &base.resources,
        &inputs,
        active_workspace,
        MAXIMUM_SHADOW_SUMMARIES,
    )
}

fn build_detail(
    base: &OverlayEffectivePackageSnapshot,
    applicable: &[OverlayManifestSnapshot],
    pinned: bool,
    replay: &OverlayScopeReplay,
) -> OverlayDetail {
    let diff = build_overlay_diff(
        base,
        replay,
        DEFAULT_OVERLAY_LIMITS.maximum_instruction_characters,
    );
    let summary = build_summary(base, applicable, pinned, replay);
    let scope_diffs = build_scope_diffs(replay);
    let mut mutations = mutation_summaries(applicable);
    let mutations_truncated = mutations.len() > MAXIMUM_DETAIL_ENTRIES;
    mutations.truncate(MAXIMUM_DETAIL_ENTRIES);
    let mut resources = resource_summaries(replay);
    let resources_truncated = resources.len() > MAXIMUM_DETAIL_ENTRIES;
    resources.truncate(MAXIMUM_DETAIL_ENTRIES);
    let mut conflicts = conflict_summaries(applicable);
    let conflicts_truncated = conflicts.len() > MAXIMUM_DETAIL_ENTRIES;
    conflicts.truncate(MAXIMUM_DETAIL_ENTRIES);
    OverlayDetail {
        summary,
        base_instructions: OverlayBoundedText::from_text(
            &base.instructions,
            MAXIMUM_DETAIL_TEXT_CHARACTERS,
        ),
        effective_instructions: OverlayBoundedText::from_text(
            replay.effective().instructions(),
            MAXIMUM_DETAIL_TEXT_CHARACTERS,
        ),
        diff,
        scope_diffs,
        scope_diffs_truncated: false,
        mutations,
        mutations_truncated,
        resources,
        resources_truncated,
        conflicts,
        conflicts_truncated,
    }
}

fn build_scope_diffs(replay: &OverlayScopeReplay) -> Vec<OverlayScopeDiff> {
    let mut before = replay.base().instructions().to_string();
    replay
        .scope_results()
        .iter()
        .map(|result| {
            let after = result
                .output()
                .map_or(before.as_str(), |output| output.instructions());
            let output_hash = result.output_hash().unwrap_or(result.last_healthy_hash());
            let diff = build_bounded_diff(
                result.input_hash(),
                output_hash,
                &before,
                after,
                &format!("overlay-scope:{}", result.scope().as_str()),
                MAXIMUM_DETAIL_TEXT_CHARACTERS,
            );
            let scope_diff = OverlayScopeDiff {
                scope: result.scope(),
                revision: result.revision(),
                input_hash: result.input_hash().to_string(),
                output_hash: output_hash.to_string(),
                diff,
            };
            before = after.to_string();
            scope_diff
        })
        .collect()
}

fn build_summary(
    base: &OverlayEffectivePackageSnapshot,
    applicable: &[OverlayManifestSnapshot],
    pinned: bool,
    replay: &OverlayScopeReplay,
) -> OverlaySummary {
    let scopes = applicable
        .iter()
        .map(|snapshot| scope_summary(snapshot, base, replay))
        .collect::<Vec<_>>();
    let status = overall_status(&scopes);
    let needs_reconcile = scopes
        .iter()
        .any(|scope| scope.status == OverlayScopeStatus::NeedsReconciliation);
    let last_healthy_scope = scopes
        .iter()
        .rev()
        .find(|scope| scope.status == OverlayScopeStatus::Applied)
        .map(|scope| scope.scope);
    OverlaySummary {
        canonical_skill_id: base.canonical_skill_id.clone(),
        base_layer: base.base_layer,
        status,
        needs_reconcile,
        pinned,
        base_instruction_hash: base.instruction_hash.clone(),
        base_package_hash: base.package_hash.clone(),
        effective_hash: replay.effective().effective_hash().to_string(),
        last_healthy_scope,
        scopes,
        scopes_truncated: false,
    }
}

fn scope_summary(
    snapshot: &OverlayManifestSnapshot,
    base: &OverlayEffectivePackageSnapshot,
    replay: &OverlayScopeReplay,
) -> OverlayScopeSummary {
    let document = &snapshot.document;
    let trusted = document
        .trust()
        .is_trusted_for_revision(document.revision());
    let status = if !trusted {
        OverlayScopeStatus::Untrusted
    } else {
        replay
            .scope_results()
            .iter()
            .find(|result| {
                result.scope() == document.scope() && result.revision() == document.revision()
            })
            .map_or(
                OverlayScopeStatus::IntegrityFailure,
                |result| match result.status() {
                    OverlayScopeReplayStatus::Applied => OverlayScopeStatus::Applied,
                    OverlayScopeReplayStatus::Untrusted => OverlayScopeStatus::Untrusted,
                    OverlayScopeReplayStatus::NeedsReconciliation => {
                        OverlayScopeStatus::NeedsReconciliation
                    }
                    OverlayScopeReplayStatus::Conflict(_) => {
                        OverlayScopeStatus::NeedsReconciliation
                    }
                    OverlayScopeReplayStatus::IntegrityFailure(_) => {
                        OverlayScopeStatus::IntegrityFailure
                    }
                    OverlayScopeReplayStatus::Blocked { .. } => {
                        OverlayScopeStatus::BlockedByEarlierScope
                    }
                },
            )
    };
    let needs_reconcile = status == OverlayScopeStatus::NeedsReconciliation;
    OverlayScopeSummary {
        scope: document.scope(),
        revision: document.revision(),
        trust: document.trust().state(),
        status,
        active_mutation_count: active_mutation_count(document),
        conflict_count: document.conflicts.len(),
        base_hash_changed: base_hash_changed(document, base),
        needs_reconcile,
    }
}

fn base_hash_changed(
    document: &crate::contexts::tooling::skills::domain::OverlayDocument,
    base: &OverlayEffectivePackageSnapshot,
) -> bool {
    document.base_witness.base_identity != base.base_identity
        || document.base_witness.instruction_hash != base.instruction_hash
        || document.base_witness.package_hash != base.package_hash
}

fn active_mutation_count(
    document: &crate::contexts::tooling::skills::domain::OverlayDocument,
) -> usize {
    document
        .patches
        .iter()
        .filter(|item| item.state() == OverlayMutationState::Active)
        .count()
        + document
            .learn_blocks
            .iter()
            .filter(|item| item.state() == OverlayMutationState::Active)
            .count()
        + document
            .files
            .iter()
            .filter(|item| item.state() == OverlayMutationState::Active)
            .count()
}

fn overall_status(scopes: &[OverlayScopeSummary]) -> OverlayStatus {
    if scopes.is_empty() {
        return OverlayStatus::None;
    }
    if scopes
        .iter()
        .any(|scope| scope.status == OverlayScopeStatus::IntegrityFailure)
    {
        OverlayStatus::IntegrityFailure
    } else if scopes
        .iter()
        .any(|scope| scope.status == OverlayScopeStatus::BlockedByEarlierScope)
    {
        OverlayStatus::Blocked
    } else if scopes
        .iter()
        .any(|scope| scope.status == OverlayScopeStatus::NeedsReconciliation)
    {
        OverlayStatus::NeedsReconciliation
    } else if scopes
        .iter()
        .any(|scope| scope.status == OverlayScopeStatus::Applied)
    {
        OverlayStatus::Healthy
    } else {
        OverlayStatus::Untrusted
    }
}

fn mutation_summaries(applicable: &[OverlayManifestSnapshot]) -> Vec<OverlayMutationSummary> {
    let mut summaries = Vec::new();
    for snapshot in applicable {
        let document = &snapshot.document;
        summaries.extend(document.patches.iter().map(|item| OverlayMutationSummary {
            id: item.id.clone(),
            kind: OverlayMutationKind::Patch,
            scope: document.scope(),
            state: item.state(),
            created_at: item.created_at.clone(),
            updated_at: item.updated_at.clone(),
        }));
        summaries.extend(
            document
                .learn_blocks
                .iter()
                .map(|item| OverlayMutationSummary {
                    id: item.id.clone(),
                    kind: OverlayMutationKind::LearnedGuidance,
                    scope: document.scope(),
                    state: item.state(),
                    created_at: item.created_at.clone(),
                    updated_at: item.updated_at.clone(),
                }),
        );
        summaries.extend(document.files.iter().map(|item| OverlayMutationSummary {
            id: item.id.clone(),
            kind: OverlayMutationKind::SupportingFile,
            scope: document.scope(),
            state: item.state(),
            created_at: item.created_at.clone(),
            updated_at: item.updated_at.clone(),
        }));
    }
    summaries
}

fn conflict_summaries(applicable: &[OverlayManifestSnapshot]) -> Vec<OverlayConflictSummary> {
    applicable
        .iter()
        .flat_map(|snapshot| snapshot.document.conflicts.iter())
        .map(|conflict| OverlayConflictSummary {
            id: conflict.id().to_string(),
            mutation_id: conflict.mutation_id().to_string(),
            safe_reason: conflict.reason.clone(),
            state: conflict.state(),
            resolution_revision: conflict.resolution_revision(),
        })
        .collect()
}

fn resource_summaries(replay: &OverlayScopeReplay) -> Vec<OverlayResourceSummary> {
    replay
        .effective()
        .resources()
        .iter()
        .filter_map(|resource| {
            let EffectiveResourceSource::Overlay {
                scope, mutation_id, ..
            } = &resource.source
            else {
                return None;
            };
            Some(OverlayResourceSummary {
                mutation_id: mutation_id.clone(),
                logical_path: resource.logical_path.clone(),
                media_type: resource.media_type.clone(),
                size_bytes: resource.size_bytes,
                content_hash: resource.content_hash.clone(),
                effective_scope: *scope,
                state: OverlayMutationState::Active,
                shadowed: resource
                    .shadowed
                    .iter()
                    .map(|shadow| match &shadow.source {
                        EffectiveResourceSource::Base { layer } => OverlayResourceShadow {
                            scope: None,
                            base_layer: Some(*layer),
                            content_hash: shadow.content_hash.clone(),
                        },
                        EffectiveResourceSource::Overlay { scope, .. } => OverlayResourceShadow {
                            scope: Some(*scope),
                            base_layer: None,
                            content_hash: shadow.content_hash.clone(),
                        },
                    })
                    .collect(),
                shadowed_truncated: resource.shadowed_truncated,
            })
        })
        .collect()
}

fn preview_mutation_id(request: &OverlayMutationRequest) -> String {
    let mut hasher = Sha256::new();
    hasher.update(request.canonical_skill_id.as_str().as_bytes());
    hasher.update(request.scope.as_str().as_bytes());
    hasher.update(
        request
            .witnesses
            .expected_overlay_revision
            .unwrap_or_default()
            .to_le_bytes(),
    );
    match &request.mutation {
        OverlayMutation::ExactPatch {
            old_string,
            new_string,
            replace_all,
        } => {
            hasher.update(b"patch");
            hasher.update(old_string.as_bytes());
            hasher.update(new_string.as_bytes());
            hasher.update([u8::from(*replace_all)]);
        }
        OverlayMutation::LearnedGuidance { guidance } => {
            hasher.update(b"guidance");
            hasher.update(guidance.as_bytes());
        }
        OverlayMutation::SupportingFile {
            logical_path,
            media_type,
            content,
        } => {
            hasher.update(b"file");
            hasher.update(logical_path.as_bytes());
            hasher.update(media_type.as_bytes());
            hasher.update(content);
        }
        OverlayMutation::Disable { mutation_id } => {
            hasher.update(b"disable");
            hasher.update(mutation_id.as_bytes());
        }
        OverlayMutation::Revert { mutation_id } => {
            hasher.update(b"revert");
            hasher.update(mutation_id.as_bytes());
        }
    }
    let digest = hasher
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    format!("preview-{}", &digest[..16])
}
