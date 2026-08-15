import type { SkillScopeInput } from "./skill";

// Configuration records live at User or Project scope only: defaults come from the schema, so a
// third writable layer would have no owner and no reset semantics.
export type SkillConfigurationScope = "user" | "project";

export type SkillConfigurationProvenance = "project" | "user" | "default" | "missing";

export type SkillConfigurationSecretState = "configured" | "missing" | "error";

export type SkillConfigurationDrift = "compatible" | "migration-required" | "invalid";

export type SkillConfigurationReadiness =
  | "ready"
  | "missing-required"
  | "migration-required"
  | "invalid"
  | "not-configurable";

// "not-evaluated" is distinct from "not-configurable": a Skill read without its package cannot
// answer the question, and reporting it as unconfigurable would hide a configurable Skill.
export type SkillConfigurableState =
  | "not-evaluated"
  | "not-configurable"
  | "configurable"
  | "schema-unsupported";

export type SkillConfigurationCleanupState = "none" | "pending" | "failed";

export type SkillConfigurationConsumption = "native" | "unsupported-external-cli";

export type SkillConfigurationScalarType = "string" | "integer" | "number" | "boolean";

export type SkillConfigurationFieldKind = "scalar" | "list";

export interface SkillConfigurationFieldType {
  kind: SkillConfigurationFieldKind;
  scalar: SkillConfigurationScalarType;
}

// Tagged rather than a bare JSON value: 3 and 3.0 are the same JSON number, and an integer
// property that a later schema retypes to a number has to stay distinguishable from one that was
// always a number.
export type SkillConfigurationValue =
  | { type: "string"; value: string }
  | { type: "integer"; value: number }
  | { type: "number"; value: number }
  | { type: "boolean"; value: boolean }
  | { type: "list"; value: SkillConfigurationValue[] };

export interface SkillConfigurationValueEntry {
  key: string;
  value: SkillConfigurationValue;
}

export interface SkillConfigurationConstraints {
  minimum: number | null;
  maximum: number | null;
  minLength: number | null;
  maxLength: number | null;
  minItems: number | null;
  maxItems: number | null;
}

export interface SkillConfigurationPresentation {
  label: string | null;
  labelKey: string | null;
  help: string | null;
  order: number | null;
  advanced: boolean;
  multiline: boolean;
}

export interface SkillConfigurationGroupDescriptor {
  key: string;
  declaredKey: string;
  presentation: SkillConfigurationPresentation;
}

export interface SkillConfigurationFieldDescriptor {
  key: string;
  declaredKey: string;
  group: string | null;
  fieldType: SkillConfigurationFieldType;
  required: boolean;
  secret: boolean;
  defaultValue: SkillConfigurationValue | null;
  choices: SkillConfigurationValue[];
  constraints: SkillConfigurationConstraints;
  presentation: SkillConfigurationPresentation;
  description: string | null;
}

export interface SkillConfigurationDescriptor {
  schemaHash: string;
  groups: SkillConfigurationGroupDescriptor[];
  fields: SkillConfigurationFieldDescriptor[];
}

// A secret property reports presence and nothing else: `value` stays null even when the credential
// exists, and no response carries the secret or its credential alias.
export interface SkillConfigurationProperty {
  key: string;
  value: SkillConfigurationValue | null;
  provenance: SkillConfigurationProvenance;
  secret: boolean;
  secretState: SkillConfigurationSecretState | null;
}

export interface SkillConfigurationScopeState {
  scope: SkillConfigurationScope;
  storedRevision: number;
  schemaHash: string;
  drift: SkillConfigurationDrift;
  configuredPropertyCount: number;
  configuredSecretCount: number;
}

export interface SkillConfigurationResolution {
  properties: SkillConfigurationProperty[];
  drift: SkillConfigurationDrift;
  readiness: SkillConfigurationReadiness;
  scopes: SkillConfigurationScopeState[];
}

export interface SkillConfigurationRecordState {
  scope: SkillConfigurationScope;
  workspaceIdentity: string | null;
  schemaHash: string;
  baseRevision: string;
  storedRevision: number;
  validationState: SkillConfigurationDrift;
  values: SkillConfigurationValueEntry[];
  secretKeys: string[];
  orphanedAt: string | null;
  cleanupState: SkillConfigurationCleanupState;
}

// Stated per binding kind because there is no bridge: an external CLI receives the package without
// values, and the UI has to say so rather than let a user assume otherwise.
export interface SkillConfigurationRuntimeSupport {
  nativeApi: SkillConfigurationConsumption;
  externalCli: SkillConfigurationConsumption;
}

export interface SkillConfigurationView {
  skillId: string;
  state: SkillConfigurableState;
  unsupportedReason: string | null;
  schemaHash: string | null;
  baseRevision: string | null;
  workspaceIdentity: string | null;
  availableScopes: SkillConfigurationScope[];
  descriptor: SkillConfigurationDescriptor | null;
  resolution: SkillConfigurationResolution | null;
  readiness: SkillConfigurationReadiness;
  drift: SkillConfigurationDrift | null;
  runtime: SkillConfigurationRuntimeSupport;
}

// Property names only. They are already public in the Skill package; values and credential aliases
// are not, and a cleanup report is exactly where one would otherwise leak.
export type SkillConfigurationRecovery =
  | { status: "clean" }
  | { status: "incomplete"; properties: string[] };

export type SkillConfigurationRejectionCode =
  | "unknown-property"
  | "not-configurable"
  | "invalid-value"
  | "payload-too-large"
  | "schema-changed"
  | "base-revision-changed"
  | "stale"
  | "invalid-workspace"
  | "credential-failure"
  | "repository-failure"
  | "recovery-required"
  | "reconciliation-incomplete";

export interface SkillConfigurationRejection {
  code: SkillConfigurationRejectionCode;
  message: string;
  propertyKey: string | null;
  properties: string[];
  expected: string | null;
  actual: string | null;
  bytes: number | null;
  /** Present when the write lost a compare-and-swap, so an editor can refresh from the winner. */
  current: SkillConfigurationRecordState | null;
}

export interface SkillConfigurationSaveResult {
  record: SkillConfigurationRecordState;
  preview: SkillConfigurationResolution;
  recovery: SkillConfigurationRecovery;
}

export type SkillConfigurationSaveOutcome =
  | { status: "saved"; result: SkillConfigurationSaveResult }
  | { status: "rejected"; rejection: SkillConfigurationRejection };

export type SkillConfigurationResolutionOutcome =
  | { status: "resolved"; resolution: SkillConfigurationResolution }
  | { status: "rejected"; rejection: SkillConfigurationRejection };

export type SkillConfigurationRetentionOutcome =
  | { status: "applied"; recovery: SkillConfigurationRecovery }
  | { status: "rejected"; rejection: SkillConfigurationRejection };

// Three-way intent rather than a nullable value: round-tripping a stored secret would require
// reading it back out of the credential store and through a response.
export type SkillConfigurationSecretIntent =
  | { action: "preserve" }
  | { action: "replace"; value: string }
  | { action: "clear" };

export interface SkillConfigurationSecretIntentInput {
  key: string;
  intent: SkillConfigurationSecretIntent;
}

export interface SkillConfigurationWriteInput extends SkillScopeInput {
  configScope: SkillConfigurationScope;
  schemaHash: string;
  baseRevision: string;
  /** `null` asserts that the scope has no stored record yet; any mismatch is stale. */
  expectedRevision?: number | null;
  values?: SkillConfigurationValueEntry[];
  secretIntents?: SkillConfigurationSecretIntentInput[];
}

export interface SkillConfigurationScopeInput extends SkillScopeInput {
  configScope: SkillConfigurationScope;
}

export type SkillConfigurationRetention = "retain" | "delete";

export type SkillConfigurationObsoleteAction =
  | { action: "mapTo"; targetKey: string }
  | { action: "discard" };

export interface SkillConfigurationObsoleteFieldChoice {
  key: string;
  choice: SkillConfigurationObsoleteAction;
}

export interface SkillConfigurationReconciliationPlan {
  fields?: SkillConfigurationObsoleteFieldChoice[];
  clearSecrets?: string[];
}

export interface SkillConfigurationReconciliationInput {
  request: SkillConfigurationWriteInput;
  plan?: SkillConfigurationReconciliationPlan;
}
