// @vitest-environment jsdom

import { screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import "../../../i18n";
import { createAgentServiceDouble, renderWithAppProviders } from "../../../test/render";
import type {
  MemoryDetail,
  MemoryPage,
  MemorySummary,
  UpdateMemoryInput,
} from "../../../types/personalization-memory";
import { MemoryListSection } from "./memory-list-section";

const ID = "mem-0000000000000001";

function detail(overrides: Partial<MemoryDetail> = {}): MemoryDetail {
  return {
    id: ID,
    name: "vanehub-uses-npm",
    description: "This project pins npm.",
    memoryType: "project",
    content: "VaneHub AI uses npm; package-lock.json is authoritative.",
    scopeKind: "workspace",
    workspaceKey: "ws-vanehub",
    audienceAgentIds: ["claude-code"],
    status: "active",
    source: "onepiece_automatic",
    sensitivity: "normal",
    revision: 4,
    sourceAgentId: "onepiece",
    sourceSessionId: null,
    createdAt: "2026-01-01T09:00:00Z",
    updatedAt: "2026-02-01T09:00:00Z",
    ...overrides,
  };
}

function summary(): MemorySummary {
  const record = detail();
  return {
    id: record.id,
    name: record.name,
    description: record.description,
    memoryType: record.memoryType,
    scopeKind: record.scopeKind,
    workspaceKey: record.workspaceKey,
    status: record.status,
    source: record.source,
    sensitivity: "normal",
    revision: record.revision,
    updatedAt: record.updatedAt,
  };
}

function renderList(
  overrides: Parameters<typeof createAgentServiceDouble>[0] = {},
  options: { onOpenSession?: (sessionId: string) => void; record?: MemoryDetail } = {},
) {
  let stored = options.record ?? detail();
  const updatePersonalizationMemory = vi.fn(async (input: UpdateMemoryInput) => {
    if (input.expectedRevision !== stored.revision) {
      throw new Error(
        `personalization-revision-conflict: expected ${input.expectedRevision}, stored ${stored.revision}`,
      );
    }
    stored = { ...stored, ...input, revision: stored.revision + 1 } as MemoryDetail;
    return stored;
  });
  const deletePersonalizationMemory = vi.fn(async (_id: string, expectedRevision?: number) => {
    if (expectedRevision !== stored.revision) {
      throw new Error(
        `personalization-revision-conflict: expected ${expectedRevision}, stored ${stored.revision}`,
      );
    }
    return { matched: 1, deletedFiles: 1, removedProjectionRows: 1, revokedRetrievalEntries: 1, quarantined: 0, failures: [] };
  });
  const service = createAgentServiceDouble({
    listPersonalizationAgentCapabilities: async () => [],
    listKnownProjects: async () => [],
    listKnownRemoteWorkspaces: async () => [],
    queryPersonalizationMemories: async (): Promise<MemoryPage> => ({
      items: [summary()],
      nextCursor: null,
      totalMatched: 1,
    }),
    getPersonalizationMemory: async () => stored,
    updatePersonalizationMemory,
    deletePersonalizationMemory,
    ...overrides,
  });
  const rendered = renderWithAppProviders(
    <MemoryListSection onOpenSession={options.onOpenSession} service={service} />,
  );
  return {
    ...rendered,
    updatePersonalizationMemory,
    deletePersonalizationMemory,
    moveStore: (next: MemoryDetail) => {
      stored = next;
    },
    current: () => stored,
  };
}

async function openRow() {
  await userEvent.click(await screen.findByTestId(`personalization-memory-open-${ID}`));
  return screen.findByTestId("personalization-detail");
}

describe("MemoryDetailPanel", () => {
  it("says nothing is open before a row is chosen", async () => {
    renderList();

    expect(await screen.findByTestId("personalization-detail-empty")).toBeTruthy();
  });

  it("reads the body only for the row the user opened", async () => {
    renderList();
    await screen.findByTestId(`personalization-memory-open-${ID}`);

    // The list carried no body, so nothing showed it until now.
    expect(screen.queryByTestId("personalization-detail-body")).toBeNull();

    await openRow();

    expect(screen.getByTestId("personalization-detail-body").textContent).toContain(
      "package-lock.json is authoritative",
    );
  });

  it("shows the metadata, provenance and timestamps recorded about it", async () => {
    renderList();
    const panel = await openRow();
    const metadata = within(panel).getByTestId("personalization-detail-metadata");

    expect(metadata.textContent).toContain("ws-vanehub");
    expect(metadata.textContent).toContain("claude-code");
    expect(metadata.textContent).toContain("onepiece");
    expect(metadata.textContent).toContain("修订号 4");
  });

  it("says every Agent rather than showing an empty audience", async () => {
    renderList({ getPersonalizationMemory: async () => detail({ audienceAgentIds: null }) });
    const panel = await openRow();

    // An empty list would read as "no Agent can see this", which is the opposite of what it means.
    expect(within(panel).getByTestId("personalization-detail-metadata").textContent).toContain(
      "全部 Agent",
    );
  });

  it("saves an edit with the revision the user was looking at", async () => {
    const world = renderList();
    await openRow();

    await userEvent.click(screen.getByTestId("personalization-detail-edit"));
    const content = screen.getByTestId("personalization-detail-content");
    await userEvent.clear(content);
    await userEvent.type(content, "npm only.");
    await userEvent.click(screen.getByTestId("personalization-detail-save"));

    await waitFor(() => {
      expect(world.updatePersonalizationMemory).toHaveBeenCalledWith(
        expect.objectContaining({ id: ID, expectedRevision: 4, content: "npm only." }),
      );
    });
    expect(world.current().content).toBe("npm only.");
  });

  it("archives and reactivates through the same write", async () => {
    const world = renderList();
    await openRow();

    await userEvent.click(screen.getByTestId("personalization-detail-status"));

    await waitFor(() => {
      expect(world.updatePersonalizationMemory).toHaveBeenCalledWith(
        expect.objectContaining({ status: "archived", expectedRevision: 4 }),
      );
    });

    await waitFor(() => {
      expect(screen.getByTestId("personalization-detail-status").textContent).toContain("重新启用");
    });
  });

  it("refuses to overwrite a memory that changed while it was open", async () => {
    const world = renderList();
    await openRow();

    await userEvent.click(screen.getByTestId("personalization-detail-edit"));
    world.moveStore(detail({ revision: 9, content: "Changed elsewhere." }));
    await userEvent.type(screen.getByTestId("personalization-detail-name"), "!");
    await userEvent.click(screen.getByTestId("personalization-detail-save"));

    await waitFor(() => {
      expect(screen.getByTestId("personalization-detail-conflict")).toBeTruthy();
    });
    expect(world.current().content).toBe("Changed elsewhere.");
  });

  it("deletes with the revision the user was looking at", async () => {
    const world = renderList();
    await openRow();

    await userEvent.click(screen.getByTestId("personalization-detail-delete"));
    await userEvent.click(await screen.findByRole("button", { name: "确认" }));

    // A delete without a revision removes whatever is there now, so a stale panel destroys an edit
    // its owner never saw.
    await waitFor(() => {
      expect(world.deletePersonalizationMemory).toHaveBeenCalledWith(ID, 4);
    });
    expect(await screen.findByTestId("personalization-detail-deleted")).toBeTruthy();
  });

  it("keeps the edit when a save fails for another reason", async () => {
    renderList({
      updatePersonalizationMemory: async () => {
        throw new Error("personalization-storage-unavailable");
      },
    });
    await openRow();

    await userEvent.click(screen.getByTestId("personalization-detail-edit"));
    const content = screen.getByTestId("personalization-detail-content");
    await userEvent.clear(content);
    await userEvent.type(content, "Worth keeping.");
    await userEvent.click(screen.getByTestId("personalization-detail-save"));

    await waitFor(() => {
      expect(screen.getByTestId("personalization-detail-failure")).toBeTruthy();
    });
    expect((screen.getByTestId("personalization-detail-content") as HTMLTextAreaElement).value).toBe(
      "Worth keeping.",
    );
  });

  /**
   * A memory outlives the conversation it was extracted from, and "where did this come from" is the
   * first question a surprising one raises. The link is offered only when there is both a session
   * recorded and somewhere to open it.
   */
  it("offers the session a memory was recorded in", async () => {
    const onOpenSession = vi.fn();
    renderList({}, { onOpenSession, record: detail({ sourceSessionId: "ses-42" }) });
    await openRow();

    await userEvent.click(await screen.findByTestId("personalization-detail-open-session"));

    expect(onOpenSession).toHaveBeenCalledWith("ses-42");
  });

  it("offers nothing to open for a memory the user wrote themselves", async () => {
    renderList({}, { onOpenSession: vi.fn(), record: detail({ sourceSessionId: null }) });
    await openRow();

    expect(screen.queryByTestId("personalization-detail-open-session")).toBeNull();
  });

  it("offers nothing to open when the surface cannot navigate", async () => {
    // The settings shell supplies the route; a caller that has none must not render a dead control.
    renderList({}, { record: detail({ sourceSessionId: "ses-42" }) });
    await openRow();

    expect(screen.queryByTestId("personalization-detail-open-session")).toBeNull();
  });

  it("forgets one record's edit when another is opened", async () => {
    renderList();
    await openRow();
    await userEvent.click(screen.getByTestId("personalization-detail-edit"));
    await userEvent.type(screen.getByTestId("personalization-detail-name"), "-edited");

    await userEvent.click(screen.getByTestId("personalization-detail-close"));
    await openRow();

    // Carrying a draft across would offer one record's text as an edit to another.
    expect(screen.queryByTestId("personalization-detail-name")).toBeNull();
    expect(screen.getByTestId("personalization-detail")).toBeTruthy();
  });
});
