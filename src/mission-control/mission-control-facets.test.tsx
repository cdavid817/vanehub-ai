// @vitest-environment jsdom

import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import { activateAppLanguage, i18n } from "../i18n";
import type { MissionControlFacet, MissionControlFacetState, MissionControlRunDetail, MissionControlRunSummary } from "../types/mission-control";
import { MissionControlFacetPanel } from "./mission-control-facets";

afterEach(() => cleanup());

const ALL_FACETS: MissionControlFacet[] = ["overview", "timeline", "tools", "files", "review", "verification", "context", "usage", "logs"];
// 16.10/16.11: 7 of 9 facets have a real component now (up from 5) -- see mission-control-facets.tsx's
// own FACET_COMPONENTS doc comment for why verification/context stay unbuilt this pass.
const BUILT_FACETS: MissionControlFacet[] = ["overview", "timeline", "tools", "files", "review", "usage", "logs"];
const UNBUILT_FACETS: MissionControlFacet[] = ["verification", "context"];

const FACET_TEST_IDS: Record<string, string> = {
  overview: "mission-control-overview-facet",
  usage: "mission-control-usage-facet",
  timeline: "mission-control-timeline-facet",
  tools: "mission-control-tools-facet",
  files: "mission-control-files-facet",
  logs: "mission-control-logs-facet",
  review: "mission-control-review-facet",
};

function runSummary(overrides: Partial<MissionControlRunSummary> = {}): MissionControlRunSummary {
  return {
    runId: "run-1", version: 1, ownerType: "agent", ownerId: "owner-1", agentId: "claude-code",
    title: "Run 1", state: "running", createdAt: "2026-08-16T00:00:00.000Z", updatedAt: "2026-08-16T00:00:00.000Z",
    endedAt: null, projectId: null, workspace: null, phase: null, attention: null, reasonCode: null,
    verification: "unavailable", tokens: null, cost: null, actions: [],
    // No session/review link: keeps every real resolver- or navigation-backed facet's own fetch a
    // no-op short-circuit in these router-focused tests, so they exercise routing only, not data
    // fetching -- each facet's own dedicated test file already covers its real content in depth.
    navigation: null,
    runner: null,
    ...overrides,
  };
}

function detail(states: Partial<Record<MissionControlFacet, MissionControlFacetState>> = {}): MissionControlRunDetail {
  return { run: runSummary(), facets: ALL_FACETS.map((facet) => ({ facet, state: states[facet] ?? "available" })) };
}

describe("MissionControlFacetPanel", () => {
  it.each(BUILT_FACETS)("mounts the real %s facet component when the backend marks it available", async (facet) => {
    await activateAppLanguage("en");
    render(<MissionControlFacetPanel detail={detail()} facet={facet} />);
    expect(screen.getByTestId(FACET_TEST_IDS[facet])).toBeTruthy();
  });

  it.each(BUILT_FACETS)("shows the shared unavailable state, not the mounted component, when the backend marks %s unavailable", async (facet) => {
    await activateAppLanguage("en");
    render(<MissionControlFacetPanel detail={detail({ [facet]: "unavailable" })} facet={facet} />);
    expect(screen.queryByTestId(FACET_TEST_IDS[facet])).toBeNull();
    expect(screen.getByText("Unavailable")).toBeTruthy();
  });

  it.each(BUILT_FACETS)("shows the shared restricted state, not the mounted component, when the backend marks %s restricted", async (facet) => {
    await activateAppLanguage("en");
    render(<MissionControlFacetPanel detail={detail({ [facet]: "restricted" })} facet={facet} />);
    expect(screen.queryByTestId(FACET_TEST_IDS[facet])).toBeNull();
    expect(screen.getByText("Restricted")).toBeTruthy();
  });

  it.each(UNBUILT_FACETS)("shows a distinct not-built state for %s, regardless of backend availability", async (facet) => {
    await activateAppLanguage("en");
    render(<MissionControlFacetPanel detail={detail()} facet={facet} />);
    expect(screen.getByText("Not built yet")).toBeTruthy();
    // Distinct from the backend's own tone -- never claims "Unavailable"/"Restricted" for a gap that
    // is this client's own, not the backend's.
    expect(screen.queryByText("Unavailable")).toBeNull();
    expect(screen.queryByText("Restricted")).toBeNull();
  });

  it("shows the not-built state for verification/context even when the backend marks them available", async () => {
    // Defensive: a client-side gap does not become real content just because a future backend
    // starts reporting a link for a facet this client has no component for.
    await activateAppLanguage("en");
    render(<MissionControlFacetPanel detail={detail({ context: "available", verification: "available" })} facet="verification" />);
    expect(screen.getByText("Not built yet")).toBeTruthy();
  });

  it("never renders the retired generic lazyDetail placeholder, for any facet", async () => {
    await activateAppLanguage("en");
    const retiredText = i18n.t("missionControl.lazyDetail");
    for (const facet of ALL_FACETS) {
      const { unmount } = render(<MissionControlFacetPanel detail={detail()} facet={facet} />);
      expect(screen.queryByText(retiredText)).toBeNull();
      unmount();
    }
  });

  it("loads and translates the not-built copy in every locale, not falling back to zh-CN", async () => {
    for (const locale of ["en", "zh-CN", "zh-TW", "ja", "ko"] as const) {
      await activateAppLanguage(locale);
      expect(i18n.hasResourceBundle(locale, "translation")).toBe(true);
      const t = i18n.getFixedT(locale);
      expect(t("missionControl.facetNotBuilt.title")).not.toBe("missionControl.facetNotBuilt.title");
      expect(t("missionControl.facetNotBuilt.description")).not.toBe("missionControl.facetNotBuilt.description");
    }
    await activateAppLanguage("en");
  });
});
