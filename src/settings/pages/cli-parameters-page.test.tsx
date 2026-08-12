import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { renderToString } from "react-dom/server";
import { describe, expect, it } from "vitest";
import "../../i18n";
import { managedCliAgentIds } from "../../types/agent";
import { createCliParameterProfile } from "../../services/cli-parameter-catalog";
import { CliParametersPage } from "./cli-parameters-page";

describe("CliParametersPage", () => {
  it("renders all managed CLIs and OnePiece in the shared settings order", () => {
    const queryClient = new QueryClient();
    queryClient.setQueryData(
      ["cli-parameter-profiles"],
      [...managedCliAgentIds].reverse().map((agentId) => createCliParameterProfile(agentId)),
    );

    const html = renderToString(
      <QueryClientProvider client={queryClient}>
        <CliParametersPage searchTerm="" />
      </QueryClientProvider>,
    );

    expect(html).toContain("CLI 参数管理");
    expect(html).toContain("Claude Code");
    expect(html).toContain("Codex CLI");
    expect(html).toContain("Gemini CLI");
    expect(html).toContain("OpenCode");
    expect(html).toContain("Antigravity CLI");
    expect(html).toContain("OnePiece");
    const positions = ["Claude Code", "Codex CLI", "OpenCode", "Antigravity CLI", "Gemini CLI", "OnePiece"]
      .map((label) => html.indexOf(`>${label}<`));
    expect(positions).toEqual([...positions].sort((left, right) => left - right));
    expect(html).toContain("安全参数预览");
    expect(html).not.toContain("prompt=");
  });
});
