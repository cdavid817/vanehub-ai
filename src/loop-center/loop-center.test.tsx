// @vitest-environment jsdom

import { forwardRef } from "react";
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
// LoopInspector renders AgentRunOwnerStatus, which calls agentService.listAgentRuns -- a service
// method unrelated to 17.2's own selection wiring and out of scope for this file's mock. It
// always renders regardless of selection, so stubbing it (still forwardRef, matching its real
// signature, since loop-center.tsx passes it a ref) keeps these tests isolated to LoopCenter's own
// definition/run selection instead of the inspector panel's content.
vi.mock("./loop-inspector", () => ({
  LoopInspector: forwardRef<HTMLElement, Record<string, unknown>>((_props, ref) => <aside ref={ref} />),
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
