import type { JsonObject, JsonValue } from "../types/lsp";

/**
 * Value coercion for a runtime response that has to fail closed.
 *
 * Every helper here refuses rather than repairs: a malformed field throws instead of becoming a
 * default, because a default is a value the backend never sent and the UI would then present as
 * fact. Split out of `lsp-contract.ts` so the shape normalizers there read as shapes.
 */

const maximumJsonDepth = 32;
const maximumJsonItems = 1024;
const maximumListItems = 1024;
const rfc3339Pattern = /^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(?:\.\d+)?(?:Z|[+-]\d{2}:\d{2})$/;

export function invalidResponse(): never {
  throw new Error("The runtime returned an invalid LSP response.");
}

export function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

export function isMember<T extends string>(values: readonly T[], value: unknown): value is T {
  return typeof value === "string" && values.some((candidate) => candidate === value);
}

export function member<T extends string>(values: readonly T[], value: unknown): T {
  return isMember(values, value) ? value : invalidResponse();
}

export function requiredString(value: unknown, maximumLength = 4096): string {
  if (typeof value !== "string" || value.trim() === "" || value.length > maximumLength) {
    return invalidResponse();
  }
  return value;
}

export function optionalString(value: unknown): string | null {
  return value === null ? null : requiredString(value);
}

export function booleanValue(value: unknown): boolean {
  return typeof value === "boolean" ? value : invalidResponse();
}

export function count(value: unknown): number {
  if (typeof value !== "number" || !Number.isSafeInteger(value) || value < 0) {
    return invalidResponse();
  }
  return value;
}

export function arrayValue(value: unknown, maximum = maximumListItems): readonly unknown[] {
  if (!Array.isArray(value) || value.length > maximum) return invalidResponse();
  return value;
}

export function optionalTimestamp(value: unknown): string | null {
  if (value === null) return null;
  const timestamp = requiredString(value, 64);
  if (!rfc3339Pattern.test(timestamp) || Number.isNaN(Date.parse(timestamp))) {
    return invalidResponse();
  }
  return timestamp;
}

export function normalizeJsonValue(value: unknown, depth: number): JsonValue {
  if (depth > maximumJsonDepth) return invalidResponse();
  if (value === null || typeof value === "string" || typeof value === "boolean") return value;
  if (typeof value === "number") return Number.isFinite(value) ? value : invalidResponse();
  if (Array.isArray(value)) {
    if (value.length > maximumJsonItems) return invalidResponse();
    return value.map((item) => normalizeJsonValue(item, depth + 1));
  }
  return normalizeJsonObject(value, depth + 1);
}

export function normalizeJsonObject(value: unknown, depth = 0): JsonObject {
  if (!isRecord(value) || depth > maximumJsonDepth) return invalidResponse();
  const entries = Object.entries(value);
  if (entries.length > maximumJsonItems) return invalidResponse();
  const normalized: [string, JsonValue][] = entries.map(([key, item]) => [
    requiredString(key, 256), normalizeJsonValue(item, depth + 1),
  ]);
  return Object.fromEntries<JsonValue>(normalized);
}

export function optionalStringArray(value: unknown, maximum = 32): string[] | null {
  return value === null ? null : arrayValue(value, maximum).map((item) => requiredString(item));
}

export function stringArray(value: unknown, maximum = 16): string[] {
  return arrayValue(value, maximum).map((item) => requiredString(item, 1024));
}

export function unique(ids: readonly string[]): boolean {
  return new Set(ids).size === ids.length;
}
