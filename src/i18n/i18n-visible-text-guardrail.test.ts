import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";

const checkedFiles = [
  "src/settings/pages/agents-page.tsx",
  "src/settings/pages/about-page.tsx",
  "src/settings/pages/usage-statistics-page.tsx",
  "src/settings/pages/cli-parameters-page.tsx",
  "src/settings/pages/sdk-page.tsx",
  "src/settings/pages/extensions-page.tsx",
  "src/settings/pages/mcp-page.tsx",
  "src/settings/pages/mcp/mcp-server-card.tsx",
  "src/settings/pages/mcp/mcp-server-form.tsx",
  "src/settings/pages/mcp/mcp-import-export.tsx",
  "src/main-layout/create-session-dialog.tsx",
  "src/main-layout/main-layout.tsx",
  "src/components/chat/MessageItem.tsx",
  "src/components/chat/selectors/ConfigSelect.tsx",
  "src/components/chat/selectors/ModeSelect.tsx",
  "src/components/chat/selectors/ModelSelect.tsx",
  "src/components/chat/selectors/ProviderSelect.tsx",
  "src/components/chat/selectors/ReasoningSelect.tsx",
  "src/session-workspace/session-tab-bar.tsx",
  "src/session-workspace/workspace-state.tsx",
  "src/session-workspace/files-tab.tsx",
  "src/session-workspace/documents-tab.tsx",
  "src/session-workspace/changes-tab.tsx",
  "src/session-workspace/terminal-tab.tsx",
  "src/session-workspace/shell-tab.tsx",
  "src/session-workspace/logs-tab.tsx",
  "src/session-workspace/report-tab.tsx",
];

const disallowedLiteralPatterns = [
  /["'`>]创建会话["'`<]/,
  /["'`>]项目文件夹["'`<]/,
  /["'`>]浏览["'`<]/,
  /["'`>]取消["'`<]/,
  /["'`>]Refresh["'`<]/,
  /["'`>]Refreshing["'`<]/,
  /["'`>]Agent Filter["'`<]/,
  /["'`>]Filter capability tag["'`<]/,
  /["'`>]SDK Dependencies["'`<]/,
  /["'`>]Current version["'`<]/,
  /["'`>]Operation Logs["'`<]/,
  /["'`>]MCP Servers["'`<]/,
  /["'`>]Import\/Export["'`<]/,
  /["'`>]Project Configuration["'`<]/,
  /["'`>]No visible MCP servers["'`<]/,
  /["'`>]Configure["'`<]/,
  /["'`>]Reasoning depth/,
];

describe("visible UI text i18n guardrail", () => {
  it("keeps fixed page/component copy in i18n resources", () => {
    for (const file of checkedFiles) {
      const source = readFileSync(file, "utf8");
      for (const pattern of disallowedLiteralPatterns) {
        expect(source, `${file} contains hard-coded visible text matching: ${pattern}`).not.toMatch(pattern);
      }
    }
  });
});
