import {
  completeRunningTimelineFixture,
  resetExecutionTimelineFixtures,
  WEB_EVENT_PAGE_TWO,
} from "./web-execution-observability-fixtures";
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

let timelines: ExecutionTimeline[] = resetExecutionTimelineFixtures();

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
    // Copied, not hardcoded. Writing `truncated: false` here would silently override any fixture
    // that reports a clipped list, making the truncation path unreachable from the browser build
    // no matter what the fixtures say.
    eventCoverage: { ...timeline.eventCoverage },
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

  async getTimeline(runId, eventPageToken) {
    const timeline = timelines.find((item) => item.run.runId === runId);
    if (!timeline) throw new Error(`execution run not found: ${runId}`);
    const page = cloneTimeline(timeline);
    if (!eventPageToken) return page;
    if (eventPageToken === WEB_EVENT_PAGE_TWO) {
      // The tail of the one fixture that reports a clipped list. Complete coverage this time, so a
      // reader who follows the continuation reaches the end and the notice stops claiming loss.
      return {
        ...page,
        events: [
          {
            sequence: 2,
            spanId: "00f067aa0ba902b7",
            name: "process.exited",
            timestamp: "2026-07-23T08:00:01.200Z",
            attributes: { "process.exit.code": 0 },
          },
        ],
        eventCoverage: { truncated: false, nextPageToken: null },
      };
    }
    // A token this adapter never issued. Answer with the empty tail rather than page one, which
    // would hand back the same events and let a paging loop run forever. Ignoring the argument
    // entirely is worse still: TypeScript accepts a narrower implementation, so the divergence
    // would never surface at a call site.
    return { ...page, events: [], eventCoverage: { truncated: false, nextPageToken: null } };
  },

  async getObservationCapabilities() {
    return capabilities.map((capability) => ({ ...capability }));
  },
};

export function resetWebExecutionObservabilityForTest() {
  settings = { ...defaultSettings };
  timelines = resetExecutionTimelineFixtures();
}

/**
 * Advances the running fixture to a terminal state.
 *
 * The browser build has to be able to cross this boundary, because the rules that only hold on
 * one side of it — no duration while running, no critical path until everything has ended — are
 * unobservable from a fixture that is permanently on one side.
 */
export function completeRunningExecutionForTest() {
  completeRunningTimelineFixture(timelines);
}
