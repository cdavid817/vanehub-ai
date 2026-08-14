import type {
  ModelInvocation,
  TokenUsageDetailsPage,
  TokenUsageDetailsQuery,
  TokenUsageSummary,
  TokenUsageSummaryQuery,
  UsageBreakdown,
  UsageBreakdownDimension,
  UsageEntityCounts,
  UsageMeasure,
  UsageObservation,
  UsageQualityTotals,
} from "../types/token-usage";

interface FixtureEntry {
  invocation: ModelInvocation;
  observation: UsageObservation;
}

const at = "2026-08-10T10:00:00.000Z";

function fixture(
  id: string,
  invocation: Partial<ModelInvocation>,
  observation: Partial<UsageObservation>,
): FixtureEntry {
  return {
    invocation: {
      id,
      generationId: null,
      runId: null,
      operationId: null,
      sessionId: "web-token-cli",
      messageId: null,
      agentId: "codex-cli",
      providerId: "openai",
      profileId: null,
      endpointId: null,
      modelId: "gpt-5-codex",
      interactionKind: "managed-cli",
      purpose: "assistant-initial",
      requestSequence: 1,
      attempt: 1,
      status: "succeeded",
      startedAt: at,
      completedAt: at,
      ...invocation,
    },
    observation: {
      id: `usage-${id}`,
      invocationId: id,
      quality: "reported",
      unit: "tokens",
      measurementKind: "interval",
      dimensions: {
        input: 100,
        output: 50,
        cachedInput: 30,
        cacheWriteInput: 0,
        reasoningOutput: 0,
        providerTotal: 150,
      },
      cacheOverlap: "subset",
      reasoningOverlap: "unknown",
      normalizationVersion: "web-fixture-v1",
      source: "web-mock",
      sourceRevision: "1",
      eventAt: at,
      observedAt: at,
      ...observation,
    },
  };
}

const fixtures: FixtureEntry[] = [
  fixture("web-inv-cli", { generationId: "web-gen-cli", messageId: "web-msg-cli" }, {}),
  fixture("web-inv-terminal", {
    interactionKind: "terminal-cli",
    purpose: "terminal-interval",
    requestSequence: 2,
  }, {
    quality: "reported-derived",
    dimensions: { input: 60, output: 15, cachedInput: 0, cacheWriteInput: 0, reasoningOutput: 0, providerTotal: 75 },
    cacheOverlap: "exclusive",
    reasoningOverlap: "exclusive",
  }),
  fixture("web-inv-onepiece-initial", {
    generationId: "web-gen-onepiece",
    messageId: "web-msg-onepiece",
    sessionId: "web-token-onepiece",
    agentId: "onepiece",
    providerId: "anthropic",
    profileId: "web-profile-primary",
    endpointId: "anthropic-messages",
    modelId: "claude-sonnet-4-5",
    interactionKind: "native-api",
  }, {
    dimensions: { input: 120, output: 80, cachedInput: 20, cacheWriteInput: 10, reasoningOutput: 0, providerTotal: 200 },
  }),
  fixture("web-inv-onepiece-tool", {
    generationId: "web-gen-onepiece",
    messageId: "web-msg-onepiece",
    sessionId: "web-token-onepiece",
    agentId: "onepiece",
    providerId: "openai-compatible",
    profileId: "web-profile-secondary",
    endpointId: "compatible-chat",
    modelId: "reasoning-model",
    interactionKind: "native-api",
    purpose: "tool-continuation",
    requestSequence: 2,
    status: "failed",
  }, {
    dimensions: { input: 50, output: 30, cachedInput: 0, cacheWriteInput: 0, reasoningOutput: 10, providerTotal: 90 },
  }),
  fixture("web-inv-compaction", {
    sessionId: "web-token-onepiece",
    agentId: "onepiece",
    providerId: "anthropic",
    modelId: "claude-haiku-4-5",
    interactionKind: "native-api",
    purpose: "context-compaction",
  }, {
    dimensions: { input: 30, output: 10, cachedInput: 0, cacheWriteInput: 0, reasoningOutput: 0, providerTotal: 40 },
  }),
  fixture("web-inv-estimated", {
    generationId: "web-gen-estimated",
    sessionId: "web-token-onepiece",
    agentId: "onepiece",
    providerId: null,
    modelId: null,
    interactionKind: "native-api",
    purpose: "retry",
  }, {
    quality: "estimated",
    unit: "characters",
    dimensions: { input: 600, output: 300, cachedInput: 0, cacheWriteInput: 0, reasoningOutput: 0, providerTotal: null },
    cacheOverlap: "exclusive",
    reasoningOverlap: "exclusive",
  }),
];

function emptyMeasure(unit: UsageMeasure["unit"]): UsageMeasure {
  return {
    unit,
    dimensions: { input: 0, output: 0, cachedInput: 0, cacheWriteInput: 0, reasoningOutput: 0, providerTotal: null },
    headlineTotal: 0,
    callCount: 0,
    observationCount: 0,
  };
}

function emptyQualityTotals(): UsageQualityTotals {
  return { reported: emptyMeasure("tokens"), reportedDerived: emptyMeasure("tokens"), estimated: emptyMeasure("characters") };
}

function observationTotal(observation: UsageObservation) {
  if (observation.dimensions.providerTotal !== null) return observation.dimensions.providerTotal;
  if (observation.cacheOverlap === "unknown" || observation.reasoningOverlap === "unknown") return null;
  return observation.dimensions.input + observation.dimensions.output
    + (observation.cacheOverlap === "exclusive" ? observation.dimensions.cachedInput + observation.dimensions.cacheWriteInput : 0)
    + (observation.reasoningOverlap === "exclusive" ? observation.dimensions.reasoningOutput : 0);
}

function aggregate(entries: FixtureEntry[]): UsageQualityTotals {
  const totals = emptyQualityTotals();
  for (const { observation } of entries) {
    const measure = observation.quality === "reported-derived" ? totals.reportedDerived : totals[observation.quality];
    for (const key of ["input", "output", "cachedInput", "cacheWriteInput", "reasoningOutput"] as const) {
      measure.dimensions[key] += observation.dimensions[key];
    }
    const total = observationTotal(observation);
    measure.headlineTotal = total === null || measure.headlineTotal === null ? null : measure.headlineTotal + total;
    measure.callCount += 1;
    measure.observationCount += 1;
  }
  return totals;
}

function counts(entries: FixtureEntry[]): UsageEntityCounts {
  return {
    calls: new Set(entries.map(({ invocation }) => invocation.id)).size,
    generations: new Set(entries.flatMap(({ invocation }) => invocation.generationId ? [invocation.generationId] : [])).size,
    sessions: new Set(entries.map(({ invocation }) => invocation.sessionId)).size,
  };
}

function matches(entry: FixtureEntry, query: TokenUsageSummaryQuery | TokenUsageDetailsQuery) {
  const { invocation, observation } = entry;
  return (!query.sessionId || invocation.sessionId === query.sessionId)
    && (!query.agentId || invocation.agentId === query.agentId)
    && (!query.providerId || invocation.providerId === query.providerId)
    && (!query.modelId || invocation.modelId === query.modelId)
    && (!query.purpose || invocation.purpose === query.purpose)
    && (!query.quality || observation.quality === query.quality)
    && (!query.status || invocation.status === query.status);
}

function summaryMatches(entry: FixtureEntry, query: TokenUsageSummaryQuery) {
  const startedAt = new Date(entry.invocation.startedAt).getTime();
  const rangeStart = query.rangeStart ? new Date(query.rangeStart).getTime() : null;
  const rangeEnd = query.rangeEnd ? new Date(query.rangeEnd).getTime() : null;
  return matches(entry, query)
    && (!query.messageId || entry.invocation.messageId === query.messageId)
    && (!query.generationId || entry.invocation.generationId === query.generationId)
    && (rangeStart === null || startedAt >= rangeStart)
    && (rangeEnd === null || startedAt < rangeEnd);
}

function localDate(value: string) {
  const date = new Date(value);
  const year = date.getFullYear();
  const month = String(date.getMonth() + 1).padStart(2, "0");
  const day = String(date.getDate()).padStart(2, "0");
  return `${year}-${month}-${day}`;
}

function breakdown(entries: FixtureEntry[], dimension: UsageBreakdownDimension): UsageBreakdown {
  const grouped = new Map<string, FixtureEntry[]>();
  for (const entry of entries) {
    const value = dimension === "quality" ? entry.observation.quality
      : dimension === "status" ? entry.invocation.status
      : dimension === "purpose" ? entry.invocation.purpose
      : dimension === "agent" ? entry.invocation.agentId
      : dimension === "provider" ? entry.invocation.providerId ?? "unknown"
      : entry.invocation.modelId ?? "unknown";
    grouped.set(value, [...(grouped.get(value) ?? []), entry]);
  }
  return {
    dimension,
    entries: [...grouped].map(([key, values]) => ({ key, totals: aggregate(values), counts: counts(values) })),
  };
}

export function queryWebTokenUsageSummary(query: TokenUsageSummaryQuery): TokenUsageSummary {
  const entries = fixtures.filter((entry) => summaryMatches(entry, query));
  const internalPurposes = new Set(["context-compaction", "memory-extraction"]);
  const dimensions: UsageBreakdownDimension[] = ["agent", "provider", "model", "purpose", "quality", "status"];
  return {
    schemaVersion: 1,
    totals: aggregate(entries),
    userResponse: aggregate(entries.filter(({ invocation }) => !internalPurposes.has(invocation.purpose))),
    internal: aggregate(entries.filter(({ invocation }) => internalPurposes.has(invocation.purpose))),
    counts: counts(entries),
    daily: entries.length ? [{ localDate: localDate(at), totals: aggregate(entries), counts: counts(entries) }] : [],
    breakdowns: dimensions.map((dimension) => ({
      ...breakdown(entries, dimension),
      entries: breakdown(entries, dimension).entries.slice(0, query.breakdownLimit ?? 10),
    })),
    generatedAt: "2026-08-10T10:05:00.000Z",
  };
}

export function queryWebTokenUsageDetails(query: TokenUsageDetailsQuery): TokenUsageDetailsPage {
  const afterIndex = query.afterId ? fixtures.findIndex(({ invocation }) => invocation.id === query.afterId) : -1;
  const matching = fixtures.filter((entry, index) => index > afterIndex && matches(entry, query));
  const page = matching.slice(0, Math.min(Math.max(query.limit ?? 25, 1), 100));
  return {
    schemaVersion: 1,
    invocations: page.map(({ invocation }) => structuredClone(invocation)),
    observations: page.map(({ observation }) => structuredClone(observation)),
    nextCursor: matching.length > page.length ? page.at(-1)?.invocation.id ?? null : null,
  };
}
