import {
  codeIndexLanguages,
  codeIndexPhases,
  type CodeEmbeddingConfirmation,
  type CodeIndexAuditEntry,
  type CodeIndexAuditEvent,
  type CodeIndexAuditReason,
  type CodeIndexConfigurationInput,
  type CodeIndexLanguage,
  type CodeIndexStatus,
  type CodeIndexWorkspace,
} from "../types/code-index";

const auditEvents: readonly CodeIndexAuditEvent[] = [
  "admitted", "skipped", "indexed", "failed", "deleted", "rebuilt",
];
const auditReasons: readonly CodeIndexAuditReason[] = [
  "outside_selected_roots", "sensitive_file", "user_excluded", "language_disabled",
  "size_limit", "binary", "unreadable", "parse", "auth", "invalid_request",
  "rate_limit", "network", "stale_generation",
];
const maximumFileBytes = 10 * 1024 * 1024;

function invalidResponse(): never {
  throw new Error("The runtime returned an invalid code-index response.");
}

function invalidConfiguration(message: string): never {
  throw new Error(`Invalid code-index configuration: ${message}.`);
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function isMember<T extends string>(values: readonly T[], value: unknown): value is T {
  return typeof value === "string" && values.some((candidate) => candidate === value);
}

function requiredString(value: unknown): string {
  if (typeof value !== "string" || value.trim() === "") return invalidResponse();
  return value;
}

function optionalString(value: unknown): string | null {
  if (value === null) return null;
  return requiredString(value);
}

function count(value: unknown): number {
  if (typeof value !== "number" || !Number.isSafeInteger(value) || value < 0) {
    return invalidResponse();
  }
  return value;
}

function timestamp(value: unknown): string {
  const result = requiredString(value);
  if (Number.isNaN(Date.parse(result))) return invalidResponse();
  return result;
}

function stringArray(value: unknown, allowEmptyItems = false): string[] {
  if (!Array.isArray(value)) return invalidResponse();
  return value.map((item) => {
    if (typeof item !== "string" || (!allowEmptyItems && item.length === 0)) {
      return invalidResponse();
    }
    return item;
  });
}

function languageArray(value: unknown): CodeIndexLanguage[] {
  const values = stringArray(value);
  if (!values.every((item): item is CodeIndexLanguage => (
    codeIndexLanguages.some((language) => language === item)
  ))) return invalidResponse();
  if (new Set(values).size !== values.length) return invalidResponse();
  return values;
}

function normalizeRoot(root: string): string {
  const normalized = root.replaceAll("\\", "/").replace(/^\.\//, "").replace(/\/$/, "");
  if (normalized === "." || normalized === "") return "";
  if (/^(?:[a-zA-Z]:|\/)/.test(normalized) || normalized.split("/").some((part) => part === "..")) {
    return invalidConfiguration("selected roots must remain inside the workspace");
  }
  return normalized;
}

export function normalizeCodeIndexConfiguration(
  value: CodeIndexConfigurationInput,
): CodeIndexConfigurationInput {
  if (typeof value.enabled !== "boolean") invalidConfiguration("enabled must be boolean");
  if (!Array.isArray(value.selectedRoots)) invalidConfiguration("selected roots must be a list");
  const selectedRoots = [...new Set(value.selectedRoots.map((root) => {
    if (typeof root !== "string") return invalidConfiguration("selected roots must be strings");
    return normalizeRoot(root);
  }))].sort();
  if (!Array.isArray(value.languages) || !value.languages.every((language) => (
    codeIndexLanguages.some((supported) => supported === language)
  ))) invalidConfiguration("languages contain an unsupported value");
  const languages = [...new Set(value.languages)].sort() as CodeIndexLanguage[];
  if (value.enabled && languages.length === 0) invalidConfiguration("enabled indexes need a language");
  if (!Array.isArray(value.exclusionPatterns) || value.exclusionPatterns.length > 128) {
    invalidConfiguration("exclusion patterns exceed the supported limit");
  }
  const exclusionPatterns = value.exclusionPatterns.map((pattern) => {
    if (typeof pattern !== "string" || pattern.length === 0 || pattern.length > 256) {
      return invalidConfiguration("exclusion patterns must contain 1 to 256 characters");
    }
    return pattern;
  });
  if (!Number.isSafeInteger(value.maxFileBytes) || value.maxFileBytes < 1 || value.maxFileBytes > maximumFileBytes) {
    invalidConfiguration("maximum file size is outside the supported range");
  }
  return { enabled: value.enabled, selectedRoots: selectedRoots.length ? selectedRoots : [""], languages, exclusionPatterns, maxFileBytes: value.maxFileBytes };
}

export function normalizeCodeIndexStatus(value: unknown): CodeIndexStatus {
  if (!isRecord(value) || !isMember(codeIndexPhases, value.phase)) return invalidResponse();
  const result: CodeIndexStatus = {
    phase: value.phase, totalFiles: count(value.totalFiles), processedFiles: count(value.processedFiles),
    failedFiles: count(value.failedFiles), totalChunks: count(value.totalChunks),
    processedChunks: count(value.processedChunks), pendingChunks: count(value.pendingChunks),
    indexedChunks: count(value.indexedChunks), failedChunks: count(value.failedChunks),
    redactionCount: count(value.redactionCount),
    estimatedEmbeddingRequests: count(value.estimatedEmbeddingRequests),
    lastFailureCategory: optionalString(value.lastFailureCategory), updatedAt: timestamp(value.updatedAt),
  };
  if (result.processedFiles > result.totalFiles || result.failedFiles > result.processedFiles
    || result.processedChunks !== result.indexedChunks + result.failedChunks
    || result.totalChunks !== result.processedChunks + result.pendingChunks) return invalidResponse();
  return result;
}

export function normalizeCodeIndexWorkspace(value: unknown): CodeIndexWorkspace {
  if (!isRecord(value) || typeof value.enabled !== "boolean") return invalidResponse();
  const configuration = normalizeCodeIndexConfiguration({
    enabled: value.enabled,
    selectedRoots: stringArray(value.selectedRoots, true),
    languages: languageArray(value.languages),
    exclusionPatterns: stringArray(value.exclusionPatterns),
    maxFileBytes: count(value.maxFileBytes),
  });
  return {
    workspaceId: requiredString(value.workspaceId), canonicalRoot: requiredString(value.canonicalRoot),
    displayName: requiredString(value.displayName), ...configuration,
    indexVersion: requiredString(value.indexVersion), generation: count(value.generation),
    status: normalizeCodeIndexStatus(value.status),
  };
}

export function normalizeCodeIndexWorkspaces(value: unknown): CodeIndexWorkspace[] {
  if (!Array.isArray(value)) return invalidResponse();
  const workspaces = value.map(normalizeCodeIndexWorkspace);
  if (new Set(workspaces.map((workspace) => workspace.workspaceId)).size !== workspaces.length) {
    return invalidResponse();
  }
  return workspaces;
}

export function normalizeCodeEmbeddingConfirmation(value: unknown): CodeEmbeddingConfirmation {
  if (!isRecord(value)) return invalidResponse();
  return { profileId: requiredString(value.profileId), model: requiredString(value.model), generation: count(value.generation) };
}

export function normalizeCodeIndexAuditEntries(value: unknown): CodeIndexAuditEntry[] {
  if (!Array.isArray(value)) return invalidResponse();
  return value.map((entry) => {
    if (!isRecord(entry) || !isMember(auditEvents, entry.event)
      || (entry.reason !== null && !isMember(auditReasons, entry.reason))) return invalidResponse();
    return {
      auditId: count(entry.auditId), workspaceId: requiredString(entry.workspaceId),
      relativePath: optionalString(entry.relativePath), event: entry.event,
      reason: entry.reason, itemCount: count(entry.itemCount), createdAt: timestamp(entry.createdAt),
    };
  });
}
