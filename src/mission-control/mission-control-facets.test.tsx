// @vitest-environment jsdom

import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import { activateAppLanguage, i18n } from "../i18n";
import type { MissionControlFacet, MissionControlFacetState, MissionControlRunDetail, MissionControlRunSummary } from "../types/mission-control";
import { MissionControlFacetPanel } from "./mission-control-facets";

afterEach(() => cleanup());

const ALL_FACETS: MissionControlFacet[] = ["overview", "timeline", "tools", "files", "review", "verification", "context", "usage", "logs"];

function runSummary(overrides: Partial<MissionControlRunSummary> = {}): MissionControlRunSummary {
  return {
    runId: "run-1", version: 1, ownerType: "agent", ownerId: "owner-1", agentId: "claude-code",
    title: "Run 1", state: "running", createdAt: "2026-08-16T00:00:00.000Z", updatedAt: "2026-08-16T00:00:00.000Z",
    endedAt: null, projectId: null, workspace: null, phase: null, attention: null, reasonCode: null,
    verification: "unavailable", tokens: null, cost: null, actions: [],
    // No session link: keeps the real Usage/Timeline/Tools/Files facets' own resolver a no-op
    // short-circuit in these router-focused tests, so they exercise routing only, not data fetching.
    navigation: null,
    runner: null,
    ...overrides,
  };
}

function detail(states: Partial<Record<MissionControlFacet, MissionControlFacetState>> = {}): MissionControlRunDetail {
  return { run: runSummary(), facets: ALL_FACETS.map((facet) => ({ facet, state: states[facet] ?? "available" })) };
}

describe("MissionControlFacetPanel", () => {
  it("renders the Overview facet's real content when it is available", async () => {
    await activateAppLanguage("en");
    render(<MissionControlFacetPanel detail={detail()} facet="overview" />);
    expect(screen.getByTestId("mission-control-overview-facet")).toBeTruthy();
  });

  it("renders the Usage facet when it is available", async () => {
    await activateAppLanguage("en");
    render(<MissionControlFacetPanel detail={detail()} facet="usage" />);
    expect(screen.getByTestId("mission-control-usage-facet")).toBeTruthy();
  });

  it.each([
    ["timeline", "mission-control-timeline-facet"],
    ["tools", "mission-control-tools-facet"],
    ["files", "mission-control-files-facet"],
  ] as const)("renders the %s facet when it is available", async (facet, testId) => {
    await activateAppLanguage("en");
    render(<MissionControlFacetPanel detail={detail()} facet={facet} />);
    expect(screen.getByTestId(testId)).toBeTruthy();
  });

  it.each(["review", "verification", "context", "logs"] as const)(
    "keeps the existing placeholder for the %s facet, unbuilt in this pass",
    async (facet) => {
      await activateAppLanguage("en");
      render(<MissionControlFacetPanel detail={detail()} facet={facet} />);
      expect(screen.getByText(new RegExp(`Selected: ${i18n.t(`missionControl.facet.${facet}`)}`))).toBeTruthy();
    },
  );

  it("falls back to the placeholder for overview/usage when the backend has not actually marked them available", async () => {
    await activateAppLanguage("en");
    const restrictedDetail = detail({ overview: "unavailable", usage: "restricted" });

    render(<MissionControlFacetPanel detail={restrictedDetail} facet="overview" />);
    expect(screen.queryByTestId("mission-control-overview-facet")).toBeNull();
    expect(screen.getByText(/Selected: Overview/)).toBeTruthy();
    cleanup();

    render(<MissionControlFacetPanel detail={restrictedDetail} facet="usage" />);
    expect(screen.queryByTestId("mission-control-usage-facet")).toBeNull();
    expect(screen.getByText(/Selected: Usage/)).toBeTruthy();
  });

  it("falls back to the placeholder for timeline/tools/files when the backend has not actually marked them available", async () => {
    await activateAppLanguage("en");
    const restrictedDetail = detail({ timeline: "unavailable", tools: "restricted", files: "unavailable" });

    render(<MissionControlFacetPanel detail={restrictedDetail} facet="timeline" />);
    expect(screen.queryByTestId("mission-control-timeline-facet")).toBeNull();
    expect(screen.getByText(/Selected: Timeline/)).toBeTruthy();
    cleanup();

    render(<MissionControlFacetPanel detail={restrictedDetail} facet="tools" />);
    expect(screen.queryByTestId("mission-control-tools-facet")).toBeNull();
    expect(screen.getByText(/Selected: Tools/)).toBeTruthy();
    cleanup();

    render(<MissionControlFacetPanel detail={restrictedDetail} facet="files" />);
    expect(screen.queryByTestId("mission-control-files-facet")).toBeNull();
    expect(screen.getByText(/Selected: Files/)).toBeTruthy();
  });
});
