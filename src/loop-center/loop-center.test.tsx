// @vitest-environment jsdom

import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { fireEvent, render, screen } from "@testing-library/react";
import { beforeAll, beforeEach, describe, expect, it, vi } from "vitest";
import { activateAppLanguage } from "../i18n";
import { loopQueryKeys } from "../hooks/loop-query";
import { loopDefinitionFixture, loopRunFixture } from "../test/loop-fixtures";
import { LoopCenter, type LoopCenterProps } from "./loop-center";

const mocks = vi.hoisted(() => ({
  listLoopDefinitions: vi.fn(),
  listLoopRuns: vi.fn(),
  getLoopRun: vi.fn(),
  subscribeLoopEvents: vi.fn(),
}));

vi.mock("../services/runtime-agent-client", () => ({
  agentService: {
    listLoopDefinitions: mocks.listLoopDefinitions,
    listLoopRuns: mocks.listLoopRuns,
    getLoopRun: mocks.getLoopRun,
    subscribeLoopEvents: mocks.subscribeLoopEvents,
  },
}));

// 17.2's own selection wiring is what this file tests -- LoopDefinitionOverview and LoopTimeline
// are stubbed to bare divs exposing just the id they were handed, the same isolation
// runs-destination.test.tsx uses for LazyFeature, so these tests never depend on either
// component's own service calls (LoopDefinitionOverview's agent-registry lookup in particular) or
// rendered detail -- only on which one LoopCenter chose to show, and with which id.
vi.mock("./loop-definition-overview", () => ({
  LoopDefinitionOverview: ({ definition }: { definition: { id: string } }) => (
    <div data-definition-overview-id={definition.id} data-testid="loop-definition-overview" />
  ),
}));
vi.mock("./loop-timeline", () => ({
  LoopTimeline: ({ run }: { run: { id: string } }) => (
    <div data-run-id={run.id} data-testid="loop-timeline" />
  ),
}));
// 17.3 Piece B: the inspector column is now the shared Inspector shell, whose "loop-iteration"
// detail is the lazily-loaded LoopIterationInspectorProvider (registered in
// inspector-provider-registry.ts) -- it renders LoopInspectorBody, which renders
// AgentRunOwnerStatus, which calls agentService.listAgentRuns, a service method unrelated to
// 17.2's own selection wiring and out of scope for this file's mock. Stubbing the provider itself
// (rather than loop-inspector.tsx, which loop-center.tsx no longer imports at all) keeps these
// tests isolated to LoopCenter's own definition/run selection instead of the inspector panel's
// content, the same isolation the LoopDefinitionOverview/LoopTimeline mocks above already use.
vi.mock("./loop-iteration-inspector-provider", () => ({
  LoopIterationInspectorProvider: () => <div data-testid="loop-iteration-inspector-provider-stub" />,
}));

const definitionA = loopDefinitionFixture({ id: "definition-a", name: "Definition A" });
const definitionB = loopDefinitionFixture({ id: "definition-b", name: "Definition B" });
const runA1 = loopRunFixture("running", { id: "run-a1", definitionId: "definition-a" });

function buildQueryClient() {
  const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  client.setQueryData(loopQueryKeys.definitions, [definitionA, definitionB]);
  client.setQueryData(loopQueryKeys.runs("definition-a"), [runA1]);
  client.setQueryData(loopQueryKeys.runs("definition-b"), []);
  client.setQueryData(loopQueryKeys.run("run-a1"), runA1);
  return client;
}

function renderLoopCenter(props: Partial<LoopCenterProps> = {}) {
  return render(
    <QueryClientProvider client={buildQueryClient()}>
      <LoopCenter {...props} />
    </QueryClientProvider>,
  );
}

describe("LoopCenter route-driven selection (17.2)", () => {
  beforeAll(async () => activateAppLanguage("en"));

  beforeEach(() => {
    mocks.listLoopDefinitions.mockReset().mockResolvedValue([definitionA, definitionB]);
    mocks.listLoopRuns.mockReset().mockImplementation((definitionId?: string) =>
      Promise.resolve(definitionId === "definition-a" ? [runA1] : []));
    mocks.getLoopRun.mockReset().mockResolvedValue(runA1);
    mocks.subscribeLoopEvents.mockReset().mockResolvedValue(() => {});
  });

  it("uses the definitionId/loopRunId props as the initial selection, matching the route's own current run", async () => {
    renderLoopCenter({ definitionId: "definition-a", loopRunId: "run-a1" });
    expect((await screen.findByTestId("loop-timeline")).dataset.runId).toBe("run-a1");
    expect(screen.queryByTestId("loop-definition-overview")).toBeNull();
  });

  it("uses the definitionId prop alone to pre-select the definition overview when no run is routed", async () => {
    renderLoopCenter({ definitionId: "definition-b" });
    expect((await screen.findByTestId("loop-definition-overview")).dataset.definitionOverviewId).toBe("definition-b");
  });

  it("reports a definition selection through onSelectionChange", async () => {
    const onSelectionChange = vi.fn();
    renderLoopCenter({ onSelectionChange });
    // No definitionId prop here -- waits for the pre-existing auto-select-first-definition effect
    // (unaffected by 17.2) to settle on definition-a before the click below.
    await screen.findByTestId("loop-definition-overview");

    fireEvent.click(screen.getByRole("button", { name: /Definition B/ }));

    expect(onSelectionChange).toHaveBeenCalledTimes(1);
    expect(onSelectionChange).toHaveBeenCalledWith({ definitionId: "definition-b", loopRunId: undefined });
  });

  it("reports a run selection through onSelectionChange, paired with its owning definitionId", async () => {
    const onSelectionChange = vi.fn();
    renderLoopCenter({ definitionId: "definition-a", onSelectionChange });
    await screen.findByTestId("loop-definition-overview");

    fireEvent.click(screen.getByRole("button", { name: /Running/i }));

    expect(onSelectionChange).toHaveBeenCalledTimes(1);
    expect(onSelectionChange).toHaveBeenCalledWith({ definitionId: "definition-a", loopRunId: "run-a1" });
  });

  it("selecting a different definition clears the previously-selected run, locally and in what's reported back", async () => {
    const onSelectionChange = vi.fn();
    renderLoopCenter({ definitionId: "definition-a", loopRunId: "run-a1", onSelectionChange });
    expect(await screen.findByTestId("loop-timeline")).toBeTruthy();

    fireEvent.click(screen.getByRole("button", { name: /Definition B/ }));

    // Reported back as one combined call, never as a run-still-selected intermediate state.
    expect(onSelectionChange).toHaveBeenCalledWith({ definitionId: "definition-b", loopRunId: undefined });
    expect(screen.queryByTestId("loop-timeline")).toBeNull();
    expect((await screen.findByTestId("loop-definition-overview")).dataset.definitionOverviewId).toBe("definition-b");
  });

  it("re-syncs the selection when the route's own ids change while this stays mounted", async () => {
    const client = buildQueryClient();
    const { rerender } = render(
      <QueryClientProvider client={client}>
        <LoopCenter definitionId="definition-a" loopRunId="run-a1" />
      </QueryClientProvider>,
    );
    expect((await screen.findByTestId("loop-timeline")).dataset.runId).toBe("run-a1");

    // Simulates RunsDestination keeping Loops mounted across a route change (5.13) rather than
    // remounting it -- a definition-only route drops the previous run selection along with it.
    rerender(
      <QueryClientProvider client={client}>
        <LoopCenter definitionId="definition-b" />
      </QueryClientProvider>,
    );

    expect((await screen.findByTestId("loop-definition-overview")).dataset.definitionOverviewId).toBe("definition-b");
    expect(screen.queryByTestId("loop-timeline")).toBeNull();
  });
});

describe("LoopCenter inspector selection (17.3 Piece B)", () => {
  beforeAll(async () => activateAppLanguage("en"));

  beforeEach(() => {
    mocks.listLoopDefinitions.mockReset().mockResolvedValue([definitionA, definitionB]);
    mocks.listLoopRuns.mockReset().mockImplementation((definitionId?: string) =>
      Promise.resolve(definitionId === "definition-a" ? [runA1] : []));
    mocks.getLoopRun.mockReset().mockResolvedValue(runA1);
    mocks.subscribeLoopEvents.mockReset().mockResolvedValue(() => {});
  });

  it("drives the shared Inspector's loop-iteration provider once a run is selected, and drops it once none is", async () => {
    const { rerender } = render(
      <QueryClientProvider client={buildQueryClient()}>
        <LoopCenter definitionId="definition-a" loopRunId="run-a1" />
      </QueryClientProvider>,
    );

    // useLoopInspection's own effect follows {kind:"loop-iteration", loopRunId, iterationId} once
    // the run loads, which resolves the registered provider (inspector-provider-registry.ts) --
    // stubbed above to a bare, identifiable div, so its mere presence proves the follow/registry/
    // LazyFeature chain actually connected end to end, not just that nothing crashed.
    expect(await screen.findByTestId("loop-iteration-inspector-provider-stub")).toBeTruthy();

    // Clearing the run selection (back to a definition-only view) returns the Inspector to its own
    // overview state -- the same "no run selected" outcome LoopInspector's own `!run` branch used
    // to render directly, now reached through useWorkbenchInspection's `returnToOverview` instead.
    const client = buildQueryClient();
    rerender(
      <QueryClientProvider client={client}>
        <LoopCenter definitionId="definition-b" />
      </QueryClientProvider>,
    );
    await screen.findByTestId("loop-definition-overview");
    expect(screen.queryByTestId("loop-iteration-inspector-provider-stub")).toBeNull();
    expect(screen.getByText("No run selected")).toBeTruthy();
  });
});
