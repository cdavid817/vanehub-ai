// @vitest-environment jsdom

import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";
import "../i18n";
import { loopQueryKeys } from "../hooks/loop-query";
import { agentService } from "../services/runtime-agent-client";
import { loopDefinitionFixture, loopRunFixture } from "../test/loop-fixtures";
import type { LoopDefinition } from "../types/loop";
import { LoopDefinitionOverview } from "./loop-definition-overview";

describe("LoopDefinitionOverview", () => {
  afterEach(() => vi.restoreAllMocks());

  it("shows the full saved definition and guards actions while a run is active", async () => {
    vi.spyOn(agentService, "listAgents").mockResolvedValue([{
      id: "codex-cli",
      displayName: "Codex CLI",
      provider: "test",
      launch: { kind: "cli", command: "codex" },
      supportedInteractionModes: ["cli"],
      availabilityState: "available",
      capabilityTags: [],
      agentOrigin: "builtin",
    }]);
    renderOverview([loopRunFixture("running")]);
    expect(screen.getByRole("heading", { name: "Fixture Loop" })).toBeTruthy();
    expect(screen.getByText("Tests pass")).toBeTruthy();
    // Roles resolve to the registry display name; an id shows only while the registry has no entry.
    expect(await screen.findByText("Codex CLI")).toBeTruthy();
    // Read-only labels drop the editor's "one per line" guidance.
    expect(screen.queryByText("允许路径（每行一项）")).toBeNull();
    expect(screen.getByText("允许路径")).toBeTruthy();
    expect((screen.getByRole("button", { name: "检查并启动" }) as HTMLButtonElement).disabled).toBe(true);
    expect((screen.getByRole("button", { name: "删除" }) as HTMLButtonElement).disabled).toBe(true);
  });

  it("keeps direct start unavailable for a disabled definition", () => {
    const client = new QueryClient();
    render(<QueryClientProvider client={client}><LoopDefinitionOverview definition={loopDefinitionFixture({ enabled: false })} onDeleted={() => undefined} onEdit={() => undefined} onPreflight={() => undefined} runs={[]} /></QueryClientProvider>);
    expect(screen.getByText("已禁用")).toBeTruthy();
    expect((screen.getByRole("button", { name: "检查并启动" }) as HTMLButtonElement).disabled).toBe(true);
  });

  it("duplicates as a disabled distinctly named definition", async () => {
    const create = vi.spyOn(agentService, "createLoopDefinition").mockResolvedValue(loopDefinitionFixture({ id: "copy" }));
    renderOverview([]);
    const user = userEvent.setup();
    await user.click(screen.getByRole("button", { name: "创建副本" }));
    const name = screen.getByLabelText("名称");
    expect((name as HTMLInputElement).value).toBe("Fixture Loop 副本");
    await user.clear(name);
    await user.type(name, "Distinct copy");
    await user.click(screen.getByRole("button", { name: "确认" }));
    expect(create).toHaveBeenCalledWith(expect.objectContaining({ enabled: false, name: "Distinct copy", expectedVersion: null }));
  });

  it("requires confirmation before deleting", async () => {
    const remove = vi.spyOn(agentService, "deleteLoopDefinition").mockResolvedValue();
    renderOverview([]);
    const user = userEvent.setup();
    await user.click(screen.getByRole("button", { name: "删除" }));
    expect(remove).not.toHaveBeenCalled();
    await user.click(screen.getByRole("button", { name: "确认" }));
    expect(remove).toHaveBeenCalledWith("definition-1");
  });
});

// Task 17.14: distinct actions on one definition must not share a single combined pending flag
// unless they would genuinely race the same row (design.md Decision 11: a mutation only disables
// its own target action).
describe("LoopDefinitionOverview pending-state grouping", () => {
  afterEach(() => vi.restoreAllMocks());

  it("keeps Edit, Toggle, and Delete available while an unrelated Duplicate save is in flight", async () => {
    let resolveCreate: (definition: LoopDefinition) => void = () => undefined;
    vi.spyOn(agentService, "createLoopDefinition").mockReturnValue(
      new Promise<LoopDefinition>((resolve) => { resolveCreate = resolve; }),
    );
    renderOverview([]);
    const user = userEvent.setup();
    await user.click(screen.getByRole("button", { name: "创建副本" }));
    await user.clear(screen.getByLabelText("名称"));
    await user.type(screen.getByLabelText("名称"), "Distinct copy");
    await user.click(screen.getByRole("button", { name: "确认" }));

    // Duplicate's own request never resolved, so it is still pending -- but it only reads this
    // definition as a template for a brand-new row, so Edit/Toggle/Delete must stay available.
    await waitFor(() => expect(screen.getByText("正在应用定义变更…")).toBeTruthy());
    expect((screen.getByRole("button", { name: "编辑" }) as HTMLButtonElement).disabled).toBe(false);
    expect((screen.getByRole("button", { name: "禁用" }) as HTMLButtonElement).disabled).toBe(false);
    expect((screen.getByRole("button", { name: "删除" }) as HTMLButtonElement).disabled).toBe(false);

    resolveCreate(loopDefinitionFixture({ id: "copy" }));
    await waitFor(() => expect(screen.queryByRole("alertdialog")).toBeNull());
  });

  it("disables Edit and Delete while Toggle is in flight, since both act on this same row", async () => {
    let resolveUpdate: (definition: LoopDefinition) => void = () => undefined;
    vi.spyOn(agentService, "updateLoopDefinition").mockReturnValue(
      new Promise<LoopDefinition>((resolve) => { resolveUpdate = resolve; }),
    );
    renderOverview([]);
    const user = userEvent.setup();
    await user.click(screen.getByRole("button", { name: "禁用" }));

    await waitFor(() => expect((screen.getByRole("button", { name: "编辑" }) as HTMLButtonElement).disabled).toBe(true));
    expect((screen.getByRole("button", { name: "删除" }) as HTMLButtonElement).disabled).toBe(true);
    // Duplicate is unaffected by Toggle's own pending state -- independence holds in both directions.
    expect((screen.getByRole("button", { name: "创建副本" }) as HTMLButtonElement).disabled).toBe(false);

    resolveUpdate(loopDefinitionFixture({ enabled: false }));
    await waitFor(() => expect((screen.getByRole("button", { name: "编辑" }) as HTMLButtonElement).disabled).toBe(false));
  });
});

// Task 17.14: Toggle patches `loopQueryKeys.definitions` in place instead of `invalidateQueries` +
// a whole-collection refetch, which would swap every row's object identity and made an unrelated
// definition's row flicker/reload for this one row's own edit.
describe("LoopDefinitionOverview definitions cache patching", () => {
  afterEach(() => vi.restoreAllMocks());

  it("patches only this row after Toggle succeeds, leaving an unrelated row's identity untouched", async () => {
    const target = loopDefinitionFixture();
    const unrelatedRow = loopDefinitionFixture({ id: "definition-other", name: "Other" });
    const list = vi.spyOn(agentService, "listLoopDefinitions");
    vi.spyOn(agentService, "updateLoopDefinition").mockResolvedValue({ ...target, enabled: false, version: 2 });
    const client = new QueryClient({ defaultOptions: { mutations: { retry: false }, queries: { retry: false } } });
    client.setQueryData(loopQueryKeys.definitions, [target, unrelatedRow]);
    render(<QueryClientProvider client={client}><LoopDefinitionOverview definition={target} onDeleted={() => undefined} onEdit={() => undefined} onPreflight={() => undefined} runs={[]} /></QueryClientProvider>);
    const user = userEvent.setup();

    await user.click(screen.getByRole("button", { name: "禁用" }));
    await waitFor(() => {
      const patched = client.getQueryData<LoopDefinition[]>(loopQueryKeys.definitions);
      expect(patched?.find((item) => item.id === target.id)?.enabled).toBe(false);
    });

    const patched = client.getQueryData<LoopDefinition[]>(loopQueryKeys.definitions);
    expect(patched?.find((item) => item.id === "definition-other")).toBe(unrelatedRow);
    expect(list).not.toHaveBeenCalled();
  });
});

function renderOverview(runs: ReturnType<typeof loopRunFixture>[]) {
  const client = new QueryClient({ defaultOptions: { mutations: { retry: false }, queries: { retry: false } } });
  render(<QueryClientProvider client={client}><LoopDefinitionOverview definition={loopDefinitionFixture()} onDeleted={() => undefined} onEdit={() => undefined} onPreflight={() => undefined} runs={runs} /></QueryClientProvider>);
}
