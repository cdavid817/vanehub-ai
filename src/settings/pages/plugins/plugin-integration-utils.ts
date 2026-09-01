import type { PluginIntegrationStatus } from "../../../types/plugin-integration";

export function statusKey(status: PluginIntegrationStatus) {
  return `plugins.status.${status}`;
}

export function errorMessage(error: unknown) {
  return error instanceof Error ? error.message : String(error);
}
