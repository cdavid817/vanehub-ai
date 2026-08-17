import type { SkillScopeInput } from "./skill";

export type SkillToolValidationState = "pending" | "valid" | "invalid";
export type SkillToolImplementationKind = "declarative" | "wasm";
export type SkillToolRuntimeSupport = "supported" | "module-runtime-disabled" | "unsupported-web-runtime";

export interface SkillToolOwnerInput extends SkillScopeInput {
  skillId: string;
}

export interface SkillToolDiagnostic {
  severity: "info" | "warn" | "error";
  code: string;
  detail: string;
}

export interface SkillToolCapabilityDiff {
  previousDigest?: string;
  currentDigest: string;
  added: string[];
  removed: string[];
  changed: boolean;
}

export interface SkillToolIntegrity {
  baseRevision: string;
  manifestHash: string;
  implementationHash: string;
  capabilityDigest: string;
}

export interface SkillToolRevision {
  skillId: string;
  toolId: string;
  canonicalId: string;
  revision: string;
  sourceScope: "global" | "workspace";
  workspacePath?: string;
  implementationKind: SkillToolImplementationKind;
  baseRevision: string;
  manifestHash: string;
  implementationHash: string;
  capabilityDigest: string;
  capabilityDiff?: SkillToolCapabilityDiff;
  validation: SkillToolValidationState;
  validationCode?: string;
  trusted: boolean;
  enabled: boolean;
  quarantined: boolean;
  quarantineReason?: string;
  consecutiveFailures: number;
  diagnostics: SkillToolDiagnostic[];
  runtimeSupport: SkillToolRuntimeSupport;
  enforcementStrength: "wasm-hard-limits" | "bounded-native-io";
  createdAt: string;
  updatedAt: string;
}

export interface SkillToolRevisionInput {
  revision: string;
}

export interface SkillToolTrustInput extends SkillToolRevisionInput {
  trusted: boolean;
  actor: string;
}

export interface SkillToolEnablementInput extends SkillToolRevisionInput {
  enabled: boolean;
}

export interface SkillToolQuarantineInput extends SkillToolRevisionInput {
  reason: string;
}
