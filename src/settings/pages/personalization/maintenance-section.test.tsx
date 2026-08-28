// @vitest-environment jsdom

import { screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import "../../../i18n";
import { createAgentServiceDouble, renderWithAppProviders } from "../../../test/render";
import type { PersonalizationHealth } from "../../../types/personalization";
import type { MaintenanceResult } from "../../../types/personalization-memory";
import { PersonalizationMaintenanceSection } from "./maintenance-section";

function health(overrides: Partial<PersonalizationHealth> = {}): PersonalizationHealth {
  return {
    state: "ready",
    memoryAvailable: true,
    pendingCandidates: 2,
    lastReconciledAt: null,
    repairRequired: false,
    ...overrides,
  };
}

function outcome(overrides: Partial<MaintenanceResult> = {}): MaintenanceResult {
  return {
    matched: 12,
    deletedFiles: 0,
    removedProjectionRows: 3,
    revokedRetrievalEntries: 1,
    quarantined: 2,
    failures: [],
    ...overrides,
  };
}

function renderSection(overrides: Parameters<typeof createAgentServiceDouble>[0] = {}) {
  const reconcilePersonalizationMemories = vi.fn(async () => outcome());
  const service = createAgentServiceDouble({
    getPersonalizationHealth: async () => health(),
    reconcilePersonalizationMemories,
    ...overrides,
  });
  const rendered = renderWithAppProviders(<PersonalizationMaintenanceSection service={service} />);
  return { ...rendered, reconcilePersonalizationMemories };
}

describe("PersonalizationMaintenanceSection", () => {
  it("reports the store's own state rather than assuming it", async () => {
    renderSection({ getPersonalizationHealth: async () => health({ state: "migrating", memoryAvailable: false }) });

    await waitFor(() => {
      expect(screen.getByTestId("personalization-maintenance-state").textContent).toContain("迁移中");
    });
  });

  it("marks a store that needs repair before it can be used", async () => {
    renderSection({
      getPersonalizationHealth: async () =>
        health({ state: "repair_required", memoryAvailable: false, repairRequired: true }),
    });

    await waitFor(() => {
      expect(screen.getByTestId("personalization-maintenance-repair")).toBeTruthy();
    });
  });

  it("says a rebuild has never run rather than showing zeros nobody measured", async () => {
    renderSection();

    await waitFor(() => {
      expect(screen.getByTestId("personalization-maintenance-last-run").textContent).toContain(
        "从未执行过重建",
      );
    });
    // Nothing counts malformed or quarantined entries until a rebuild walks the directory, so
    // there is no result to show yet.
    expect(screen.queryByTestId("personalization-maintenance-result")).toBeNull();
  });

  it("says when the last rebuild ran once one has", async () => {
    renderSection({
      getPersonalizationHealth: async () => health({ lastReconciledAt: "2026-02-01T09:00:00Z" }),
    });

    await waitFor(() => {
      expect(screen.getByTestId("personalization-maintenance-last-run").textContent).toContain(
        "上次重建于",
      );
    });
  });

  it("reports what the rebuild found on each surface", async () => {
    const world = renderSection();
    await screen.findByTestId("personalization-maintenance-rebuild");

    await userEvent.click(screen.getByTestId("personalization-maintenance-rebuild"));

    await waitFor(() => {
      expect(world.reconcilePersonalizationMemories).toHaveBeenCalledTimes(1);
    });
    const result = await screen.findByTestId("personalization-maintenance-result");
    expect(result.textContent).toContain("12");
    expect(result.textContent).toContain("3");
    expect(result.textContent).toContain("2");
    expect(screen.getByTestId("personalization-maintenance-clean")).toBeTruthy();
  });

  it("says which surfaces it could not repair instead of reporting a clean run", async () => {
    renderSection({
      reconcilePersonalizationMemories: async () => outcome({ failures: ["retrieval-index"] }),
    });
    await screen.findByTestId("personalization-maintenance-rebuild");

    await userEvent.click(screen.getByTestId("personalization-maintenance-rebuild"));

    const partial = await screen.findByTestId("personalization-maintenance-partial");
    expect(partial.textContent).toContain("检索索引");
    expect(screen.queryByTestId("personalization-maintenance-clean")).toBeNull();
  });

  it("says nothing changed when the rebuild does not run", async () => {
    renderSection({
      reconcilePersonalizationMemories: async () => {
        throw new Error("personalization-maintenance-busy");
      },
    });
    await screen.findByTestId("personalization-maintenance-rebuild");

    await userEvent.click(screen.getByTestId("personalization-maintenance-rebuild"));

    await waitFor(() => {
      expect(screen.getByTestId("personalization-maintenance-failed")).toBeTruthy();
    });
    expect(screen.queryByTestId("personalization-maintenance-result")).toBeNull();
  });

  it("says the state is unreadable rather than rendering a healthy store", async () => {
    renderSection({
      getPersonalizationHealth: async () => {
        throw new Error("personalization-storage-unavailable");
      },
    });

    await waitFor(() => {
      expect(screen.getByTestId("personalization-maintenance-error")).toBeTruthy();
    });
    expect(screen.queryByTestId("personalization-maintenance-rebuild")).toBeNull();
  });
});
