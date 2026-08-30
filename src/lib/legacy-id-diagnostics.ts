/**
 * Deprecation policy for redesign-unified-workbench-ui (task 1.7):
 *
 * - Legacy destination ids (sessions/loops/work-board/goals/evaluations/mission-control) and
 *   legacy session tab ids (chat/changes/documents/files/terminal/shell/logs/traces/report) keep
 *   working through a compatibility adapter for one full stable release cycle after the new
 *   route/surface registries ship (task groups 4 and 8), matching design.md's route-compatibility
 *   commitment: "旧 route adapter 至少保留一个稳定版本周期."
 * - Removal is a decision for the migration flag's own cleanup milestone (task 22.1), not
 *   something a later, unrelated task may do quietly — see implementation-notes.md.
 * - A legacy id that has no entry in the compatibility adapter's mapping table is not silently
 *   dropped: `warnUnmappedLegacyId` records it during development so a missing mapping is caught
 *   before release rather than surfacing as a blank surface a user can't explain.
 */

const seenUnmappedIds = new Set<string>();
const MAX_LOGGED_ID_LENGTH = 64;

/**
 * A legacy id is a small fixed vocabulary the app itself defines (route segments, tab names) —
 * never user-entered text — so it is safe to reach a developer console. The length clamp is a
 * defense against a caller passing something else by mistake, not an expectation this ever fires.
 */
function safeIdForLogging(id: string): string {
  return id.length > MAX_LOGGED_ID_LENGTH ? `${id.slice(0, MAX_LOGGED_ID_LENGTH)}…` : id;
}

/**
 * Call when a legacy compatibility adapter (destination or session-tab id mapping) receives a
 * value it has no mapping for. Development-only and deduped per (category, id) so a render loop
 * hitting the same unmapped value repeatedly does not spam the console.
 */
export function warnUnmappedLegacyId(category: string, id: string): void {
  if (!import.meta.env.DEV) return;
  const dedupeKey = `${category}:${id}`;
  if (seenUnmappedIds.has(dedupeKey)) return;
  seenUnmappedIds.add(dedupeKey);
  console.warn(`[legacy-id] no ${category} mapping for "${safeIdForLogging(id)}" — falling back to default.`);
}

/** Test-only: clears dedupe state so each test starts with a clean slate. */
export function resetLegacyIdDiagnosticsForTests(): void {
  seenUnmappedIds.clear();
}
