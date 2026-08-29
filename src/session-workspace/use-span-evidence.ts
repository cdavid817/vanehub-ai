import { useQuery } from "@tanstack/react-query";
import { useMemo } from "react";
import {
  evidenceRunIdSchema,
  evidenceSessionIdSchema,
  evidenceSpanIdSchema,
  evidenceTraceIdSchema,
} from "../contracts/session-workspace-evidence-ids";
import { agentService as defaultAgentService } from "../services/runtime-agent-client";
import type { AgentService } from "../services/agent-service";
import type { ExecutionRecord } from "../types/session-workspace-evidence";
import type { SessionLogEntry } from "../types/session-workspace";

/**
 * The evidence a span points at, fetched from whoever owns it.
 *
 * Not embedded in the trace DTO, and the reason is not tidiness. Log text and command output are
 * exactly the material redaction exists for, and a trace payload is one of the places redaction
 * has no second chance to run — so the trace carries identifiers and the services that already
 * answer for those records answer for them here too.
 *
 * The scope is the span's own correlation, which is what makes this a lookup rather than a guess:
 * the log index has filtered on `traceId` and `spanId` since it existed, and the evidence journal
 * records the same pair. Neither is being asked to match on anything derived.
 */

/** How many linked records one section will show before it stops. */
export const SPAN_EVIDENCE_LIMIT = 20;

export interface SpanEvidence {
  logs: SessionLogEntry[];
  commands: ExecutionRecord[];
  findings: ExecutionRecord[];
  /** True while either query is in flight. The drawer shows what it has and says the rest is coming. */
  loading: boolean;
  /**
   * Set when a query failed.
   *
   * Distinct from an empty result, and the distinction is the whole point: an empty section means
   * the span linked to nothing, and a failed one means nobody knows. Rendering them the same way
   * turns "we could not look" into "there is nothing there".
   */
  failed: boolean;
}

export function useSpanEvidence({
  enabled,
  runId,
  sessionId,
  service = defaultAgentService,
  spanId,
  traceId,
}: {
  enabled: boolean;
  runId: string;
  sessionId: string | null;
  /** Injected so the drawer can be driven without a runtime, as every other panel here is. */
  service?: AgentService;
  spanId: string;
  traceId: string;
}): SpanEvidence {
  const active = enabled && Boolean(sessionId) && Boolean(spanId);

  const logs = useQuery({
    queryKey: ["span-logs", sessionId, traceId, spanId],
    queryFn: () => service.listSessionLogs({
      sessionId: sessionId ?? "",
      levels: [],
      search: "",
      traceId,
      spanId,
      limit: SPAN_EVIDENCE_LIMIT,
    }),
    enabled: active,
  });

  // Parsed rather than cast. These are branded ids, and the brand exists so a value that is not
  // one cannot reach a query that assumes it is — a cast would put the check nowhere.
  const scope = useMemo(() => {
    const session = evidenceSessionIdSchema.safeParse(sessionId);
    if (!session.success) return null;
    const run = evidenceRunIdSchema.safeParse(runId);
    const trace = evidenceTraceIdSchema.safeParse(traceId);
    const span = evidenceSpanIdSchema.safeParse(spanId);
    return {
      sessionId: session.data,
      // A correlation that failed to parse is dropped rather than passed through: it would
      // narrow the query to something no record can match, and the empty result would read as
      // "this span touched nothing".
      runId: run.success ? run.data : undefined,
      traceId: trace.success ? trace.data : undefined,
      spanId: span.success ? span.data : undefined,
    };
  }, [runId, sessionId, spanId, traceId]);

  const records = useQuery({
    queryKey: ["span-evidence", sessionId, runId, traceId, spanId],
    queryFn: () => service.listExecutionRecords({
      scope: scope ?? { sessionId: evidenceSessionIdSchema.parse(sessionId ?? "") },
      limit: SPAN_EVIDENCE_LIMIT,
    }),
    enabled: active && scope !== null,
  });

  const items: ExecutionRecord[] = records.data?.items ?? [];
  return {
    logs: logs.data?.items ?? [],
    commands: items.filter((record) => record.kind === "command"),
    // Verifications are what a reader means by a finding: something the run checked and reported
    // an outcome for. A tool call is work, not a finding, so it is not folded in here.
    findings: items.filter((record) => record.kind === "verification"),
    loading: logs.isLoading || records.isLoading,
    failed: logs.isError || records.isError,
  };
}
