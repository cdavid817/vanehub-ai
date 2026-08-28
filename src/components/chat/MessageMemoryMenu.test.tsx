// @vitest-environment jsdom

import { screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import "../../i18n";
import { createAgentServiceDouble, renderWithAppProviders } from "../../test/render";
import type { CreateMemoryInput, MemoryPage } from "../../types/personalization-memory";
import { deriveMemoryName, MessageMemoryMenu } from "./MessageMemoryMenu";

const CONTENT = "This project pins npm; pnpm breaks the katex chunk split.";

function renderMenu(
  projectPath: string | null,
  overrides: Parameters<typeof createAgentServiceDouble>[0] = {},
) {
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
    sourceSessionId: null,
    createdAt: "2026-02-01T09:00:00Z",
    updatedAt: "2026-02-01T09:00:00Z",
  }));
  const resolvePersonalizationWorkspace = vi.fn(async () => ({
    workspaceKey: "ws-resolved",
    kind: "local" as const,
  }));
  const updatePersonalizationMemory = vi.fn(async () => {
    throw new Error("not used by this test");
  });
  const service = createAgentServiceDouble({
    createPersonalizationMemory,
    resolvePersonalizationWorkspace,
    updatePersonalizationMemory,
    queryPersonalizationMemories: async (): Promise<MemoryPage> => ({
      items: [
        {
          id: "mem-0000000000000002",
          name: "vanehub-uses-pnpm",
          description: "Wrong: says pnpm.",
          memoryType: "project",
          scopeKind: "workspace",
          workspaceKey: "ws-resolved",
          status: "active",
          source: "onepiece_automatic",
          sensitivity: "normal",
          revision: 3,
          updatedAt: "2026-02-01T09:00:00Z",
        },
      ],
      nextCursor: null,
      totalMatched: 1,
    }),
    ...overrides,
  });
  const rendered = renderWithAppProviders(
    <MessageMemoryMenu
      content={CONTENT}
      context={{ agentId: "synthetic-lab-agent", projectPath }}
      service={service}
    />,
  );
  return { ...rendered, createPersonalizationMemory, resolvePersonalizationWorkspace, service };
}

describe("MessageMemoryMenu", () => {
  it("derives a name from the text and falls back when nothing slugs", () => {
    expect(deriveMemoryName("This project pins npm", "fallback")).toBe("this-project-pins-npm");
    // Non-ASCII slugs to nothing, which would otherwise produce a memory with an empty name.
    expect(deriveMemoryName("日本語のみ", "fallback")).toBe("fallback");
  });

  it("remembers globally with the user recorded as the author", async () => {
    const world = renderMenu("/code/vanehub");

    await userEvent.click(screen.getByTestId("message-remember-global"));

    await waitFor(() => {
      expect(world.createPersonalizationMemory).toHaveBeenCalledWith(
        expect.objectContaining({ scopeKind: "global", content: CONTENT }),
      );
    });
    expect(await screen.findByTestId("message-memory-saved")).toBeTruthy();
  });

  it("resolves the project natively before scoping a memory to it", async () => {
    const world = renderMenu("/code/vanehub");

    await userEvent.click(screen.getByTestId("message-remember-project"));

    // A key this build invented would belong to a workspace nothing resolves to.
    await waitFor(() => {
      expect(world.resolvePersonalizationWorkspace).toHaveBeenCalledWith({
        projectPath: "/code/vanehub",
      });
    });
    expect(world.createPersonalizationMemory).toHaveBeenCalledWith(
      expect.objectContaining({ scopeKind: "workspace", workspaceKey: "ws-resolved" }),
    );
  });

  it("offers no project scope when the session has no project", async () => {
    const world = renderMenu(null);

    // Silently writing a global memory instead would store something the user did not ask for.
    expect(screen.getByTestId("message-remember-project").hasAttribute("disabled")).toBe(true);
    expect(world.createPersonalizationMemory).not.toHaveBeenCalled();
  });

  it("says nothing was changed when the write fails", async () => {
    renderMenu("/code/vanehub", {
      createPersonalizationMemory: async () => {
        throw new Error("personalization-storage-unavailable");
      },
    });

    await userEvent.click(screen.getByTestId("message-remember-global"));

    await waitFor(() => {
      expect(screen.getByTestId("message-memory-failed")).toBeTruthy();
    });
    expect(screen.queryByTestId("message-memory-saved")).toBeNull();
  });

  it("lists only memories this Agent can read when forgetting one", async () => {
    const queryPersonalizationMemories = vi.fn(async (): Promise<MemoryPage> => ({
      items: [],
      nextCursor: null,
      totalMatched: 0,
    }));
    renderMenu("/code/vanehub", { queryPersonalizationMemories });

    await userEvent.click(screen.getByTestId("message-forget-open"));

    // Listing every memory would offer records this Agent could never have been influenced by.
    await waitFor(() => {
      expect(queryPersonalizationMemories).toHaveBeenCalledWith(
        expect.objectContaining({ audienceAgentId: "synthetic-lab-agent", status: "active" }),
      );
    });
  });

  it("forgets with the revision the row was rendered with", async () => {
    const updatePersonalizationMemory = vi.fn(async () => ({
      id: "mem-0000000000000002",
      name: "vanehub-uses-pnpm",
      description: "Wrong: says pnpm.",
      memoryType: "project" as const,
      content: "",
      scopeKind: "workspace" as const,
      workspaceKey: "ws-resolved",
      audienceAgentIds: null,
      status: "archived" as const,
      source: "onepiece_automatic" as const,
      sensitivity: "normal" as const,
      revision: 4,
      sourceAgentId: null,
      sourceSessionId: null,
      createdAt: "2026-02-01T09:00:00Z",
      updatedAt: "2026-02-01T09:00:00Z",
    }));
    renderMenu("/code/vanehub", { updatePersonalizationMemory });

    await userEvent.click(screen.getByTestId("message-forget-open"));
    await userEvent.click(await screen.findByTestId("message-memory-forget-mem-0000000000000002"));

    // Forgetting without it would land on an edit made since the row was drawn.
    await waitFor(() => {
      expect(updatePersonalizationMemory).toHaveBeenCalledWith({
        id: "mem-0000000000000002",
        expectedRevision: 3,
        status: "archived",
      });
    });
  });

  it("corrects the text rather than only archiving it", async () => {
    const updatePersonalizationMemory = vi.fn(async () => {
      throw new Error("unused");
    });
    renderMenu("/code/vanehub", { updatePersonalizationMemory });

    await userEvent.click(screen.getByTestId("message-forget-open"));
    await userEvent.click(await screen.findByTestId("message-memory-correct-mem-0000000000000002"));
    await userEvent.type(
      screen.getByTestId("message-memory-correction-mem-0000000000000002"),
      "npm only.",
    );
    await userEvent.click(screen.getByTestId("message-memory-correct-save-mem-0000000000000002"));

    await waitFor(() => {
      expect(updatePersonalizationMemory).toHaveBeenCalledWith({
        id: "mem-0000000000000002",
        expectedRevision: 3,
        content: "npm only.",
      });
    });
  });
});
