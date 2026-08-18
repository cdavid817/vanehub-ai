import type {
  EvidenceOverview,
  EvidenceQueryInput,
  EvidenceSeedLineage,
  PurgeEvidenceInput,
  PurgeEvidenceOutcome,
} from "./agent-service";
import type { SkillDriftReport, SkillScopeInput, SkillSyncResult } from "../types/skill";
import type {
  SkillToolEnablementInput,
  SkillToolOwnerInput,
  SkillToolQuarantineInput,
  SkillToolRevision,
  SkillToolRevisionInput,
  SkillToolTrustInput,
} from "../types/skill-tools";

export interface SkillGovernanceService {
  listSkillTools(input: SkillToolOwnerInput): Promise<SkillToolRevision[]>;
  validateSkillToolRevision(input: SkillToolRevisionInput): Promise<SkillToolRevision>;
  setSkillToolTrust(input: SkillToolTrustInput): Promise<SkillToolRevision>;
  setSkillToolEnabled(input: SkillToolEnablementInput): Promise<SkillToolRevision>;
  quarantineSkillTool(input: SkillToolQuarantineInput): Promise<SkillToolRevision>;
  recoverSkillTool(input: SkillToolRevisionInput): Promise<SkillToolRevision>;
  getSkillToolDiagnostics(input: SkillToolRevisionInput): Promise<SkillToolRevision>;
  detectSkillDrift(input: SkillScopeInput): Promise<SkillDriftReport>;
  syncSkillDrift(input: SkillScopeInput): Promise<SkillSyncResult>;
}

export interface SkillEvidenceService {
  querySkillEvolutionEvidence(input: EvidenceQueryInput): Promise<EvidenceOverview>;
  getSkillEvolutionSeedLineage(seedId: string, input: EvidenceQueryInput): Promise<EvidenceSeedLineage | null>;
  purgeSkillEvolutionEvidence(input: PurgeEvidenceInput): Promise<PurgeEvidenceOutcome>;
}
