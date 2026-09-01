import type { ExtensionFrameworkStatus } from "../../../types/extension";

/** Shared by the page's own search filter and the per-card status badge so both translate the
 *  same framework lifecycle status the same way. */
export function statusKey(status: ExtensionFrameworkStatus["status"]) {
  return `extensions.status.${status}`;
}
