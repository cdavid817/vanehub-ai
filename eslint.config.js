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
  // 技术债预算清单——存量超限文件。每条是上限,不是豁免:关掉规则等于放任增长,
  // 上一版就是这么从 7479 行漂到 10339 行的,还留下一条指向已删除文件的死条目。
  // 下调预算不需要任何理由;上调必须在同一个 commit 里写明原因。
  // 禁止新增条目;文件降到 300 行以下后删除该条目,由全局 max-lines 接管。
  // 子树聚合预算在 scripts/architecture/ 里,防止"拆分"退化成复制粘贴。
  ...[
    ["src/services/web-agent-client.ts", 1332],
    ["src/services/tauri-agent-client.ts", 1213],
    ["src/types/agent.ts", 702],
    ["src/services/agent-service.ts", 338],
    ["src/main-layout/main-layout.tsx", 528],
    ["src/contracts/agent.ts", 504],
    ["src/settings/pages/sdk-page.tsx", 396],
    ["src/main-layout/create-session-dialog.tsx", 318],
  ].map(([file, max]) => ({
    files: [file],
    rules: {
      "max-lines": ["error", { max, skipBlankLines: false, skipComments: false }],
    },
  })),
);
