use serde::{Deserialize, Serialize};

pub(crate) const ASSESSMENT_SCHEMA_VERSION_V1: u16 = 1;

macro_rules! assessment_enum {
    ($name:ident { $($variant:ident),+ $(,)? }) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
        #[serde(rename_all = "snake_case")]
        pub(crate) enum $name { $($variant),+ }
    };
}

assessment_enum!(AssessmentAttemptStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Superseded
});
assessment_enum!(SelectionClassification {
    Selected,
    Ambiguous,
    NoTarget
});
assessment_enum!(QualityCheckResult {
    Pass,
    Fail,
    Review,
    NotApplicable
});
assessment_enum!(AssessmentRoute {
    Advance,
    Drop,
    RecordMemoryOnly,
    MergeDuplicate,
    NeedsHumanReview
});
assessment_enum!(AssessmentConfidence { Low, Medium, High });
assessment_enum!(AssessmentRisk { Low, Medium, High });
assessment_enum!(EvaluatorFallbackReason {
    DisabledConsent,
    ProviderUnavailable,
    Timeout,
    RateLimited,
    InvalidSchema,
    InventedTarget,
    MissingCitation,
    ProviderFailure,
});
assessment_enum!(TargetLifecycle {
    Active,
    Pinned,
    Archived,
    Missing,
    Malformed
});
assessment_enum!(TargetTrust {
    Trusted,
    Untrusted,
    Unknown
});
assessment_enum!(TargetScope {
    Project,
    User,
    Remote,
    System
});
assessment_enum!(TargetExclusionReason {
    Shadowed,
    Missing,
    Malformed,
    HistoricalOnly
});
assessment_enum!(EvidenceAttribution {
    Verified,
    Correlated,
    Weak,
    Unattributed
});
assessment_enum!(QualityCheckKind {
    PrivacyResidue,
    EvidenceSufficiency,
    DuplicateKnowledge,
    TransientIncident,
    GuidanceSpecificity,
    EvidenceConsistency,
    TargetCompatibility,
    ExecutableContentRisk,
    TargetLifecycleMutability,
});

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct SanitizedAssessmentInput {
    pub(crate) schema_version: u16,
    pub(crate) seed_id: String,
    pub(crate) seed_revision: String,
    pub(crate) seed_fingerprint: String,
    pub(crate) lineage_hash: String,
    pub(crate) workspace_id: Option<String>,
    pub(crate) sanitizer_version: String,
    pub(crate) evidence_ids: Vec<String>,
    pub(crate) attribution: EvidenceAttribution,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct EffectiveTargetWitness {
    pub(crate) skill_id: String,
    pub(crate) skill_type: String,
    pub(crate) revision_hash: String,
    pub(crate) scope: TargetScope,
    pub(crate) lifecycle: TargetLifecycle,
    pub(crate) trust: TargetTrust,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct RankedTarget {
    pub(crate) witness: EffectiveTargetWitness,
    pub(crate) score: u8,
    pub(crate) attribution_score: u8,
    pub(crate) participation_score: u8,
    pub(crate) compatibility_score: u8,
    pub(crate) lexical_score: u8,
    pub(crate) locality_score: u8,
    pub(crate) matched_feature_classes: Vec<String>,
    pub(crate) exclusions: Vec<TargetExclusionReason>,
    pub(crate) attribution_uncertain: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct SelectionThresholdWitness {
    pub(crate) leading_score: u8,
    pub(crate) runner_up_score: Option<u8>,
    pub(crate) margin: u8,
    pub(crate) selected_minimum: u8,
    pub(crate) ambiguous_minimum: u8,
    pub(crate) required_margin: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct LessonShape {
    pub(crate) trigger: Option<String>,
    pub(crate) required_behavior: Option<String>,
    pub(crate) prohibited_behavior: Option<String>,
    pub(crate) verification: Option<String>,
    pub(crate) environment: Option<String>,
    pub(crate) content_kinds: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct QualityCheck {
    pub(crate) kind: QualityCheckKind,
    pub(crate) result: QualityCheckResult,
    pub(crate) severity: AssessmentRisk,
    pub(crate) reason_code: String,
    pub(crate) evidence_ids: Vec<String>,
    pub(crate) route_constraints: Vec<AssessmentRoute>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ConfidenceBreakdown {
    pub(crate) policy_version: String,
    pub(crate) evidence_strength_bps: u16,
    pub(crate) selection_score_bps: u16,
    pub(crate) selection_margin_bps: u16,
    pub(crate) lineage_independence_bps: u16,
    pub(crate) check_completeness_bps: u16,
    pub(crate) contradiction_penalty_bps: u16,
    pub(crate) model_corroboration_bps: u16,
    pub(crate) system_confidence_bps: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct RoutingRuleWitness {
    pub(crate) rule_code: String,
    pub(crate) route: AssessmentRoute,
    pub(crate) matched: bool,
    pub(crate) reason_code: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct RoutingDecision {
    pub(crate) policy_version: String,
    pub(crate) route: AssessmentRoute,
    pub(crate) winning_rule: String,
    pub(crate) route_constraints: Vec<AssessmentRoute>,
    pub(crate) rules: Vec<RoutingRuleWitness>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct EvaluatorResult {
    pub(crate) consulted: bool,
    pub(crate) selected_target_id: Option<String>,
    pub(crate) confidence: Option<f32>,
    pub(crate) recommended_route: Option<AssessmentRoute>,
    pub(crate) cited_evidence_ids: Vec<String>,
    pub(crate) fallback_reason: Option<EvaluatorFallbackReason>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct AssessmentOutput {
    pub(crate) schema_version: u16,
    pub(crate) attempt_id: String,
    pub(crate) status: AssessmentAttemptStatus,
    pub(crate) classification: SelectionClassification,
    pub(crate) route: AssessmentRoute,
    pub(crate) confidence: AssessmentConfidence,
    pub(crate) risk: AssessmentRisk,
    pub(crate) targets: Vec<RankedTarget>,
    pub(crate) selection_threshold: SelectionThresholdWitness,
    pub(crate) attribution_uncertain: bool,
    pub(crate) lesson_shape: LessonShape,
    pub(crate) checks: Vec<QualityCheck>,
    pub(crate) evaluator: EvaluatorResult,
}
