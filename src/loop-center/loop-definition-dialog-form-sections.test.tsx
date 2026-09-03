// @vitest-environment jsdom

import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";
import "../i18n";
import { agentService } from "../services/runtime-agent-client";
import { loopDefinitionFixture } from "../test/loop-fixtures";
import { LoopDefinitionDialog } from "./loop-definition-dialog";

/**
 * Task 17.5: each step now wraps its fields in the shared `src/ui/forms/FormSection.tsx`
 * primitive instead of a hand-rolled layout. These tests look for FormSection's own real heading
 * markup -- an actual `<h3>` naming the step, which no earlier version of this dialog rendered
 * inside its content pane (only the wizard chrome's small progress-tab subtitle did) -- rather
 * than just checking text content, so a regression back to a bespoke div wouldn't slip through
 * the way a text-only assertion could.
 */
describe("LoopDefinitionDialog FormSection adoption", () => {
  afterEach(() => vi.restoreAllMocks());

  it("renders every step's fields under a real FormSection heading", async () => {
    const definition = loopDefinitionFixture();
    vi.spyOn(agentService, "listAgents").mockResolvedValue([]);
    vi.spyOn(agentService, "listLoopProjectChoices").mockResolvedValue([]);
    vi.spyOn(agentService, "listLoopBranches").mockResolvedValue([]);
    const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
    render(<QueryClientProvider client={client}><LoopDefinitionDialog definition={definition} onClose={() => undefined} onSaved={() => undefined} /></QueryClientProvider>);
    const user = userEvent.setup();

    expect(screen.getByRole("heading", { level: 3, name: "目标与范围" })).toBeTruthy();
    await user.click(screen.getByRole("button", { name: "下一步" }));
    expect(screen.getByRole("heading", { level: 3, name: "角色智能体" })).toBeTruthy();
    await user.click(screen.getByRole("button", { name: "下一步" }));
    expect(screen.getByRole("heading", { level: 3, name: "验证与限制" })).toBeTruthy();
    await user.click(screen.getByRole("button", { name: "下一步" }));
    expect(screen.getByRole("heading", { level: 3, name: "检查确认" })).toBeTruthy();
  });
});
