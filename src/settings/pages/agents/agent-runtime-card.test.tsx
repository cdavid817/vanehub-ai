// @vitest-environment jsdom

import { screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { mockAgents } from "../../../services/mock-agent-data";
import { renderWithAppProviders } from "../../../test/render";
import { AgentRuntimeCard } from "./agent-runtime-card";

describe("AgentRuntimeCard", () => {
  it("opens configuration management without selecting the runtime Agent", async () => {
    const agent = mockAgents.find((candidate) => candidate.id === "claude-code")!;
    const onManageConfigurations = vi.fn();
    const onSelect = vi.fn();
    const { user } = renderWithAppProviders(<AgentRuntimeCard active={false} activeMode={null} agent={agent} onDelete={vi.fn()} onEdit={vi.fn()} onManageConfigurations={onManageConfigurations} onSelect={onSelect} />);

    await user.click(screen.getByRole("button", { name: "管理全局配置" }));
    expect(onManageConfigurations).toHaveBeenCalledWith("claude-code");
    expect(onSelect).not.toHaveBeenCalled();
  });
});
