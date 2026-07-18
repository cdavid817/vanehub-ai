import { readFileSync } from "node:fs";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { renderToString } from "react-dom/server";
import { describe, expect, it } from "vitest";
import type { PromptHook, PromptHookListResult } from "../../types/prompt-hook";
import { PromptHookDialogs } from "./prompt-hooks/prompt-hook-dialogs";
import { PromptHooksPage } from "./prompt-hooks-page";

describe("PromptHooksPage", () => {
  it("renders hook summaries without default content preview", () => {
    const queryClient = new QueryClient();
    const hooks: PromptHookListResult = {
      stats: { total: 1, enabled: 1, builtin: 1, user: 0 },
      hooks: [
        {
          id: "law-runtime-boundary",
          name: "Runtime Boundary",
          description: "Keep tool calls behind the service boundary.",
          category: "law",
          stage: "per-turn",
          order: 100,
          version: 1,
          source: "builtin",
          enabled: true,
          disableable: false,
          cliBindings: ["codex-cli"],
          governance: { safetyTier: "readonly", transparencyTier: "opt-in-view", governanceTier: "immutable" },
          templateBody: "Never render this by default.",
          createdAt: "2026-07-18T00:00:00.000Z",
          updatedAt: "2026-07-18T00:00:00.000Z",
        },
      ],
    };

    queryClient.setQueryData(["agents", "prompt-hooks"], [
      {
        id: "codex-cli",
        displayName: "Codex CLI",
        provider: "OpenAI",
        launch: { kind: "cli", command: "codex" },
        supportedInteractionModes: ["cli"],
        availabilityState: "available",
        capabilityTags: [],
      },
    ]);
    queryClient.setQueryData(["prompt-hooks"], hooks);
    queryClient.setQueryData(["prompt-hook-traces"], [
      {
        id: "trace-1",
        hookId: "navigation-project-hints",
        category: "navigation",
        stage: "per-turn",
        status: "skipped",
        reason: "not-bound",
        agentId: "codex-cli",
        createdAt: "2026-07-18T00:00:00.000Z",
      },
    ]);

    const html = renderToString(
      <QueryClientProvider client={queryClient}>
        <PromptHooksPage searchTerm="" />
      </QueryClientProvider>,
    );

    expect(html).toContain("law-runtime-boundary");
    expect(html).toContain("点击预览");
    expect(html).toContain("not-bound");
    expect(html).toContain("时间");
    expect(html).not.toContain("Never render this by default.");
  });

  it("renders delete confirmation and localized mutation errors", () => {
    const hook = sampleUserHook();
    const html = renderToString(
      <PromptHookDialogs
        error="Invalid Prompt Hook id"
        state={{ mode: "delete", hook, preview: null }}
        onClose={() => undefined}
        onCreate={() => undefined}
        onDelete={() => undefined}
        onUpdate={() => undefined}
      />,
    );

    expect(html).toContain("删除 Prompt Hook");
    expect(html).toContain("user-review-focus");
    expect(html).toContain("Hook ID 只能包含小写字母、数字和短横线。");
  });

  it("uses shared semantic styling for both settings themes", () => {
    const files = [
      "src/settings/pages/prompt-hooks-page.tsx",
      "src/settings/pages/prompt-hooks/prompt-hook-card-list.tsx",
      "src/settings/pages/prompt-hooks/prompt-hook-dialogs.tsx",
      "src/settings/pages/prompt-hooks/prompt-hook-filter-toolbar.tsx",
      "src/settings/pages/prompt-hooks/prompt-hook-stats-cards.tsx",
      "src/settings/pages/prompt-hooks/prompt-hook-trace-panel.tsx",
    ];
    const combined = files.map((file) => readFileSync(file, "utf8")).join("\n");

    expect(combined).not.toContain("data-theme");
    expect(combined).not.toMatch(/theme\s*===\s*["'](?:minimal|futuristic)/);
    expect(combined).not.toMatch(/\bstyle=\{/);
  });
});

function sampleUserHook(): PromptHook {
  return {
    id: "user-review-focus",
    name: "Review Focus",
    description: "Focus review output.",
    category: "dynamic",
    stage: "per-turn",
    order: 500,
    version: 1,
    source: "user",
    enabled: true,
    disableable: true,
    cliBindings: ["codex-cli"],
    governance: { safetyTier: "editable", transparencyTier: "opt-in-view", governanceTier: "human-gated" },
    templateBody: "Review {{sampleInput}}",
    createdAt: "2026-07-18T00:00:00.000Z",
    updatedAt: "2026-07-18T00:00:00.000Z",
  };
}
