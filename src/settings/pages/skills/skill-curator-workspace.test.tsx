// @vitest-environment jsdom

import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it } from "vitest";
import { resetWebSkillCuratorForTest, webSkillCuratorClient } from "../../../adapters/web-skill-curator-client";
import "../../../i18n";
import { activateAppLanguage } from "../../../i18n";
import type { SkillCuratorService } from "../../../services/skill-curator-service";
import { SkillCuratorWorkspace } from "./skill-curator-workspace";

function renderWorkspace(workspace = "mock://deterministic", service: SkillCuratorService = webSkillCuratorClient) {
  const client = new QueryClient({ defaultOptions: { queries: { retry: false }, mutations: { retry: false } } });
  return render(<QueryClientProvider client={client}><SkillCuratorWorkspace initialWorkspaceId={workspace} service={service} /></QueryClientProvider>);
}

async function openCandidate() {
  const queue = await screen.findByRole("list", { name: "Curator candidate queue" });
  fireEvent.click(within(queue).getByRole("listitem"));
  await screen.findByRole("heading", { name: "review", level: 3 });
}

async function createDraftAndPreview() {
  await createDraft();
  await userEvent.click(screen.getByRole("button", { name: "Create current preview" }));
  await screen.findByText("Final effective change", { selector: "button" });
}

async function createDraft() {
  await userEvent.type(screen.getByLabelText("Learned guidance"), "Prefer bounded changes.");
  await userEvent.type(screen.getByLabelText("Evidence-bound rationale"), "Sanitized evidence supports this change.");
  await userEvent.type(screen.getByLabelText("Expected effective change"), "Adds one guidance block.");
  await userEvent.click(screen.getByRole("button", { name: "Save draft" }));
  await waitFor(() => expect((screen.getByRole("button", { name: "Create current preview" }) as HTMLButtonElement).disabled).toBe(false));
}

describe("Skill Curator workspace", () => {
  beforeEach(async () => {
    resetWebSkillCuratorForTest();
    await activateAppLanguage("en");
  });

  it("loads a service-backed queue and completes witnessed single-candidate approval", async () => {
    renderWorkspace();
    expect(await screen.findByText("1 candidates")).toBeTruthy();
    await openCandidate();
    expect(screen.getByText("9 / 9 checks")).toBeTruthy();
    expect(screen.getByText(/Target override, base editing/)).toBeTruthy();
    await userEvent.click(screen.getByRole("radio", { name: "Exact patch" }));
    expect(screen.getByLabelText("Exact existing text")).toBeTruthy();
    expect(screen.getByLabelText("Replacement text")).toBeTruthy();
    await userEvent.click(screen.getByRole("radio", { name: "Learned guidance" }));

    await createDraftAndPreview();
    const confirmation = screen.getByLabelText(/I reviewed this exact effective diff/);
    expect((screen.getByRole("button", { name: "Approve and apply Overlay" }) as HTMLButtonElement).disabled).toBe(true);
    await userEvent.click(confirmation);
    await userEvent.click(screen.getByRole("button", { name: "Approve and apply Overlay" }));

    await waitFor(() => expect(screen.getAllByText("Applied").length).toBeGreaterThan(0));
    expect(screen.getByRole("link", { name: /Open applied Overlay history/ })).toBeTruthy();
    expect(screen.queryByRole("button", { name: /approve all/i })).toBeNull();
  });

  it("requires categorized deferral, traps focus, and supports manual resume", async () => {
    renderWorkspace();
    await openCandidate();
    await userEvent.click(screen.getByRole("button", { name: "Defer" }));
    const dialog = screen.getByRole("dialog", { name: "Defer candidate" });
    const reason = within(dialog).getByLabelText("Required reason category");
    expect(document.activeElement).toBe(reason);
    expect((within(dialog).getByRole("button", { name: "Defer" }) as HTMLButtonElement).disabled).toBe(true);
    await userEvent.selectOptions(reason, "need_more_evidence");
    await userEvent.click(within(dialog).getByRole("button", { name: "Defer" }));
    await waitFor(() => expect(screen.getAllByText("Deferred").length).toBeGreaterThan(0));
    await userEvent.click(screen.getByRole("button", { name: "Resume review" }));
    await waitFor(() => expect(screen.queryByRole("button", { name: "Resume review" })).toBeNull());
  });

  it("shows accurate empty, loading, and error states without demo candidates", async () => {
    const emptyView = renderWorkspace("mock://empty");
    expect(screen.getByText("Loading Curator queue…")).toBeTruthy();
    expect(await screen.findByText("No matching candidates")).toBeTruthy();
    emptyView.unmount();

    const failing: SkillCuratorService = {
      ...webSkillCuratorClient,
      async querySkillCuratorQueue() { throw new Error("storage unavailable"); },
    };
    renderWorkspace("mock://error", failing);
    expect((await screen.findByRole("alert")).textContent).toContain("Curator queue could not be loaded");
  });

  it("uses responsive, theme-token surfaces and labeled filters", async () => {
    const view = renderWorkspace();
    await screen.findByText("1 candidates");
    expect(screen.getByRole("group", { name: "Curator queue filters" }).className).toContain("sm:grid-cols-2 xl:grid-cols-4");
    expect(view.container.querySelector(".bg-background")).not.toBeNull();
    expect(screen.getByLabelText("Risk")).toBeTruthy();
    expect(screen.getByLabelText("Notification")).toBeTruthy();
  });

  it("updates bounded policy controls without exposing automatic governance", async () => {
    renderWorkspace();
    await screen.findByText("1 candidates");
    await userEvent.click(screen.getByText("Queue policy and retention"));
    const notificationToggle = await screen.findByLabelText("Immediate notifications");
    await userEvent.click(notificationToggle);
    await userEvent.click(screen.getByRole("button", { name: "Save policy" }));
    await waitFor(async () => {
      const result = await webSkillCuratorClient.getSkillCuratorPolicy("mock://deterministic");
      expect(result.ok && result.value.notificationsEnabled).toBe(false);
    });
    expect(screen.queryByRole("checkbox", { name: /auto-apply/i })).toBeNull();
    expect(screen.queryByRole("button", { name: /approve all/i })).toBeNull();
  });

  it("surfaces apply failure and prepares a witnessed retry without automatic apply", async () => {
    renderWorkspace("mock://application-failure");
    await openCandidate();
    await createDraftAndPreview();
    await userEvent.click(screen.getByLabelText(/I reviewed this exact effective diff/));
    await userEvent.click(screen.getByRole("button", { name: "Approve and apply Overlay" }));
    await screen.findByText(/Overlay application failed with stable category/);
    await userEvent.click(screen.getByRole("button", { name: "Prepare retry" }));
    await waitFor(() => expect(screen.queryByRole("button", { name: "Prepare retry" })).toBeNull());
    expect(screen.getByText(/Save a valid draft, then create a witnessed Overlay preview/)).toBeTruthy();
  });

  it("explains high-risk, pinned, and superseded non-bypass states", async () => {
    const highRisk = renderWorkspace("mock://high-risk");
    await openCandidate();
    expect(screen.getByText(/High-risk candidates require explicit review/)).toBeTruthy();
    highRisk.unmount();

    resetWebSkillCuratorForTest();
    const pinned = renderWorkspace("mock://pinned");
    await openCandidate();
    await createDraft();
    await userEvent.click(screen.getByRole("button", { name: "Create current preview" }));
    expect(await screen.findByText(/target_pinned/)).toBeTruthy();
    pinned.unmount();

    resetWebSkillCuratorForTest();
    renderWorkspace("mock://supersede-on-preview");
    await openCandidate();
    await createDraft();
    await userEvent.click(screen.getByRole("button", { name: "Create current preview" }));
    expect(await screen.findByText(/newer assessment superseded this candidate/i)).toBeTruthy();
  });
});
