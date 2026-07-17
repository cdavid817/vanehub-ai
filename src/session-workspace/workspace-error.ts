import { normalizeServiceError, type ServiceErrorCode } from "../services/service-error";

export type WorkspaceErrorKey =
  | "sessionTabs.error.validation"
  | "sessionTabs.error.notFound"
  | "sessionTabs.error.unsupported"
  | "sessionTabs.error.runtime";

const errorKeys: Record<ServiceErrorCode, WorkspaceErrorKey> = {
  validation: "sessionTabs.error.validation",
  "not-found": "sessionTabs.error.notFound",
  "unsupported-runtime": "sessionTabs.error.unsupported",
  runtime: "sessionTabs.error.runtime",
  unknown: "sessionTabs.error.runtime",
};

export function workspaceErrorKey(error: unknown): WorkspaceErrorKey {
  return errorKeys[normalizeServiceError(error).code];
}
