// @vitest-environment jsdom

import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import "../../../i18n";
import { activateAppLanguage } from "../../../i18n";
import {
  resetWebSkillEvolutionOrchestrationForTest,
  seedWebEvolutionBreakerForTest,
  webSkillEvolutionOrchestrationClient,
} from "../../../services/web-skill-evolution-orchestration-client";
import { SkillEvolutionOrchestrationWorkspace } from "./skill-evolution-orchestration-workspace";

function renderWorkspace(onOpenCurator = vi.fn()) {
  const client = new QueryClient({
    defaultOptions: { mutations: { retry: false }, queries: { retry: false } },
  });
  return {
    ...render(<QueryClientProvider client={client}><SkillEvolutionOrchestrationWorkspace initial={{ workspaceId: "workspace-one" }} onOpenCurator={onOpenCurator} service={webSkillEvolutionOrchestrationClient} /></QueryClientProvider>),
    onOpenCurator,
  };
}

describe("Skill evolution orchestration workspace", () => {
  beforeEach(async () => {
    resetWebSkillEvolutionOrchestrationForTest();
    await activateAppLanguage("en");
  });

  it("discloses default-off policy and records an observe-only decision", async () => {
    renderWorkspace();
    expect(await screen.findByRole("heading", { name: "Evolution orchestration" })).toBeTruthy();
    expect(await screen.findByText("Web simulation")).toBeTruthy();
    expect(screen.getByText(/manual request only changes scheduling priority/i)).toBeTruthy();
    await userEvent.click(screen.getByRole("tab", { name: "Policy" }));
    await userEvent.click(screen.getByRole("radio", { name: /Observe/ }));
    await userEvent.type(screen.getByLabelText("Allowed Skill IDs"), "code-review");
    await userEvent.click(screen.getByLabelText(/locally consent/));
    await userEvent.click(screen.getByRole("button", { name: "Save policy" }));
    await waitFor(() => expect(screen.getByRole("radio", { name: /Observe/ }).getAttribute("aria-checked")).toBe("true"));
    await userEvent.click(screen.getByRole("button", { name: "Request manual run" }));
    await userEvent.click(screen.getByRole("tab", { name: "Decisions & history" }));
    expect((await screen.findAllByText("Would apply")).length).toBeGreaterThan(0);
    expect(screen.getByText("Deterministic authorized correction")).toBeTruthy();
    expect(screen.getByText("No automatic applications have been committed.")).toBeTruthy();
  });

  it("shows safe application probation, breaker recovery, and Curator-only rollback review", async () => {
    seedWebEvolutionBreakerForTest("workspace-one");
    const { onOpenCurator } = renderWorkspace();
    await screen.findByText("Scheduler overview");
    await userEvent.click(screen.getByRole("tab", { name: "Policy" }));
    await userEvent.click(screen.getByRole("radio", { name: /Enabled/ }));
    await userEvent.type(screen.getByLabelText("Allowed Skill IDs"), "readme-generation");
    await userEvent.click(screen.getByLabelText(/locally consent/));
    await userEvent.click(screen.getByRole("button", { name: "Save policy" }));
    await userEvent.click(screen.getByRole("button", { name: "Request manual run" }));
    await userEvent.click(screen.getByRole("tab", { name: "Decisions & history" }));
    expect(await screen.findByText("Applied")).toBeTruthy();
    expect(screen.getByText("Probation")).toBeTruthy();
    expect(screen.getAllByText(/correction text|diff content/i).length).toBeGreaterThan(0);

    await userEvent.click(screen.getByRole("tab", { name: "Safety" }));
    const breaker = await screen.findByText("Awaiting acknowledgement");
    const card = breaker.closest("li");
    expect(card).not.toBeNull();
    await userEvent.click(within(card!).getByRole("button", { name: "Open rollback review" }));
    expect(onOpenCurator).toHaveBeenCalledWith("workspace-one");
    await userEvent.click(within(card!).getByRole("button", { name: "Acknowledge recovery" }));
    await waitFor(() => expect(screen.getByText("Closed")).toBeTruthy());
    expect(screen.getByText(/never rolls a Skill back automatically/i)).toBeTruthy();
  });

  it("uses responsive theme-token surfaces and labeled keyboard tabs", async () => {
    const view = renderWorkspace();
    await screen.findByText("Scheduler overview");
    expect(screen.getByRole("tablist", { name: "Evolution orchestration views" })).toBeTruthy();
    const runsTab = screen.getByRole("tab", { name: "Runs" });
    runsTab.focus();
    await userEvent.keyboard("{ArrowRight}");
    expect(document.activeElement).toBe(screen.getByRole("tab", { name: "Policy" }));
    expect(view.container.querySelector(".sm\\:grid-cols-4")).not.toBeNull();
    expect(view.container.querySelector(".bg-background")).not.toBeNull();
  });
});
