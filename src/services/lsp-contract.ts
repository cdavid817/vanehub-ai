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
  type LspConfiguration,
  type LspLanguageConfiguration,
  type LspDistribution,
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
import {
  arrayValue,
  booleanValue,
  count,
  invalidResponse,
  isRecord,
  member,
  normalizeJsonObject,
  optionalString,
  optionalStringArray,
  optionalTimestamp,
  requiredString,
  stringArray,
  unique,
} from "./lsp-contract-values";

const maximumInitializationOptionsBytes = 32 * 1024;
function optionalReason(value: unknown): LspSafeReasonCode | null {
  if (value === null) return null;
  return member(lspSafeReasonCodes, value);
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
    distribution: normalizeDistribution(value.distribution),
    installed: booleanValue(value.installed),
  };
}

function normalizeDistribution(value: unknown): LspDistribution | null {
  if (value === null || value === undefined) return null;
  if (!isRecord(value)) return invalidResponse();
  return { verified: booleanValue(value.verified) };
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
