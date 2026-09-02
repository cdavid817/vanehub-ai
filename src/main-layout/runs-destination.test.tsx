// @vitest-environment jsdom

import { useEffect } from "react";
import { fireEvent, render, screen } from "@testing-library/react";
import { beforeAll, beforeEach, describe, expect, it, vi } from "vitest";
import { activateAppLanguage } from "../i18n";
import { RunsDestination } from "./runs-destination";
import type { RunsSection } from "./workbench-route";

const lazyFeatureMounts = vi.hoisted(() => ({ count: 0 }));

// The real MissionControl/LoopCenter/ScheduledTasksPanel each reach real services on mount;
// this file tests RunsDestination's own routing/tab logic, not their content, so LazyFeature is
// replaced with a stub that exposes which loader+props it was asked to render, and how many times
// a fresh instance has mounted — the only way 5.13's "stays mounted, not remounted" claim is provable.
vi.mock("../components/lazy-feature", () => ({
  LazyFeature: ({ componentProps }: { componentProps: Record<string, unknown> }) => {
    useEffect(() => { lazyFeatureMounts.count += 1; }, []);
    return (
      <div
        data-initial-run-id={String(componentProps.initialRunId)}
        data-props={Object.keys(componentProps).sort().join(",")}
        data-schedule-id={String(componentProps.scheduleId)}
        data-testid="lazy-feature"
      />
    );
  },
}));

describe("RunsDestination", () => {
  beforeAll(async () => activateAppLanguage("en"));
  beforeEach(() => { lazyFeatureMounts.count = 0; });

  it("renders all five sections as tabs and marks the active one", () => {
    render(
      <RunsDestination
        agents={[]}
        location={{ section: "attention", runId: undefined }}
        onMissionControlNavigate={vi.fn()}
        onSectionChange={vi.fn()}
      />,
    );
    const tabs = screen.getAllByRole("tab");
    expect(tabs).toHaveLength(5);
    expect(screen.getByRole("tab", { name: "Attention inbox" }).getAttribute("aria-selected")).toBe("true");
    expect(screen.getByRole("tab", { name: "Active Runs" }).getAttribute("aria-selected")).toBe("false");
  });

  it("requests a section change with the exact section shape, not just its name", () => {
    const onSectionChange = vi.fn();
    render(
      <RunsDestination
        agents={[]}
        location={{ section: "attention", runId: undefined }}
        onMissionControlNavigate={vi.fn()}
        onSectionChange={onSectionChange}
      />,
    );
    fireEvent.click(screen.getByRole("tab", { name: "Loops" }));
    expect(onSectionChange).toHaveBeenCalledWith({ section: "loops" });
  });

  it.each(["attention", "active", "history"] as const)("routes %s to MissionControl with initialRunId and onNavigate wired", (section) => {
    render(
      <RunsDestination
        agents={[]}
        location={{ section, runId: undefined } as RunsSection}
        onMissionControlNavigate={vi.fn()}
        onSectionChange={vi.fn()}
      />,
    );
    expect(screen.getByTestId("lazy-feature").dataset.props).toBe("initialRunId,onNavigate");
  });

  it("routes loops to LoopCenter with onInspect wired", () => {
    render(
      <RunsDestination
        agents={[]}
        location={{ section: "loops", definitionId: undefined, loopRunId: undefined }}
        onInspectLoop={vi.fn()}
        onMissionControlNavigate={vi.fn()}
        onSectionChange={vi.fn()}
      />,
    );
    expect(screen.getByTestId("lazy-feature").dataset.props).toBe("onInspect");
  });

  it("routes schedules to ScheduledTasksPanel with the agent registry and scheduleId selection wired", () => {
    render(
      <RunsDestination
        agents={[]}
        location={{ section: "schedules", scheduleId: undefined }}
        onMissionControlNavigate={vi.fn()}
        onSectionChange={vi.fn()}
      />,
    );
    expect(screen.getByTestId("lazy-feature").dataset.props).toBe("agents,onSelectSchedule,scheduleId");
  });

  it("19.3: threads the route's own scheduleId through as ScheduledTasksPanel's current selection", () => {
    render(
      <RunsDestination
        agents={[]}
        location={{ section: "schedules", scheduleId: "task-42" }}
        onMissionControlNavigate={vi.fn()}
        onSectionChange={vi.fn()}
      />,
    );
    expect(screen.getByTestId("lazy-feature").dataset.scheduleId).toBe("task-42");
  });

  it("5.13: keeps a Loops draft alive (mounted, not remounted) across a switch to Schedules and back", () => {
    const { rerender } = render(
      <RunsDestination
        agents={[]}
        location={{ section: "loops", definitionId: undefined, loopRunId: undefined }}
        onInspectLoop={vi.fn()}
        onMissionControlNavigate={vi.fn()}
        onSectionChange={vi.fn()}
      />,
    );
    expect(lazyFeatureMounts.count).toBe(1);

    rerender(
      <RunsDestination
        agents={[]}
        location={{ section: "schedules", scheduleId: undefined }}
        onInspectLoop={vi.fn()}
        onMissionControlNavigate={vi.fn()}
        onSectionChange={vi.fn()}
      />,
    );
    // +1 for Schedules' own first mount — Loops does not mount a second time.
    expect(lazyFeatureMounts.count).toBe(2);
    const loopsInstance = screen.getAllByTestId("lazy-feature").find((element) => element.dataset.props === "onInspect");
    expect(loopsInstance).toBeTruthy();
    expect(loopsInstance?.closest("[hidden]")).toBeTruthy();

    rerender(
      <RunsDestination
        agents={[]}
        location={{ section: "loops", definitionId: undefined, loopRunId: undefined }}
        onInspectLoop={vi.fn()}
        onMissionControlNavigate={vi.fn()}
        onSectionChange={vi.fn()}
      />,
    );
    // Back on Loops: still the same two instances from before — a third mount would mean this
    // was destroyed and rebuilt, losing whatever draft it held.
    expect(lazyFeatureMounts.count).toBe(2);
  });

  it("5.13 boundary: Mission Control is not kept alive across a tab switch, unlike Loops/Schedules", () => {
    const { rerender } = render(
      <RunsDestination
        agents={[]}
        location={{ section: "attention", runId: undefined }}
        onMissionControlNavigate={vi.fn()}
        onSectionChange={vi.fn()}
      />,
    );
    expect(lazyFeatureMounts.count).toBe(1);

    rerender(
      <RunsDestination
        agents={[]}
        location={{ section: "loops", definitionId: undefined, loopRunId: undefined }}
        onInspectLoop={vi.fn()}
        onMissionControlNavigate={vi.fn()}
        onSectionChange={vi.fn()}
      />,
    );
    expect(lazyFeatureMounts.count).toBe(2);

    rerender(
      <RunsDestination
        agents={[]}
        location={{ section: "attention", runId: undefined }}
        onMissionControlNavigate={vi.fn()}
        onSectionChange={vi.fn()}
      />,
    );
    // A third mount: Mission Control was torn down when Loops became active and rebuilt here,
    // exactly the existing (documented, 4.8-covered-by-different-means) behavior for that section.
    expect(lazyFeatureMounts.count).toBe(3);
  });
});
