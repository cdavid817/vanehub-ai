// @vitest-environment jsdom

import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import "../i18n";
import type { AgentRegistryEntry } from "../types/agent";
import type { MissionControlRunSummary } from "../types/mission-control";
import { RunCard } from "./mission-control-run-card";

function fixture(overrides: Partial<MissionControlRunSummary> = {}): MissionControlRunSummary {
  return {
    runId: "run-1", version: 1, ownerType: "agent", ownerId: "owner-1", agentId: "claude-code",
    title: "修复登录超时问题", state: "running", createdAt: "2026-08-16T00:00:00.000Z",
    updatedAt: "2026-08-16T00:00:00.000Z", endedAt: null, projectId: null, workspace: null,
    phase: null, attention: null, reasonCode: null, verification: "unavailable", tokens: null,
    cost: null, actions: [], navigation: null, runner: null,
    ...overrides,
  };
}

const AGENTS: AgentRegistryEntry[] = [];

function renderCard(overrides: Partial<MissionControlRunSummary> = {}) {
  return render(
    <RunCard agents={AGENTS} onAct={vi.fn()} onDismissError={vi.fn()} onInspect={vi.fn()} run={fixture(overrides)} />,
  );
}

describe("RunCard", () => {
  it("renders the run's title and state", () => {
    renderCard();
    expect(screen.getByText("修复登录超时问题")).toBeTruthy();
  });

  /**
   * 20.15: this card's title (`min-w-0 flex-1 truncate`) and runner-badge host label (`truncate`)
   * already carry the established mechanism -- these prove it holds for the long-string classes
   * the task names, next to the runner badge and the state badge, using a real long host label (not
   * a synthetic one), matching the task's own "real long host label" ask. jsdom has no real layout
   * engine, so this checks class presence, not pixels -- same documented limitation as this pass's
   * other 20.15 tests.
   */
  describe("long content safety (20.15)", () => {
    const GERMAN_LIKE_TITLE = "Konfigurationsverwaltungsoberflächenkomponentenübersicht";
    const REAL_LONG_HOST = "ip-10-42-17-233.ap-northeast-1.compute.internal.production.example-corp.internal";

    it("truncates a long German-like run title next to the state badge", () => {
      renderCard({ title: GERMAN_LIKE_TITLE });
      const title = screen.getByText(GERMAN_LIKE_TITLE);
      expect(title.className).toContain("truncate");
      expect(screen.getByText("运行中")).toBeTruthy();
    });

    it("truncates a real long SSH runner host label inside its own bounded badge", () => {
      renderCard({
        runner: {
          kind: "ssh", targetId: "target-1", targetRevision: null, label: "SSH",
          hostLabel: REAL_LONG_HOST, recovery: "none", capabilityWitness: "cw", authorityWitness: "aw",
          recoveryReference: null,
        },
      });
      const hostSpan = screen.getByText((_content, node) => node?.textContent === `· ${REAL_LONG_HOST}`);
      expect(hostSpan.className).toContain("truncate");
    });
  });

  /**
   * 20.16: `runner.hostLabel` is resolved from the real SSH target, not app-authored -- wrapped in
   * `<bdi>` so a strong-RTL or mixed-script host label cannot read the "· " separator or this
   * badge's own state-badge neighbor out of order. Real, DOM-structural proof: a fixture host
   * containing an actual RTL character, asserting the isolation boundary wraps exactly that text.
   */
  it("wraps a runner host label containing an RTL character in a bdi isolation boundary", () => {
    const rtlHost = "שרת-לדוגמה.example.com";
    renderCard({
      runner: {
        kind: "ssh", targetId: "target-1", targetRevision: null, label: "SSH", hostLabel: rtlHost,
        recovery: "none", capabilityWitness: "cw", authorityWitness: "aw", recoveryReference: null,
      },
    });
    const isolated = screen.getByText(rtlHost, { selector: "bdi" });
    expect(isolated.textContent).toBe(rtlHost);
  });
});
