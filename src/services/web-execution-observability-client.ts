import type {
  ExecutionObservationCapability,
  ExecutionTimeline,
  ObservabilitySettings,
} from "../types/execution-observability";
import type { ExecutionObservabilityService } from "./execution-observability-service";

const defaultSettings: ObservabilitySettings = {
  localTimelineEnabled: true,
  otlpEnabled: false,
  otlpEndpoint: null,
  otlpProtocol: "http_protobuf",
  samplingRatio: 1,
  retentionDays: 30,
  capturePolicy: "metadata_only",
  mcpRelayEnabled: false,
  otlpAuthConfigured: false,
};

let settings = { ...defaultSettings };

const timelines: ExecutionTimeline[] = [
  {
    run: {
      runId: "018f0f17-4d6a-7e20-b41d-66c5271a28d0",
      traceId: "4bf92f3577b34da6a3ce929d0e0e4736",
      rootSpanId: "00f067aa0ba902b7",
      source: "desktop",
      sourceId: null,
      status: "succeeded",
      startedAt: "2026-07-23T08:00:00.000Z",
      endedAt: "2026-07-23T08:00:02.400Z",
      durationMs: 2400,
      sessionId: "web-session-1",
      operationId: "web-operation-observability",
      agentId: "codex-cli",
    },
    spans: [
      {
        spanId: "00f067aa0ba902b7",
        parentSpanId: null,
        name: "vanehub.task.execute",
        status: "succeeded",
        fidelity: "native",
        startedAt: "2026-07-23T08:00:00.000Z",
        endedAt: "2026-07-23T08:00:02.400Z",
        durationMs: 2400,
        errorClassification: null,
        attributes: { "gen_ai.provider.name": "openai" },
      },
      {
        spanId: "b7ad6b7169203331",
        parentSpanId: "00f067aa0ba902b7",
        name: "execute_tool search",
        status: "incomplete",
        fidelity: "inferred",
        startedAt: "2026-07-23T08:00:01.000Z",
        endedAt: "2026-07-23T08:00:02.300Z",
        durationMs: null,
        errorClassification: "missing_terminal_boundary",
        attributes: { "gen_ai.tool.name": "search" },
      },
      {
        spanId: "b7ad6b7169203333",
        parentSpanId: "00f067aa0ba902b7",
        name: "mcp.client request",
        status: "incomplete",
        fidelity: "opaque",
        startedAt: "2026-07-23T08:00:01.100Z",
        endedAt: null,
        durationMs: null,
        errorClassification: "traffic_not_managed",
        attributes: { "rpc.system": "mcp" },
      },
    ],
    events: [
      {
        sequence: 1,
        spanId: "00f067aa0ba902b7",
        name: "process.spawned",
        timestamp: "2026-07-23T08:00:00.500Z",
        attributes: { "process.pid.observed": true },
      },
    ],
  },
  {
    run: {
      runId: "018f0f17-4d6a-7e20-b41d-66c5271a28d1",
      traceId: "0af7651916cd43dd8448eb211c80319c",
      rootSpanId: "b7ad6b7169203332",
      source: "scheduled",
      sourceId: "web-schedule-1",
      status: "failed",
      startedAt: "2026-07-22T08:00:00.000Z",
      endedAt: "2026-07-22T08:00:00.100Z",
      durationMs: 100,
      sessionId: "web-session-1",
      operationId: "web-operation-scheduled",
      agentId: "gemini-cli",
    },
    spans: [],
    events: [],
  },
];

for (let index = 2; index < 21; index += 1) {
  const suffix = index.toString(16).padStart(12, "0");
  timelines.push({
    run: {
      ...timelines[1]!.run,
      runId: `018f0f17-4d6a-7e20-b41d-${suffix}`,
      startedAt: `2026-07-${String(22 - (index % 10)).padStart(2, "0")}T08:00:00.000Z`,
      operationId: `web-operation-${index}`,
    },
    spans: [],
    events: [],
  });
}

const capabilities: ExecutionObservationCapability[] = [
  "claude-code",
  "codex-cli",
  "gemini-cli",
  "opencode",
].flatMap((agentId) =>
  (["stdio", "http"] as const).map((transport) => ({
    agentId,
    transport,
    toolFidelity: "inferred" as const,
    mcpFidelity: "opaque" as const,
    relaySupported: false,
    detail: "Web preview does not execute native Agent or MCP traffic",
  })),
);

function cloneTimeline(timeline: ExecutionTimeline): ExecutionTimeline {
  return {
    run: { ...timeline.run },
    spans: timeline.spans.map((span) => ({ ...span, attributes: { ...span.attributes } })),
    events: timeline.events.map((event) => ({ ...event, attributes: { ...event.attributes } })),
  };
}

function validateSettings(input: ObservabilitySettings) {
  if (!Number.isFinite(input.samplingRatio) || input.samplingRatio < 0 || input.samplingRatio > 1) {
    throw new Error("samplingRatio must be between 0 and 1");
  }
  if (!Number.isInteger(input.retentionDays) || input.retentionDays < 1 || input.retentionDays > 90) {
    throw new Error("retentionDays must be between 1 and 90");
  }
  if (input.otlpEnabled || input.mcpRelayEnabled || input.otlpAuthConfigured) {
    throw new Error("Native OTLP export, credentials, and MCP relay are unavailable in Web preview");
  }
}

function pageOffset(pageToken?: string | null) {
  if (!pageToken) return 0;
  const match = /^web:(\d+)$/.exec(pageToken);
  if (!match) throw new Error("invalid Web observability page token");
  return Number(match[1]);
}

export const webExecutionObservabilityClient: ExecutionObservabilityService = {
  async getSettings() {
    return { ...settings };
  },

  async updateSettings(input) {
    validateSettings(input);
    settings = { ...input, otlpEndpoint: input.otlpEndpoint ?? null, otlpAuthToken: null };
    return { ...settings };
  },

  async listRuns(query) {
    if (!Number.isInteger(query.limit) || query.limit < 1 || query.limit > 100) {
      throw new Error("limit must be between 1 and 100");
    }
    const offset = pageOffset(query.pageToken);
    const runs = timelines
      .map((timeline) => timeline.run)
      .filter((run) => !query.sessionId || run.sessionId === query.sessionId);
    const items = runs.slice(offset, offset + query.limit).map((run) => ({ ...run }));
    const nextOffset = offset + items.length;
    return {
      items,
      nextPageToken: nextOffset < runs.length ? `web:${nextOffset}` : null,
    };
  },

  async getRun(runId) {
    const run = timelines.find((timeline) => timeline.run.runId === runId)?.run;
    if (!run) throw new Error(`execution run not found: ${runId}`);
    return { ...run };
  },

  async getTimeline(runId) {
    const timeline = timelines.find((item) => item.run.runId === runId);
    if (!timeline) throw new Error(`execution run not found: ${runId}`);
    return cloneTimeline(timeline);
  },

  async getObservationCapabilities() {
    return capabilities.map((capability) => ({ ...capability }));
  },
};

export function resetWebExecutionObservabilityForTest() {
  settings = { ...defaultSettings };
}
