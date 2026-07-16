import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { renderToString } from "react-dom/server";
import { describe, expect, it } from "vitest";
import "../../i18n";
import { SkillsPage } from "./skills-page";

describe("SkillsPage", () => {
  it("renders the service-backed Skill management modules", () => {
    const queryClient = new QueryClient();
    queryClient.setQueryData(["agents", "skills"], [
      {
        id: "codex-cli",
        displayName: "Codex CLI",
        provider: "OpenAI",
        launch: { kind: "cli", command: "codex" },
        supportedInteractionModes: ["cli"],
        availabilityState: "unknown",
        capabilityTags: ["cli"],
      },
    ]);
    queryClient.setQueryData(["skill-mount-paths"], [
      { agentId: "codex-cli", mountPath: ".codex/skills", isDefault: true },
    ]);
    queryClient.setQueryData(["skills", { scope: "global", workspacePath: null }], {
      stats: { total: 1, enabled: 1, mounted: 1 },
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
    queryClient.setQueryData(["skill-drift", { scope: "global", workspacePath: null }], {
      scope: "global",
      workspacePath: null,
      issues: [],
      driftHash: "clean",
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
});
