import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { renderToString } from "react-dom/server";
import { describe, expect, it } from "vitest";
import "../../i18n";
import { SkillsPage } from "./skills-page";

describe("SkillsPage", () => {
  it("renders only the explicit loading state before the overview is available", () => {
    const queryClient = new QueryClient();
    const html = renderToString(
      <QueryClientProvider client={queryClient}>
        <SkillsPage searchTerm="" />
      </QueryClientProvider>,
    );

    expect(html).toContain("Skill 加载中...");
    expect(html).not.toContain("没有匹配的 Skill。");
    expect(html).not.toContain("Agent 挂载路径");
  });

  it("renders the service-backed Skill management modules", () => {
    const queryClient = new QueryClient();
    queryClient.setQueryData(["skill-overview", { scope: "global", workspacePath: null }], {
      stats: { total: 1, enabled: 1, mounted: 1 },
      agents: [{ id: "codex-cli", displayName: "Codex CLI", kind: "cli" }],
      mountPaths: [{ agentId: "codex-cli", mountPath: ".codex/skills", isDefault: true }],
      apiAgentBindings: {},
      restoreCandidates: [],
      drift: { scope: "global", workspacePath: null, issues: [], driftHash: "clean" },
      skills: [
        {
          id: "tdd-discipline",
          scope: "global",
          workspacePath: null,
          source: "builtin",
          enabled: true,
          skillDir: "~/.vanehub/skills/tdd-discipline",
          skillMdPath: "~/.vanehub/skills/tdd-discipline/SKILL.md",
          contentHash: "hash",
          metadata: {
            id: "tdd-discipline",
            name: "TDD 开发纪律助手",
            description: "测试先行",
            category: "development",
            version: "1.0.0",
            triggers: ["TDD"],
          },
          boundAgentIds: ["codex-cli"],
          bindings: [],
          createdAt: "now",
          updatedAt: "now",
        },
      ],
    });
    const html = renderToString(
      <QueryClientProvider client={queryClient}>
        <SkillsPage searchTerm="" />
      </QueryClientProvider>,
    );

    expect(html).toContain("Skill 管理");
    expect(html).toContain("Agent 挂载路径");
    expect(html).toContain("TDD 开发纪律助手");
    expect(html).toContain(".codex/skills");
  });

  it("renders API agent binding controls when a registered agent is API-kind", () => {
    const queryClient = new QueryClient();
    queryClient.setQueryData(["skill-overview", { scope: "global", workspacePath: null }], {
      stats: { total: 1, enabled: 1, mounted: 0 },
      agents: [{ id: "my-api-agent", displayName: "My API Agent", kind: "api" }],
      mountPaths: [],
      apiAgentBindings: { "tdd-discipline": ["my-api-agent"] },
      restoreCandidates: [],
      drift: { scope: "global", workspacePath: null, issues: [], driftHash: "clean" },
      skills: [
        {
          id: "tdd-discipline",
          scope: "global",
          workspacePath: null,
          source: "builtin",
          enabled: true,
          skillDir: "~/.vanehub/skills/tdd-discipline",
          skillMdPath: "~/.vanehub/skills/tdd-discipline/SKILL.md",
          contentHash: "hash",
          metadata: {
            id: "tdd-discipline",
            name: "TDD 开发纪律助手",
            description: "测试先行",
            category: "development",
            version: "1.0.0",
            triggers: ["TDD"],
          },
          boundAgentIds: [],
          bindings: [],
          createdAt: "now",
          updatedAt: "now",
        },
      ],
    });
    const html = renderToString(
      <QueryClientProvider client={queryClient}>
        <SkillsPage searchTerm="" />
      </QueryClientProvider>,
    );

    expect(html).toContain("API 代理绑定");
    expect(html).toContain("My API Agent");
  });
});
