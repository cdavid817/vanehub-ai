import {
  lspDiscoveryAvailabilities,
  lspDocumentSyncModes,
  lspLanguageIdPattern,
  lspOverrideTargets,
  lspPositionEncodings,
  lspProcessStates,
  lspSafeReasonCodes,
  lspServerTestPhases,
  lspServerTestPhaseStatuses,
  type JsonObject,
  type JsonValue,
  type LspConfiguration,
  type LspLanguageConfiguration,
  type LspLanguageDescriptor,
  type LspNegotiatedCapabilities,
  type LspNegotiatedMethod,
  type LspSafeReasonCode,
  type LspServerDiscovery,
  type LspServerStatus,
  type LspServerTestInput,
  type LspServerTestPhaseResult,
  type LspServerTestResult,
  type LspWorkspaceTrust,
  type LspWorkspaceTrustUpdate,
} from "../types/lsp";

const maximumInitializationOptionsBytes = 32 * 1024;
const maximumJsonDepth = 32;
const maximumJsonItems = 1024;
const maximumListItems = 1024;
const rfc3339Pattern = /^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(?:\.\d+)?(?:Z|[+-]\d{2}:\d{2})$/;

function invalidResponse(): never {
  throw new Error("The runtime returned an invalid LSP response.");
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function isMember<T extends string>(values: readonly T[], value: unknown): value is T {
  return typeof value === "string" && values.some((candidate) => candidate === value);
}

function member<T extends string>(values: readonly T[], value: unknown): T {
  return isMember(values, value) ? value : invalidResponse();
}

function requiredString(value: unknown, maximumLength = 4096): string {
  if (typeof value !== "string" || value.trim() === "" || value.length > maximumLength) {
    return invalidResponse();
  }
  return value;
}

function optionalString(value: unknown): string | null {
  return value === null ? null : requiredString(value);
}

function booleanValue(value: unknown): boolean {
  return typeof value === "boolean" ? value : invalidResponse();
}

function count(value: unknown): number {
  if (typeof value !== "number" || !Number.isSafeInteger(value) || value < 0) {
    return invalidResponse();
  }
  return value;
}

function arrayValue(value: unknown, maximum = maximumListItems): readonly unknown[] {
  if (!Array.isArray(value) || value.length > maximum) return invalidResponse();
  return value;
}

function optionalTimestamp(value: unknown): string | null {
  if (value === null) return null;
  const timestamp = requiredString(value, 64);
  if (!rfc3339Pattern.test(timestamp) || Number.isNaN(Date.parse(timestamp))) {
    return invalidResponse();
  }
  return timestamp;
}

function optionalReason(value: unknown): LspSafeReasonCode | null {
  if (value === null) return null;
  return member(lspSafeReasonCodes, value);
}

function normalizeJsonValue(value: unknown, depth: number): JsonValue {
  if (depth > maximumJsonDepth) return invalidResponse();
  if (value === null || typeof value === "string" || typeof value === "boolean") return value;
  if (typeof value === "number") return Number.isFinite(value) ? value : invalidResponse();
  if (Array.isArray(value)) {
    if (value.length > maximumJsonItems) return invalidResponse();
    return value.map((item) => normalizeJsonValue(item, depth + 1));
  }
  return normalizeJsonObject(value, depth + 1);
}

function normalizeJsonObject(value: unknown, depth = 0): JsonObject {
  if (!isRecord(value) || depth > maximumJsonDepth) return invalidResponse();
  const entries = Object.entries(value);
  if (entries.length > maximumJsonItems) return invalidResponse();
  const normalized: [string, JsonValue][] = entries.map(([key, item]) => [
    requiredString(key, 256), normalizeJsonValue(item, depth + 1),
  ]);
  return Object.fromEntries<JsonValue>(normalized);
}

function initializationOptions(value: unknown): JsonObject {
  const result = normalizeJsonObject(value);
  if (new TextEncoder().encode(JSON.stringify(result)).length > maximumInitializationOptionsBytes) {
    return invalidResponse();
  }
  return result;
}

/** The same shape the backend enforces, so a malformed id is refused at both ends. */
function identifier(value: unknown): string {
  return typeof value === "string" && lspLanguageIdPattern.test(value)
    ? value : invalidResponse();
}

function optionalStringArray(value: unknown, maximum = 32): string[] | null {
  return value === null ? null : arrayValue(value, maximum).map((item) => requiredString(item));
}

function normalizeLanguageConfiguration(value: unknown): LspLanguageConfiguration {
  if (!isRecord(value)) return invalidResponse();
  return {
    language: identifier(value.language),
    enabled: booleanValue(value.enabled),
    executableOverride: optionalString(value.executableOverride),
    startupArguments: optionalStringArray(value.startupArguments),
    initializationOptions: initializationOptions(value.initializationOptions),
  };
}

function normalizeDescriptor(value: unknown): LspLanguageDescriptor {
  if (!isRecord(value)) return invalidResponse();
  return {
    language: identifier(value.language),
    server: identifier(value.server),
    supportedOnHost: booleanValue(value.supportedOnHost),
    defaultStartupArguments: arrayValue(value.defaultStartupArguments, 32)
      .map((item) => requiredString(item)),
    overrideTarget: member(lspOverrideTargets, value.overrideTarget),
    prerequisite: value.prerequisite === null ? null : requiredString(value.prerequisite),
  };
}

function unique(ids: readonly string[]): boolean {
  return new Set(ids).size === ids.length;
}

export function normalizeLspConfiguration(value: unknown): LspConfiguration {
  if (!isRecord(value)) return invalidResponse();
  const languages = arrayValue(value.languages, 64).map(normalizeLanguageConfiguration);
  const descriptors = arrayValue(value.descriptors, 64).map(normalizeDescriptor);
  const declared = new Set(descriptors.map((entry) => entry.language));
  // Configuration for a language the same response does not describe cannot be rendered, so it is
  // refused here rather than surfacing as a control with no label. The check replaces the fixed
  // "exactly these two, in this order" rule, which only held while the set was compiled in.
  if (!unique(languages.map((entry) => entry.language))
    || !unique(descriptors.map((entry) => entry.language))
    || languages.some((entry) => !declared.has(entry.language))) {
    return invalidResponse();
  }
  return { enabled: booleanValue(value.enabled), languages, descriptors };
}

export function normalizeLspWorkspaceTrust(value: unknown): LspWorkspaceTrust {
  if (!isRecord(value)) return invalidResponse();
  return {
    canonicalRoot: requiredString(value.canonicalRoot),
    trusted: booleanValue(value.trusted),
    revision: count(value.revision),
  };
}

export function normalizeLspWorkspaceTrustList(value: unknown): LspWorkspaceTrust[] {
  const records = arrayValue(value).map(normalizeLspWorkspaceTrust);
  if (new Set(records.map((record) => record.canonicalRoot)).size !== records.length) {
    return invalidResponse();
  }
  return records;
}

export function normalizeLspWorkspaceTrustUpdate(value: unknown): LspWorkspaceTrustUpdate {
  if (!isRecord(value)) return invalidResponse();
  return {
    canonicalRoot: requiredString(value.canonicalRoot),
    trusted: booleanValue(value.trusted),
  };
}

function stringArray(value: unknown, maximum = 16): string[] {
  return arrayValue(value, maximum).map((item) => requiredString(item, 1024));
}

function normalizeDiscovery(value: unknown): LspServerDiscovery {
  if (!isRecord(value)) return invalidResponse();
  const language = identifier(value.language);
  const server = identifier(value.server);
  const availability = member(lspDiscoveryAvailabilities, value.availability);
  const executablePath = optionalString(value.executablePath);
  const reasonCode = optionalReason(value.reasonCode);
  if ((availability === "available" && (executablePath === null || reasonCode !== null))
    || (availability === "unavailable" && (executablePath !== null || reasonCode === null))) {
    return invalidResponse();
  }
  return { language, server, availability, executablePath,
    arguments: stringArray(value.arguments), reasonCode };
}

export function normalizeLspServerDiscoveries(value: unknown): LspServerDiscovery[] {
  const records = arrayValue(value, 64).map(normalizeDiscovery);
  if (!unique(records.map((record) => record.language))) return invalidResponse();
  return records;
}

export function normalizeLspServerTestInput(value: unknown): LspServerTestInput {
  if (!isRecord(value)) return invalidResponse();
  return { language: identifier(value.language) };
}

function normalizeNegotiatedMethod(value: unknown): LspNegotiatedMethod {
  if (!isRecord(value)) return invalidResponse();
  return { method: identifier(value.method), supported: booleanValue(value.supported) };
}

function normalizeCapabilities(value: unknown): LspNegotiatedCapabilities {
  if (!isRecord(value)) return invalidResponse();
  const methods = arrayValue(value.methods, 64).map(normalizeNegotiatedMethod);
  // A duplicated method would render twice and let two rows disagree about the same fact.
  if (!unique(methods.map((entry) => entry.method))) return invalidResponse();
  return {
    positionEncoding: member(lspPositionEncodings, value.positionEncoding),
    documentSync: member(lspDocumentSyncModes, value.documentSync),
    methods,
  };
}

function optionalCapabilities(value: unknown): LspNegotiatedCapabilities | null {
  return value === null ? null : normalizeCapabilities(value);
}

function normalizeTestPhase(value: unknown): LspServerTestPhaseResult {
  if (!isRecord(value)) return invalidResponse();
  const status = member(lspServerTestPhaseStatuses, value.status);
  const reasonCode = optionalReason(value.reasonCode);
  if ((status === "failed") !== (reasonCode !== null)) {
    return invalidResponse();
  }
  return { phase: member(lspServerTestPhases, value.phase), status, reasonCode };
}

export function normalizeLspServerTestResult(value: unknown): LspServerTestResult {
  if (!isRecord(value)) return invalidResponse();
  const server = identifier(value.server);
  const phases = arrayValue(value.phases, lspServerTestPhases.length).map(normalizeTestPhase);
  if (phases.length !== lspServerTestPhases.length
    || new Set(phases.map((phase) => phase.phase)).size !== phases.length) return invalidResponse();
  return {
    server,
    phases: lspServerTestPhases.map((phase) => (
      phases.find((entry) => entry.phase === phase) ?? invalidResponse()
    )),
    negotiatedCapabilities: optionalCapabilities(value.negotiatedCapabilities),
  };
}

function normalizeServerStatus(value: unknown): LspServerStatus {
  if (!isRecord(value)) return invalidResponse();
  return {
    language: identifier(value.language), server: identifier(value.server), relativeProjectRoot: requiredString(value.relativeProjectRoot),
    state: member(lspProcessStates, value.state), restartCount: count(value.restartCount),
    lastResponseAt: optionalTimestamp(value.lastResponseAt),
    diagnosticCount: count(value.diagnosticCount), reasonCode: optionalReason(value.reasonCode),
    negotiatedCapabilities: optionalCapabilities(value.negotiatedCapabilities),
  };
}

export function normalizeLspServerStatuses(value: unknown): LspServerStatus[] {
  const statuses = arrayValue(value).map(normalizeServerStatus);
  const identities = statuses.map((status) => (
    `${status.language}\u0000${status.server}\u0000${status.relativeProjectRoot}`
  ));
  if (!unique(identities)) return invalidResponse();
  return statuses;
}
