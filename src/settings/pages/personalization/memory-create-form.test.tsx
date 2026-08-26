// @vitest-environment jsdom

import { screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import "../../../i18n";
import { createAgentServiceDouble, renderWithAppProviders } from "../../../test/render";
import type { CreateMemoryInput, MemoryPage } from "../../../types/personalization-memory";
import { MemoryListSection } from "./memory-list-section";

function renderList(overrides: Parameters<typeof createAgentServiceDouble>[0] = {}) {
  const createPersonalizationMemory = vi.fn(async (input: CreateMemoryInput) => ({
    id: "mem-0000000000000009",
    name: input.name,
    description: input.description,
    memoryType: input.memoryType,
    content: input.content,
    scopeKind: input.scopeKind,
    workspaceKey: input.workspaceKey ?? null,
    audienceAgentIds: null,
    status: "active" as const,
    source: "explicit_user" as const,
    sensitivity: "normal" as const,
    revision: 1,
    sourceAgentId: null,
    createdAt: "2026-02-01T09:00:00Z",
    updatedAt: "2026-02-01T09:00:00Z",
  }));
  const service = createAgentServiceDouble({
    listPersonalizationAgentCapabilities: async () => [],
    listKnownProjects: async () => [
      { path: "/code/vanehub", displayName: "vanehub", isGit: true, lastOpenedAt: "2026-01-01T00:00:00Z" },
    ],
    listKnownRemoteWorkspaces: async () => [],
    resolvePersonalizationWorkspace: async () => ({ workspaceKey: "ws-1", kind: "local" as const }),
    queryPersonalizationMemories: async (): Promise<MemoryPage> => ({
      items: [],
      nextCursor: null,
      totalMatched: 0,
    }),
    createPersonalizationMemory,
    ...overrides,
  });
  const rendered = renderWithAppProviders(<MemoryListSection service={service} />);
  return { ...rendered, createPersonalizationMemory };
}

async function openForm() {
  await userEvent.click(await screen.findByTestId("personalization-create-open"));
  return screen.findByTestId("personalization-create-form");
}

describe("MemoryCreateForm", () => {
  it("stays closed until the user asks for it", async () => {
    renderList();
    await screen.findByTestId("personalization-create-open");

    expect(screen.queryByTestId("personalization-create-form")).toBeNull();
  });

  it("refuses to create a memory with no name or no content", async () => {
    renderList();
    await openForm();

    expect(screen.getByTestId("personalization-create-save").hasAttribute("disabled")).toBe(true);

    await userEvent.type(screen.getByTestId("personalization-create-name"), "npm-only");
    expect(screen.getByTestId("personalization-create-save").hasAttribute("disabled")).toBe(true);

    await userEvent.type(screen.getByTestId("personalization-create-content"), "   ");
    // Whitespace is not content, and the store refuses it; refusing here is what stops the user
    // finding that out only after pressing Create.
    expect(screen.getByTestId("personalization-create-save").hasAttribute("disabled")).toBe(true);
  });

  it("creates a global memory the user wrote", async () => {
    const world = renderList();
    await openForm();

    await userEvent.type(screen.getByTestId("personalization-create-name"), "npm-only");
    await userEvent.type(screen.getByTestId("personalization-create-content"), "This project uses npm.");
    await userEvent.click(screen.getByTestId("personalization-create-save"));

    await waitFor(() => {
      expect(world.createPersonalizationMemory).toHaveBeenCalledWith({
        name: "npm-only",
        description: "",
        memoryType: "user",
        content: "This project uses npm.",
        scopeKind: "global",
      });
    });
    expect(screen.queryByTestId("personalization-create-form")).toBeNull();
  });

  it("requires a workspace before creating a workspace memory", async () => {
    const world = renderList();
    await openForm();

    await userEvent.type(screen.getByTestId("personalization-create-name"), "npm-only");
    await userEvent.type(screen.getByTestId("personalization-create-content"), "This project uses npm.");
    await userEvent.selectOptions(screen.getByTestId("personalization-create-scope"), "workspace");
    await userEvent.selectOptions(screen.getByTestId("personalization-create-workspace"), "");

    expect(screen.getByTestId("personalization-create-save").hasAttribute("disabled")).toBe(true);

    await userEvent.selectOptions(screen.getByTestId("personalization-create-workspace"), "ws-1");
    await userEvent.click(screen.getByTestId("personalization-create-save"));

    await waitFor(() => {
      expect(world.createPersonalizationMemory).toHaveBeenCalledWith(
        expect.objectContaining({ scopeKind: "workspace", workspaceKey: "ws-1" }),
      );
    });
  });

  it("keeps the form and its text when the write fails", async () => {
    renderList({
      createPersonalizationMemory: async () => {
        throw new Error("personalization-storage-unavailable");
      },
    });
    await openForm();

    await userEvent.type(screen.getByTestId("personalization-create-name"), "npm-only");
    await userEvent.type(screen.getByTestId("personalization-create-content"), "Worth keeping.");
    await userEvent.click(screen.getByTestId("personalization-create-save"));

    await waitFor(() => {
      expect(screen.getByTestId("personalization-create-error")).toBeTruthy();
    });
    expect((screen.getByTestId("personalization-create-content") as HTMLTextAreaElement).value).toBe(
      "Worth keeping.",
    );
  });
});
