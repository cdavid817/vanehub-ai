// @vitest-environment jsdom

import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";
import "../i18n";
import { agentService } from "../services/runtime-agent-client";
import { loopDefinitionFixture, loopRunFixture } from "../test/loop-fixtures";
import { LoopDefinitionOverview } from "./loop-definition-overview";

describe("LoopDefinitionOverview", () => {
  afterEach(() => vi.restoreAllMocks());

  it("shows the full saved definition and guards actions while a run is active", () => {
    renderOverview([loopRunFixture("running")]);
    expect(screen.getByRole("heading", { name: "Fixture Loop" })).toBeTruthy();
    expect(screen.getByText("Tests pass")).toBeTruthy();
    expect(screen.getByText("codex-cli")).toBeTruthy();
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

function renderOverview(runs: ReturnType<typeof loopRunFixture>[]) {
  const client = new QueryClient({ defaultOptions: { mutations: { retry: false }, queries: { retry: false } } });
  render(<QueryClientProvider client={client}><LoopDefinitionOverview definition={loopDefinitionFixture()} onDeleted={() => undefined} onEdit={() => undefined} onPreflight={() => undefined} runs={runs} /></QueryClientProvider>);
}
