import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { readFileSync } from "node:fs";
import { renderToString } from "react-dom/server";
import { describe, expect, it } from "vitest";
import "../../i18n";
import { SkillsPage } from "./skills-page";

const systemRuntimeFields = {
  layer: "system",
  origin: "shipped",
  trust: "trusted",
  availability: "available",
  immutable: true,
  shadowedDefinitions: [],
  usage: { viewCount: 1, useCount: 2, lastViewedAt: null, lastUsedAt: null, revisionWitness: null },
} as const;

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
          ...systemRuntimeFields,
        },
      ],
    });
    const html = renderToString(
      <QueryClientProvider client={queryClient}>
        <SkillsPage searchTerm="" />
      </QueryClientProvider>,
    );

    expect(html).toContain("Skill 管理");
    expect(html).toContain("按 Agent 管理");
    expect(html).toContain("Codex CLI");
    expect(html).toContain("TDD 开发纪律助手");
    expect(html).toContain("Skill 清单摘要");
    expect(html).toContain("CLI 已绑定");
    expect(html).toContain("API 已绑定");
    expect(html).toContain("按 ID、名称、分类、触发词或来源搜索");
    expect(html).not.toContain("选择本地项目目录");
  });

  it("uses the settings top-bar query and matches localized source labels", () => {
    const queryClient = new QueryClient();
    queryClient.setQueryData(["skill-overview", { scope: "global", workspacePath: null }], {
      stats: { total: 1, enabled: 1, mounted: 0 },
      agents: [], mountPaths: [], apiAgentBindings: {}, restoreCandidates: [],
      drift: { scope: "global", workspacePath: null, issues: [], driftHash: "clean" },
      skills: [{
        id: "builtin-match", scope: "global", workspacePath: null, source: "builtin", enabled: true,
        skillDir: "builtin-match", skillMdPath: "builtin-match/SKILL.md", contentHash: "hash",
        metadata: { id: "builtin-match", name: "Localized source result", description: "fixture", category: "test", version: "1", triggers: [] },
        boundAgentIds: [], bindings: [], createdAt: "now", updatedAt: "now",
        ...systemRuntimeFields,
      }],
    });
    const html = renderToString(<QueryClientProvider client={queryClient}><SkillsPage searchTerm="内置" /></QueryClientProvider>);
    expect(html).toContain("Localized source result");
    expect(html).not.toContain("skills.filters.searchPlaceholder");
  });

  it("renders API agent binding controls when a registered agent is API-kind", () => {
    const queryClient = new QueryClient();
    queryClient.setQueryData(["skill-overview", { scope: "global", workspacePath: null }], {
      stats: { total: 1, enabled: 1, mounted: 0 },
      agents: [{ id: "my-api-agent", displayName: "My API Agent with an exceptionally long registered display label", kind: "api" }],
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
          ...systemRuntimeFields,
        },
      ],
    });
    const html = renderToString(
      <QueryClientProvider client={queryClient}>
        <SkillsPage searchTerm="" />
      </QueryClientProvider>,
    );

    expect(html).toContain("API Agent");
    expect(html).toContain("My API Agent");
  });

  it("uses semantic styles and the service boundary for both themes", () => {
    const files = [
      "./skills-page.tsx",
      "./skills/skill-card-list.tsx",
      "./skills/skill-agent-navigation.tsx",
    ].map((path) => readFileSync(new URL(path, import.meta.url), "utf8")).join("\n");
    expect(files).not.toMatch(/theme\s*===\s*["'](?:minimal|futuristic)/);
    expect(files).not.toContain("data-theme");
    expect(files).not.toContain("invoke(");
  });
});
