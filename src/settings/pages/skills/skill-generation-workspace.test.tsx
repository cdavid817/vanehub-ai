// @vitest-environment jsdom

import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import "../../../i18n";
import { activateAppLanguage } from "../../../i18n";
import {
  resetWebSkillGenerationForTest,
  seedWebGenerationJobForTest,
  webSkillGenerationClient,
} from "../../../services/web-skill-generation-client";
import { SkillGenerationWorkspace } from "./skill-generation-workspace";

function renderWorkspace(onOpenCurator = vi.fn()) {
  const client = new QueryClient({ defaultOptions: { queries: { retry: false }, mutations: { retry: false } } });
  return {
    ...render(<QueryClientProvider client={client}><SkillGenerationWorkspace initialWorkspaceId="workspace-one" onOpenCurator={onOpenCurator} service={webSkillGenerationClient} /></QueryClientProvider>),
    onOpenCurator,
  };
}

describe("Skill generation workspace", () => {
  beforeEach(async () => {
    resetWebSkillGenerationForTest();
    await activateAppLanguage("en");
  });

  it("discloses consent, limits, and permanent manual governance", async () => {
    renderWorkspace();
    expect(await screen.findByRole("heading", { name: "Generation lab" })).toBeTruthy();
    expect(screen.getAllByText("Manual review only").length).toBeGreaterThan(0);
    expect(await screen.findByText(/Only sanitized, bounded dossier fields leave this device/)).toBeTruthy();
    await userEvent.type(screen.getByLabelText("Provider profile ID"), "profile-one");
    await userEvent.type(screen.getByLabelText("Model ID"), "model-one");
    await userEvent.click(screen.getByRole("button", { name: "Review and enable" }));
    await screen.findByText("Enabled");
    expect(screen.queryByRole("button", { name: /auto.?apply|install|apply overlay/i })).toBeNull();
  });

  it("inspects seven stages, rendered draft, validation, dossier pagination, and provenance", async () => {
    const completed = seedWebGenerationJobForTest("completed-review");
    renderWorkspace();
    await userEvent.click(await screen.findByText(completed.seedId));
    expect(await screen.findByText("Seven-stage execution")).toBeTruthy();
    expect(screen.getAllByText(/Attempt 1/)).toHaveLength(7);
    expect(screen.getByText("Locally rendered draft")).toBeTruthy();
    expect(screen.getByText("Validation matrix")).toBeTruthy();
    await userEvent.click(screen.getByRole("tab", { name: "Evidence dossier" }));
    expect((await screen.findAllByText("Identity and provenance")).length).toBeGreaterThan(0);
    expect(await screen.findByRole("button", { name: "Next page" })).toBeTruthy();
    await userEvent.click(screen.getByRole("tab", { name: "Provenance" }));
    await waitFor(() => expect(screen.getByText("Model calls")).toBeTruthy());
    expect(screen.getByText("Tool receipts")).toBeTruthy();
  });

  it("cancels cooperatively and regenerates as a linked immutable job", async () => {
    const running = seedWebGenerationJobForTest("running-review");
    renderWorkspace();
    await userEvent.click(await screen.findByText(running.seedId));
    await userEvent.click(await screen.findByRole("button", { name: "Cancel" }));
    await waitFor(async () => expect((await webSkillGenerationClient.getGenerationJob(running.jobId))?.status).toBe("cancelled"));
    const regenerate = screen.getByRole("button", { name: "Regenerate" });
    await waitFor(() => expect((regenerate as HTMLButtonElement).disabled).toBe(false));
    await userEvent.click(regenerate);
    await waitFor(async () => expect((await webSkillGenerationClient.getGenerationJob(running.jobId))?.status).toBe("superseded"));
  });
});
