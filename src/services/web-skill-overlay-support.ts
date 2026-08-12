import type { SkillLayer } from "../types/skill";
import type {
  SkillOverlayBoundedText,
  SkillOverlayConflictSummary,
  SkillOverlayDiff,
  SkillOverlayHistoryAction,
  SkillOverlayHistoryEntry,
  SkillOverlayMutationKind,
  SkillOverlayMutationState,
  SkillOverlayMutationSummary,
  SkillOverlayResourceSummary,
  SkillOverlayScanResult,
  SkillOverlayScope,
  SkillOverlayTargetInput,
  SkillOverlayTrust,
  SkillOverlayWitnesses,
} from "../types/skill-overlay";
import type { SkillOverlayServiceError } from "../types/skill-overlay-reconciliation";

export interface WebOverlayBase {
  skillId: string;
  layer: SkillLayer;
  instructions: string;
  instructionHash: string;
  packageHash: string;
  pinned: boolean;
}

export interface WebOverlayMutation extends SkillOverlayMutationSummary {
  oldString?: string;
  newString?: string;
  replaceAll?: boolean;
  guidance?: string;
  logicalPath?: string;
  mediaType?: string;
  content?: number[];
}

export interface WebOverlayState {
  target: SkillOverlayTargetInput;
  revision: number;
  trust: SkillOverlayTrust;
  baseInstructionHash: string;
  basePackageHash: string;
  documentHash: string;
  sourceSummary: string | null;
  mutations: WebOverlayMutation[];
  conflicts: SkillOverlayConflictSummary[];
  history: SkillOverlayHistoryEntry[];
  historyIntegrity: boolean;
}

export function overlayKey(target: SkillOverlayTargetInput): string {
  const workspace = target.scope === "project" ? target.workspacePath ?? "" : "";
  return `${target.scope}:${workspace}:${target.skillId}`;
}

export function webOverlayHash(value: string): string {
  let hash = 2_166_136_261;
  for (const character of value) {
    hash ^= character.codePointAt(0) ?? 0;
    hash = Math.imul(hash, 16_777_619);
  }
  return `web-overlay-${(hash >>> 0).toString(16).padStart(8, "0")}`;
}

export function boundedText(content: string): SkillOverlayBoundedText {
  const characters = [...content];
  return {
    content: characters.slice(0, 12_000).join(""),
    totalCharacters: characters.length,
    truncated: characters.length > 12_000,
  };
}

export function overlayDiff(before: string, after: string): SkillOverlayDiff {
  return {
    baseHash: webOverlayHash(before),
    effectiveHash: webOverlayHash(after),
    addedCharacters: Math.max(0, [...after].length - [...before].length),
    removedCharacters: Math.max(0, [...before].length - [...after].length),
    hunks: before === after ? [] : [{ label: "effective-instructions", before: boundedText(before), after: boundedText(after) }],
    hunksTruncated: false,
  };
}

export function overlayScan(passed = true, safeRuleIds: string[] = []): SkillOverlayScanResult {
  return { scannerVersion: "web-overlay-scan-v1", passed, safeRuleIds, ruleIdsTruncated: false };
}

export function overlayError(
  kind: SkillOverlayServiceError["kind"],
  code: string,
  message: string,
  values: Partial<SkillOverlayServiceError> = {},
): SkillOverlayServiceError {
  return {
    kind,
    code,
    message,
    expectedRevision: null,
    currentRevision: null,
    maximum: null,
    actual: null,
    baseChanged: null,
    payloadChanged: null,
    pinChanged: null,
    ...values,
  };
}

export function assertOverlayWitnesses(
  state: WebOverlayState | undefined,
  base: WebOverlayBase,
  witnesses: SkillOverlayWitnesses,
): void {
  const currentRevision = state?.revision ?? null;
  const baseChanged = witnesses.expectedBaseInstructionHash !== base.instructionHash
    || witnesses.expectedBasePackageHash !== base.packageHash;
  const payloadChanged = witnesses.expectedPayloadHash != null
    && witnesses.expectedPayloadHash !== state?.documentHash;
  const pinChanged = witnesses.expectedPinned !== base.pinned;
  if (witnesses.expectedOverlayRevision !== currentRevision || baseChanged || payloadChanged || pinChanged) {
    throw overlayError("stale", "stale-witnesses", "Skill Overlay witnesses are stale.", {
      expectedRevision: witnesses.expectedOverlayRevision,
      currentRevision,
      baseChanged,
      payloadChanged,
      pinChanged,
    });
  }
}

export function assertOverlayMutable(base: WebOverlayBase): void {
  if (base.pinned) {
    throw overlayError("pinned", "skill-pinned", "Unpin the Skill before changing its Overlay.");
  }
}

export function applyOverlayMutations(
  base: string,
  state: WebOverlayState | undefined,
): { instructions: string; conflicts: SkillOverlayConflictSummary[] } {
  if (!state || state.trust === "untrusted") return { instructions: base, conflicts: state?.conflicts ?? [] };
  let instructions = base;
  const conflicts: SkillOverlayConflictSummary[] = [];
  for (const mutation of state.mutations.filter((value) => value.state === "active" && value.kind === "patch")) {
    const oldString = mutation.oldString ?? "";
    const matches = oldString ? instructions.split(oldString).length - 1 : 0;
    if (matches === 0 || (!mutation.replaceAll && matches !== 1)) {
      conflicts.push(conflictFor(mutation.id, state.revision));
      continue;
    }
    instructions = mutation.replaceAll
      ? instructions.replaceAll(oldString, mutation.newString ?? "")
      : instructions.replace(oldString, mutation.newString ?? "");
  }
  if (conflicts.length > 0 || state.conflicts.some((value) => value.state === "active")) {
    return { instructions: base, conflicts: conflicts.length > 0 ? conflicts : state.conflicts };
  }
  const guidance = state.mutations
    .filter((value) => value.state === "active" && value.kind === "learnedGuidance")
    .map((value) => value.guidance ?? "");
  if (guidance.length > 0) {
    instructions += `\n\n## User-learned guidance overlay\n\n${guidance.join("\n\n")}`;
  }
  return { instructions, conflicts: [] };
}

export function conflictFor(mutationId: string, revision: number): SkillOverlayConflictSummary {
  return {
    id: `conflict-${mutationId}`,
    mutationId,
    safeReason: "exact-match-contract-failed",
    state: "active",
    resolutionRevision: revision,
  };
}

export function resourceSummaries(state: WebOverlayState | undefined): SkillOverlayResourceSummary[] {
  if (!state || state.trust === "untrusted") return [];
  return state.mutations
    .filter((value) => value.kind === "supportingFile")
    .map((value) => ({
      mutationId: value.id,
      logicalPath: value.logicalPath ?? "",
      mediaType: value.mediaType ?? "application/octet-stream",
      sizeBytes: value.content?.length ?? 0,
      contentHash: webOverlayHash((value.content ?? []).join(",")),
      effectiveScope: value.scope,
      state: value.state,
      shadowed: [],
      shadowedTruncated: false,
    }));
}

export function appendOverlayHistory(
  state: WebOverlayState,
  action: SkillOverlayHistoryAction,
  priorRevision: number | null,
  priorDocumentHash: string | null,
): void {
  const priorEventHash = state.history.at(-1)?.eventHash ?? null;
  const eventHash = webOverlayHash(`${priorEventHash}:${state.documentHash}:${action}:${state.revision}`);
  state.history.push({
    eventId: `event-${state.revision}-${state.history.length + 1}`,
    canonicalSkillId: state.target.skillId,
    scope: state.target.scope,
    priorRevision,
    nextRevision: state.revision,
    actor: "user",
    action,
    timestamp: new Date().toISOString(),
    priorDocumentHash,
    nextDocumentHash: state.documentHash,
    scannerVersion: "web-overlay-scan-v1",
    safeOutcome: "committed",
    priorEventHash,
    eventHash,
  });
}

export function newMutation(
  kind: SkillOverlayMutationKind,
  scope: SkillOverlayScope,
  sequence: number,
  values: Partial<WebOverlayMutation>,
): WebOverlayMutation {
  const timestamp = new Date().toISOString();
  return { id: `mutation-${sequence}`, kind, scope, state: "active", createdAt: timestamp, updatedAt: timestamp, ...values };
}

export function setMutationState(mutation: WebOverlayMutation, state: SkillOverlayMutationState): void {
  mutation.state = state;
  mutation.updatedAt = new Date().toISOString();
}
