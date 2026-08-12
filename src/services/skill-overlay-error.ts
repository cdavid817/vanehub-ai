import type {
  SkillOverlayErrorKind,
  SkillOverlayServiceError,
} from "../types/skill-overlay-reconciliation";

const errorKinds: readonly SkillOverlayErrorKind[] = [
  "validation",
  "notFound",
  "conflict",
  "stale",
  "pinned",
  "limit",
  "trust",
  "import",
  "integrity",
  "infrastructure",
];

function isNullableNumber(value: unknown): value is number | null {
  return value === null || typeof value === "number";
}

function isNullableBoolean(value: unknown): value is boolean | null {
  return value === null || typeof value === "boolean";
}

export function isSkillOverlayServiceError(error: unknown): error is SkillOverlayServiceError {
  if (typeof error !== "object" || error === null) return false;
  const value = error as Record<string, unknown>;
  return typeof value.kind === "string"
    && errorKinds.includes(value.kind as SkillOverlayErrorKind)
    && typeof value.code === "string"
    && typeof value.message === "string"
    && isNullableNumber(value.expectedRevision)
    && isNullableNumber(value.currentRevision)
    && isNullableNumber(value.maximum)
    && isNullableNumber(value.actual)
    && isNullableBoolean(value.baseChanged)
    && isNullableBoolean(value.payloadChanged)
    && isNullableBoolean(value.pinChanged);
}

export function normalizeSkillOverlayError(error: unknown): SkillOverlayServiceError {
  if (isSkillOverlayServiceError(error)) return error;
  return {
    kind: "infrastructure",
    code: "native-overlay-failure",
    message: error instanceof Error ? error.message : String(error),
    expectedRevision: null,
    currentRevision: null,
    maximum: null,
    actual: null,
    baseChanged: null,
    payloadChanged: null,
    pinChanged: null,
  };
}
