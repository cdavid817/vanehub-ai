// @vitest-environment jsdom

import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import { activateAppLanguage, i18n } from "../i18n";
import { formatAppDateTime, formatAppNumber } from "../i18n/format";
import type { MissionControlRunSummary } from "../types/mission-control";
import { OverviewFacet } from "./overview-facet";

afterEach(() => cleanup());

function run(overrides: Partial<MissionControlRunSummary> = {}): MissionControlRunSummary {
  return {
    runId: "run-1", version: 1, ownerType: "agent", ownerId: "owner-1", agentId: "claude-code",
    title: "Run 1", state: "running", createdAt: "2026-08-16T00:00:00.000Z", updatedAt: "2026-08-16T00:05:00.000Z",
    endedAt: "2026-08-16T00:10:00.000Z", projectId: "proj-1", workspace: "workspace-a", phase: "running",
    attention: "approval", reasonCode: "approval_required",
    verification: "passed", tokens: 12_345, cost: 1.5, actions: [],
    navigation: { kind: "session", id: "session-1", sessionId: null },
    runner: null,
    ...overrides,
  };
}

describe("OverviewFacet", () => {
  it("renders the run's project, workspace, phase, tokens, cost, and timestamps", async () => {
    // `i18n.changeLanguage` alone flips the active language marker but not what to render: only
    // `zh-CN` is bundled at module init (src/i18n/index.ts), so an un-loaded "en" falls silently
    // back to the fallback language's own copy — `activateAppLanguage` is what actually fetches it.
    await activateAppLanguage("en");
    render(<OverviewFacet run={run()} />);

    expect(screen.getByText("proj-1")).toBeTruthy();
    expect(screen.getByText("workspace-a")).toBeTruthy();
    expect(screen.getByText("running")).toBeTruthy();
    expect(screen.getByText(formatAppNumber(12_345, "en"))).toBeTruthy();
    expect(screen.getByText(formatAppNumber(1.5, "en", { maximumFractionDigits: 4 }))).toBeTruthy();
    // Computed with the same formatter rather than a hardcoded string: the exact rendering is
    // timezone-dependent, and this way the assertion tracks whatever timezone the test itself runs in.
    expect(screen.getByText(formatAppDateTime("2026-08-16T00:00:00.000Z", "en", { dateStyle: "medium", timeStyle: "short" }))).toBeTruthy();
    expect(screen.getByText(formatAppDateTime("2026-08-16T00:10:00.000Z", "en", { dateStyle: "medium", timeStyle: "short" }))).toBeTruthy();
  });

  it("pairs the attention kind with its raw reason code via the shared runner.reason fallback", async () => {
    await activateAppLanguage("en");
    render(<OverviewFacet run={run({ attention: "approval", reasonCode: "approval_required" })} />);

    // No `runner.reason.approval_required` key exists, so this also proves the defaultValue
    // fallback RunCard itself relies on is exercised here too, not a second implementation of it.
    expect(screen.getByText("Approval needed · approval_required")).toBeTruthy();
  });

  it("shows the 'not set' placeholder for every unpopulated field, and nothing else", async () => {
    await activateAppLanguage("en");
    render(<OverviewFacet run={run({
      projectId: null, workspace: null, phase: null, attention: null, reasonCode: null,
      tokens: null, cost: null, endedAt: null,
    })} />);

    // project, workspace, phase, attention, tokens, cost, endedAt — createdAt/updatedAt cannot be
    // null on this type, so they are deliberately absent from this count.
    expect(screen.getAllByText("Not set")).toHaveLength(7);
  });

  it("loads and translates every locale's new field and attention-kind labels, not falling back to zh-CN", async () => {
    for (const locale of ["en", "zh-CN", "zh-TW", "ja", "ko"] as const) {
      await activateAppLanguage(locale);
      // Direct proof the locale's own bundle is what is loaded — not.toBe(rawKey) alone would also
      // pass on a silent zh-CN fallback, which is exactly the bug this test exists to catch.
      expect(i18n.hasResourceBundle(locale, "translation")).toBe(true);
      const t = i18n.getFixedT(locale);
      for (const key of ["project", "workspace", "phase", "attention", "tokens", "cost", "createdAt", "updatedAt", "endedAt", "none"]) {
        expect(t(`missionControl.overview.${key}`)).not.toBe(`missionControl.overview.${key}`);
      }
      for (const kind of ["approval", "user", "stuck", "failed", "review"]) {
        expect(t(`missionControl.attentionKind.${kind}`)).not.toBe(`missionControl.attentionKind.${kind}`);
      }
    }
  });
});
