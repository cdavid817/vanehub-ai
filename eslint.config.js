import eslint from "@eslint/js";
import reactHooks from "eslint-plugin-react-hooks";
import tseslint from "typescript-eslint";

export default tseslint.config(
  {
    ignores: [
      ".agents",
      ".claude",
      ".codex",
      ".docs-build",
      ".docs-screenshots",
      ".docs-target",
      ".superpowers",
      ".vanehub",
      "coverage",
      "dist",
      "node_modules",
      "playwright-report",
      "src-tauri",
      "test-results",
    ],
  },
  eslint.configs.recommended,
  ...tseslint.configs.recommended,
  {
    files: ["scripts/**/*.mjs"],
    languageOptions: {
      globals: {
        console: "readonly",
        process: "readonly",
      },
    },
  },
  {
    files: ["**/*.{ts,tsx}"],
    plugins: {
      "react-hooks": reactHooks,
    },
    rules: {
      "react-hooks/rules-of-hooks": "error",
      "react-hooks/exhaustive-deps": "warn",
      "@typescript-eslint/ban-ts-comment": [
        "error",
        {
          "ts-check": false,
          "ts-expect-error": "allow-with-description",
          "ts-ignore": true,
          "ts-nocheck": true,
          minimumDescriptionLength: 8,
        },
      ],
      "@typescript-eslint/no-explicit-any": "error",
      "no-control-regex": "off",
      "max-lines": ["error", { max: 300, skipBlankLines: false, skipComments: false }],
    },
  },
  {
    // 测试文件豁免 max-lines:用例行数随覆盖线性增长,硬拆损害用例内聚
    files: ["**/*.test.{ts,tsx}", "tests/**/*.ts"],
    rules: { "max-lines": "off" },
  },
  {
    // 技术债豁免清单——存量超限文件(2026-08 盘点,行尾注为当时物理行数)。
    // 禁止向本清单新增文件;文件拆分到 300 行以下后必须移除且不得回加。
    // 拆分工作由独立 OpenSpec 变更跟踪(尤其 web-agent-client.ts)。
    files: [
      "src/services/web-agent-client.ts", // 4137
      "src/services/tauri-agent-client.ts", // 763
      "src/types/agent.ts", // 538
      "src/settings/pages/sdk-page.tsx", // 393
      "src/contracts/agent.ts", // 364
      "src/main-layout/main-layout.tsx", // 341
      "src/services/coordination-runtime.ts", // 330
      "src/services/agent-service.ts", // 307
      "src/main-layout/create-session-dialog.tsx", // 306
    ],
    rules: { "max-lines": "off" },
  },
);
