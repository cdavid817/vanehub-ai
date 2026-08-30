import { agentService } from "../services/runtime-agent-client";
import type { MissionControlRunSummary } from "../types/mission-control";
import type { SemanticStatus, WorkbenchSearchProvider, WorkbenchSearchRequest, WorkbenchSearchResult } from "./command-center-types";

/**
 * `run.attention` (already computed server/mock-side, `web-mission-control-client.ts`) is the
 * authoritative "needs a human" signal Mission Control's own UI already trusts — re-deriving it
 * from `state` here would risk drifting from that single source of truth. `"failed"` maps to
 * `"error"` specifically, not `"attention"`: it is the one attention reason that is not "please
 * look at this," it already happened.
 */
function toStatus(run: MissionControlRunSummary): SemanticStatus {
  if (run.attention === "failed") return "error";
  if (run.attention !== null) return "attention";
  if (run.state === "completed") return "success";
  if (run.state === "running" || run.state === "verifying" || run.state === "retrying" || run.state === "preparing") return "active";
  return "neutral";
}

function toSearchResult(run: MissionControlRunSummary): WorkbenchSearchResult {
  return {
    key: run.runId,
    kind: "run",
    title: run.title,
    status: toStatus(run),
    // "attention" is a fixed, deliberate choice, not the run's actual current section: Runs'
    // attention/active/history tabs all render the same MissionControl instance with the same
    // `initialRunId` restore behavior (this session's 4.8 work), so any of the three would resolve
    // to the same run — "attention" just reads better as a search-result destination than "active".
    route: { destination: "runs", section: "attention", runId: run.runId },
    updatedAt: run.updatedAt,
  };
}

/**
 * design.md Decision 4 privacy rule: no raw errors. `reasonCode` is a bounded vocabulary, not a raw
 * error string (`RunCard` in mission-control.tsx treats it as an i18n key with the raw value only
 * as a fallback) — still deliberately excluded here anyway, to keep this provider's safety
 * trivially auditable by field name alone rather than requiring a judgment call about which fields
 * are "safe enough."
 */
export const runSearchProvider: WorkbenchSearchProvider = {
  id: "runs",
  supports: (scope) => scope === "run",
  // request.signal is intentionally unused: getMissionControlOverview isn't abortable, and a
  // shared orchestrator discards stale results centrally rather than each provider doing it itself.
  async search(request: WorkbenchSearchRequest) {
    // `getMissionControlOverview` has no free-text query — attention/active/recent are each
    // already-paginated, so a generous page size per section is fetched and filtered client-side,
    // not a real cross-section query.
    const overview = await agentService.getMissionControlOverview({ limit: 50, sort: "newest" });
    const seen = new Set<string>();
    const candidates: MissionControlRunSummary[] = [];
    for (const run of [...overview.attention.items, ...overview.active.items, ...overview.recent.items]) {
      if (seen.has(run.runId)) continue;
      seen.add(run.runId);
      candidates.push(run);
    }
    const needle = request.query.trim().toLowerCase();
    const matched = needle ? candidates.filter((run) => run.title.toLowerCase().includes(needle)) : candidates;
    return { items: matched.slice(0, request.limit).map(toSearchResult), nextCursor: null };
  },
};
