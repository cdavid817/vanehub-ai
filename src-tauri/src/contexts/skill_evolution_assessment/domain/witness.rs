use sha2::{Digest, Sha256};

use super::{EffectiveTargetWitness, SanitizedAssessmentInput};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AssessmentWitness {
    pub(crate) input: SanitizedAssessmentInput,
    pub(crate) targets: Vec<EffectiveTargetWitness>,
    pub(crate) selector_policy_version: String,
    pub(crate) lexical_policy_version: String,
    pub(crate) gate_policy_version: String,
    pub(crate) routing_policy_version: String,
    pub(crate) confidence_policy_version: String,
    pub(crate) consent_version: String,
    pub(crate) evaluator_configuration: Option<String>,
}

impl AssessmentWitness {
    pub(crate) fn canonical_hash(&self) -> String {
        let mut material = Vec::new();
        append(&mut material, "seed", &self.input.seed_id);
        append(&mut material, "revision", &self.input.seed_revision);
        append(&mut material, "fingerprint", &self.input.seed_fingerprint);
        append(&mut material, "lineage", &self.input.lineage_hash);
        append(
            &mut material,
            "workspace",
            self.input.workspace_id.as_deref().unwrap_or("global"),
        );
        append(&mut material, "sanitizer", &self.input.sanitizer_version);
        for evidence_id in sorted(&self.input.evidence_ids) {
            append(&mut material, "evidence", evidence_id);
        }
        for target in sorted_targets(&self.targets) {
            append(
                &mut material,
                "target",
                &format!(
                    "{}|{}|{:?}|{:?}|{:?}",
                    target.skill_id,
                    target.revision_hash,
                    target.scope,
                    target.lifecycle,
                    target.trust
                ),
            );
        }
        append(&mut material, "selector", &self.selector_policy_version);
        append(&mut material, "lexical", &self.lexical_policy_version);
        append(&mut material, "gate", &self.gate_policy_version);
        append(&mut material, "routing", &self.routing_policy_version);
        append(&mut material, "confidence", &self.confidence_policy_version);
        append(&mut material, "consent", &self.consent_version);
        append(
            &mut material,
            "evaluator",
            self.evaluator_configuration
                .as_deref()
                .unwrap_or("disabled"),
        );
        hex(Sha256::digest(material))
    }
}

fn sorted(values: &[String]) -> Vec<&str> {
    let mut values = values.iter().map(String::as_str).collect::<Vec<_>>();
    values.sort_unstable();
    values
}

fn sorted_targets(values: &[EffectiveTargetWitness]) -> Vec<&EffectiveTargetWitness> {
    let mut values = values.iter().collect::<Vec<_>>();
    values.sort_by(|left, right| {
        left.skill_id
            .cmp(&right.skill_id)
            .then(left.revision_hash.cmp(&right.revision_hash))
    });
    values
}

fn append(material: &mut Vec<u8>, key: &str, value: &str) {
    material.extend_from_slice(key.as_bytes());
    material.push(0);
    material.extend_from_slice(value.as_bytes());
    material.push(0);
}

fn hex(bytes: impl AsRef<[u8]>) -> String {
    bytes
        .as_ref()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}
