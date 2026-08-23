import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { renderToString } from "react-dom/server";
import { describe, expect, it } from "vitest";
import "../../i18n";
import { managedCliAgentIds, type ManagedCliAgentId } from "../../types/agent";
import {
  cliParameterCatalogVersion,
  defaultCliParameterSelections,
  editableCliParameterDefinitions,
} from "../../services/cli-parameter-registry";
import { renderCliParameterSegments } from "../../services/cli-parameter-renderer";
import type { CliParameterProfile } from "../../types/cli-parameter-profile";
import { CliParametersPage } from "./cli-parameters-page";

function profile(agentId: ManagedCliAgentId): CliParameterProfile {
  const definitions = editableCliParameterDefinitions(agentId);
  const selections = defaultCliParameterSelections(agentId);
  return {
    agentId,
    catalogVersion: cliParameterCatalogVersion,
    revision: 0,
    updatedAt: null,
    installation: { installed: false, runnable: false, conflict: false },
    fields: definitions.map((definition) => ({
      definition,
      support: { state: "supported" },
      optionSupport: {},
    })),
    selections,
    savedPreviews: {
      chat: renderCliParameterSegments(definitions, selections, "chat"),
      interactive: renderCliParameterSegments(definitions, selections, "interactive"),
    },
    diagnostics: [],
  };
}

function render(searchTerm: string) {
  const queryClient = new QueryClient();
  queryClient.setQueryData(
    ["cli-parameter-profiles"],
    [...managedCliAgentIds].reverse().map(profile),
  );
  return renderToString(
    <QueryClientProvider client={queryClient}>
      <CliParametersPage searchTerm={searchTerm} />
    </QueryClientProvider>,
  );
}

describe("CliParametersPage", () => {
  it("renders all managed CLIs and OnePiece in the shared settings order", () => {
    const html = render("");

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

  it("renders the registry's editable fields and never a policy-governed one", () => {
    const html = render("");

    // `--model` is user-editable; claude-code's `--permission-mode` is policy-governed and must not
    // reach the page even though it is in the same registry.
    expect(html).toContain("--model");
    expect(html).not.toContain("--permission-mode");
    expect(html).not.toContain("--dangerously-skip-permissions");
  });

  it("filters fields by the settings search term", () => {
    const filtered = render("zzz-no-such-parameter");

    expect(filtered).not.toContain("--model");
    expect(filtered).toContain("没有参数匹配当前搜索");
  });
});
