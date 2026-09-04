// @vitest-environment jsdom

import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeAll, beforeEach, describe, expect, it, vi } from "vitest";
import { MemoryRouter } from "react-router";
import { activateAppLanguage } from "../i18n";
import { agentService } from "../services/runtime-agent-client";
import { loopRunFixture } from "../test/loop-fixtures";
import type { LoopInspectionTarget } from "../types/loop";
import type { WorkbenchSelection } from "../types/workbench-selection";
import type { InspectorProviderContext } from "../ui/inspector/inspector-provider-registry";
import { LoopIterationInspectorProvider } from "./loop-iteration-inspector-provider";

/**
 * The `loop-iteration` Inspector provider (17.3 Piece B): resolves `getLoopRun(selection.loopRunId)`
 * and hands the result to `LoopInspectorBody` (extracted from `LoopInspector`, its own byte-level
 * output already covered by loop-center-states.test.tsx) -- this file stays at the loading/error/
 * found boundary and the `context.onInspectLoop` threading, mirroring session-overview.test.tsx's
 * own scope split against SessionOverviewSections.
 */

function renderProvider(loopRunId: string, context: InspectorProviderContext = {}) {
  const queryClient = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  const selection: WorkbenchSelection = { kind: "loop-iteration", iterationId: "iteration-1", loopRunId };
  return render(
    <MemoryRouter>
      <QueryClientProvider client={queryClient}>
        <LoopIterationInspectorProvider context={context} selection={selection} />
      </QueryClientProvider>
    </MemoryRouter>,
  );
}

beforeAll(async () => {
  await activateAppLanguage("en");
});

beforeEach(() => {
  vi.spyOn(agentService, "subscribeLoopEvents").mockResolvedValue(() => {});
  // AgentRunOwnerStatus (rendered by LoopInspectorBody's own "run" section) reads this directly --
  // unrelated to this file's own loading/error/found boundary, so it is stubbed to a harmless
  // empty page throughout.
  vi.spyOn(agentService, "listAgentRuns").mockResolvedValue({ items: [], limit: 1, offset: 0 });
});

afterEach(() => {
  vi.restoreAllMocks();
});

describe("LoopIterationInspectorProvider", () => {
  it("shows a loading state while the run is still loading", () => {
    vi.spyOn(agentService, "getLoopRun").mockReturnValue(new Promise(() => {}));

    renderProvider("run-1");

    expect(screen.getByRole("status")).toBeTruthy();
  });

  it("shows a retryable error when the run fails to load", async () => {
    const getLoopRun = vi.spyOn(agentService, "getLoopRun").mockRejectedValue(new Error("network down"));

    renderProvider("run-1");

    await waitFor(() => expect(screen.getByRole("alert")).toBeTruthy());
    expect(getLoopRun).toHaveBeenCalledWith("run-1");
    expect(getLoopRun).toHaveBeenCalledTimes(1);

    fireEvent.click(screen.getByRole("button", { name: "Retry" }));
    await waitFor(() => expect(getLoopRun).toHaveBeenCalledTimes(2));
  });

  it("renders the resolved run's own run/limits/workspace content, matching LoopInspector's own former column-3 render", async () => {
    const run = loopRunFixture("running", { activeOperationId: "operation-worker" });
    vi.spyOn(agentService, "getLoopRun").mockResolvedValue(run);

    renderProvider("run-1");

    expect(await screen.findByText("operation-worker")).toBeTruthy();
    expect(screen.getByText(run.projectPath)).toBeTruthy();
  });

  it("threads context.onInspectLoop into LoopInspectionActions the same way LoopInspector's own onInspect prop always did", async () => {
    const run = loopRunFixture("running", { activeOperationId: "operation-worker" });
    vi.spyOn(agentService, "getLoopRun").mockResolvedValue(run);
    const onInspectLoop = vi.fn<(target: LoopInspectionTarget) => void>();

    renderProvider("run-1", { onInspectLoop });
    const inspectButton = await screen.findByRole("button", { name: "Open Logs" });
    fireEvent.click(inspectButton);

    expect(onInspectLoop).toHaveBeenCalledWith({ sessionId: "worker-1", surface: "logs" });
  });

  /**
   * 9.9: the Run section's own summary (status/limits/workspace fields, none of which round-trip
   * to a page of their own) now links out to this run's real, route-bound full page instead of
   * leaving the reader with only this bounded card -- `RunsDestination`/`LoopCenter` (17.2) already
   * treat `definitionId`/`loopRunId` as real two-way route state, unlike `Projects`' own unconsumed
   * `projectId` slot, so this is a genuine deep link, not a guess.
   */
  it("links the Run section to this run's own authoritative full page in Runs > Loops", async () => {
    const run = loopRunFixture("running", { activeOperationId: "operation-worker" });
    vi.spyOn(agentService, "getLoopRun").mockResolvedValue(run);

    renderProvider("run-1");

    const link = await screen.findByRole("link", { name: "Open full run" });
    expect(link.getAttribute("href")).toBe(`/workspace/runs/loops/${run.definitionId}/${run.id}`);
  });

  // 21.12 "Inspector lazy-load" budget: mirrors mission-control-detail-panel.test.tsx's own
  // "facet-switching fetch exclusivity" (16.18) pattern, scoped to what this provider actually has
  // to be lazy about. `selection.iterationId` is never read here (this file's own doc comment,
  // confirmed by reading loop-iteration-inspector-provider.tsx directly) -- there is no real
  // per-iteration fetch boundary in this codebase to prove lazy on its own, since `getLoopRun`
  // already returns every iteration nested inside the one run. The real, checkable claim is one
  // level up: selecting one run's Inspector fetches only that run, never every run
  // (`listLoopRuns`) or the whole definition catalog (`listLoopDefinitions`) up front.
  it("fetches only the selected run, never the full run list or definition catalog", async () => {
    const run = loopRunFixture("running", { activeOperationId: "operation-worker" });
    const getLoopRun = vi.spyOn(agentService, "getLoopRun").mockResolvedValue(run);
    const listLoopRuns = vi.spyOn(agentService, "listLoopRuns");
    const listLoopDefinitions = vi.spyOn(agentService, "listLoopDefinitions");

    renderProvider("run-1");

    await waitFor(() => expect(getLoopRun).toHaveBeenCalledTimes(1));
    expect(getLoopRun).toHaveBeenCalledWith("run-1");
    expect(listLoopRuns).not.toHaveBeenCalled();
    expect(listLoopDefinitions).not.toHaveBeenCalled();
  });
});
