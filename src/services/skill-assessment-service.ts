export type AssessmentAttemptStatus = "pending" | "running" | "completed" | "failed" | "superseded";
export type AssessmentClassification = "selected" | "ambiguous" | "no_target";
export type AssessmentRoute = "advance" | "drop" | "record_memory_only" | "merge_duplicate" | "needs_human_review";
export type AssessmentConfidence = "low" | "medium" | "high";
export type AssessmentRisk = "low" | "medium" | "high";
export type AssessmentCheckResult = "pass" | "fail" | "review" | "not_applicable";

export interface AssessmentQueryInput {
  workspace?: string;
  skillId?: string;
  seedId?: string;
  includeHistory?: boolean;
  limit?: number;
  cursor?: string;
}

export interface AssessmentScoreComponent {
  component: "attribution" | "participation" | "compatibility" | "lexical" | "locality";
  score: number;
}

export interface AssessmentTarget {
  ordinal: number;
  skillId: string;
  skillType: "role" | "utility" | "unknown";
  revisionHash: string;
  scope: "project" | "user" | "built_in";
  lifecycle: "active" | "pinned" | "archived" | "missing";
  trust: "trusted" | "untrusted" | "quarantined";
  score: number;
  attribution: "verified" | "correlated" | "weak" | "unattributed";
  attributionUncertain: boolean;
  matchedFeatureClasses: string[];
  exclusions: string[];
  components: AssessmentScoreComponent[];
}

export interface AssessmentSelectionThreshold {
  leadingScore: number;
  runnerUpScore?: number;
  margin: number;
  selectedMinimum: number;
  ambiguousMinimum: number;
  requiredMargin: number;
}

export interface AssessmentVersionWitnesses {
  witnessHash: string;
  lineageHash: string;
  targetUniverseHash: string;
  sanitizerVersion: string;
  selectorPolicyVersion: string;
  gatePolicyVersion: string;
  routingPolicyVersion: string;
  confidencePolicyVersion: string;
  consentVersion: string;
}

export interface AssessmentCheck {
  ordinal: number;
  kind: string;
  result: AssessmentCheckResult;
  severity: AssessmentRisk;
  reasonCode: string;
  evidenceIds: string[];
  routeConstraints: AssessmentRoute[];
}

export interface AssessmentProvenance {
  deterministic: boolean;
  modelEvaluationAllowed: boolean;
  modelConsulted: boolean;
  fallbackReason?: string;
  providerProtocol?: string;
  modelId?: string;
  templateVersion?: string;
  responseSchemaVersion?: string;
}

export interface AssessmentSummary {
  attemptId: string;
  seedId: string;
  seedRevision: string;
  status: AssessmentAttemptStatus;
  classification?: AssessmentClassification;
  route?: AssessmentRoute;
  confidence?: AssessmentConfidence;
  risk?: AssessmentRisk;
  isCurrent: boolean;
  winningRule?: string;
  createdAtMs: number;
  completedAtMs?: number;
  supersededByAttemptId?: string;
  supersessionReason?: string;
  changedWitnessHash?: string;
}

export interface AssessmentDetail extends AssessmentSummary {
  targets: AssessmentTarget[];
  checks: AssessmentCheck[];
  provenance: AssessmentProvenance;
  routeConstraints: AssessmentRoute[];
  selectionThreshold?: AssessmentSelectionThreshold;
  versionWitnesses: AssessmentVersionWitnesses;
}

export interface AssessmentPage {
  items: AssessmentSummary[];
  nextCursor?: string;
}

export interface AssessmentPolicyStatus {
  evaluatorPolicyVersion: string;
  disclosureVersion: string;
  modelEvaluationEnabled: boolean;
  providerAvailable: boolean;
  changedAtMs: number;
}

export interface UpdateAssessmentConsentInput {
  enabled: boolean;
  evaluatorPolicyVersion: string;
  disclosureVersion: string;
}

export interface ReassessmentRequest {
  seedId: string;
  expectedWitnessHash?: string;
}

export interface ReassessmentReceipt {
  queueId: string;
  status: "scheduled" | "coalesced" | "disabled" | "saturated";
}

export interface SkillAssessmentService {
  querySkillEvolutionAssessments(input: AssessmentQueryInput): Promise<AssessmentPage>;
  getSkillEvolutionAssessment(attemptId: string): Promise<AssessmentDetail | null>;
  getSkillEvolutionAssessmentPolicy(): Promise<AssessmentPolicyStatus>;
  updateSkillEvolutionAssessmentConsent(input: UpdateAssessmentConsentInput): Promise<AssessmentPolicyStatus>;
  scheduleSkillEvolutionReassessment(input: ReassessmentRequest): Promise<ReassessmentReceipt>;
}
