// @vitest-environment jsdom

import { fireEvent, render, screen } from "@testing-library/react";
import { beforeAll, describe, expect, it, vi } from "vitest";
import { activateAppLanguage } from "../i18n";
import { RunsDestination } from "./runs-destination";
import type { RunsSection } from "./workbench-route";

// The real MissionControl/LoopCenter/ScheduledTasksPanel each reach real services on mount;
// this file tests RunsDestination's own routing/tab logic, not their content, so LazyFeature is
// replaced with a stub that exposes which loader+props it was asked to render.
vi.mock("../components/lazy-feature", () => ({
  LazyFeature: ({ componentProps }: { componentProps: Record<string, unknown> }) => (
    <div
      data-initial-run-id={String(componentProps.initialRunId)}
      data-props={Object.keys(componentProps).sort().join(",")}
      data-testid="lazy-feature"
    />
  ),
}));

describe("RunsDestination", () => {
  beforeAll(async () => activateAppLanguage("en"));

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

  it("routes schedules to ScheduledTasksPanel with the agent registry wired", () => {
    render(
      <RunsDestination
        agents={[]}
        location={{ section: "schedules", scheduleId: undefined }}
        onMissionControlNavigate={vi.fn()}
        onSectionChange={vi.fn()}
      />,
    );
    expect(screen.getByTestId("lazy-feature").dataset.props).toBe("agents");
  });
});
