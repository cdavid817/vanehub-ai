import { agentService } from "../services/runtime-agent-client";
import type { Session, SessionLifecycleState } from "../types/agent";
import type { SemanticStatus, WorkbenchSearchPage, WorkbenchSearchProvider, WorkbenchSearchRequest } from "./command-center-types";

/**
 * `starting`/`running` are the only states with something actively in progress; `failed` is the
 * only unambiguous error; everything else (`idle`, `stopped`) is a resting state with nothing for
 * a reader to react to right now.
 */
const LIFECYCLE_STATUS: Record<SessionLifecycleState, SemanticStatus> = {
  idle: "neutral",
  starting: "active",
  running: "active",
  failed: "error",
  stopped: "neutral",
};

function toSearchResult(session: Session) {
  return {
    key: session.id,
    kind: "session" as const,
    title: session.title,
    subtitle: session.projectPath ?? undefined,
    status: LIFECYCLE_STATUS[session.lifecycleState],
    route: { destination: "sessions" as const, sessionId: session.id, creatingSession: false },
    updatedAt: session.updatedAt,
  };
}

/**
 * design.md Decision 4 privacy rule: results must never carry response/message content.
 * `SessionSearchMatch.excerpt` is a snippet of actual chat text for `kind: "message"` matches —
 * deliberately never read here, for any match kind, so the rule holds uniformly rather than by
 * case-by-case filtering that the next new match kind could quietly bypass.
 */
export const sessionSearchProvider: WorkbenchSearchProvider = {
  id: "sessions",
  supports: (scope) => scope === "session",
  async search(request: WorkbenchSearchRequest): Promise<WorkbenchSearchPage> {
    const results = await agentService.searchSessions({ query: request.query, limit: request.limit });
    return { items: results.slice(0, request.limit).map((result) => toSearchResult(result.session)), nextCursor: null };
  },
};
