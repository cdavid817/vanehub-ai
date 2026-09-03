import eslint from "@eslint/js";
import reactHooks from "eslint-plugin-react-hooks";
import tseslint from "typescript-eslint";

// 1213 -> 1215:合并 `upgrade-session-workspace-evidence-console` 与
// `manage-language-server-installation` 后的实测值。两侧各自在这个文件里加了方法(前者的工作区
// 检查订阅、后者的 LSP 安装/卸载),各自也都在自己那一侧记过一次上限。数字按合并树实测,不是
// 两侧相加——相加会把共有的基线算两遍。
export const LEGACY_LINE_BUDGET_EXEMPTIONS = [
  ["src/services/tauri-agent-client.ts", 1215],
  ["src/types/agent.ts", 702],
  // 528 -> 536: 接入全局 Command Center(§6)——一行 import、一次 `useCommandCenterContext` 调用
  // (聚合 navigate/onOpenSettings 与三个面板 toggle handler 为 WorkbenchCommandContext)、一行条件
  // 渲染。上下文对象的构造本身已抽到 use-command-center-context.ts,这里只留调用点。
  // 536 -> 557: task 13.8/13.9,Projects 目的地的 Continue Session / New Session 需要跨到这里
  // 已有的 goToSessions 机制,而 CreateSessionDialog 是这里唯一持有的实例。新增的是一份短暂的
  // prefill state(`sessionPrefillWorkspace`,在 dialog 每次关闭/创建成功时清空,避免侧栏的普通
  // "New Session" 继承上一次从 Projects 发起的 prefill)、两个转发 goToSessions 的 handler、
  // 传给 ProjectsDestination 的三个 prop,以及 CreateSessionDialog 自己的一个新 prop。没有
  // 复制既有分支——两个 handler 都是 goToSessions 已有能力的直接调用。
  ["src/main-layout/main-layout.tsx", 557],
  ["src/contracts/agent.ts", 504],
  ["src/settings/pages/sdk-page.tsx", 396],
  // 318 -> 335: 会话创建需要选一个个性化模式，而这个选择又必须在没有工作区时被纠正——
  // 否则存储会拒绝一个用户看不见的控件造成的提交。新增的是 state、两个派生值
  // 与三个 prop，外加打开对话框时把模式复位——记住上一次的隐私选择等于替用户重做一个
  // 他没有再确认过的决定。没有任何逻辑是从别处复制来的。
  ["src/main-layout/create-session-dialog.tsx", 335],
];

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
      // Cargo's target directory moved to the workspace root under establish-cargo-workspace-
      // skeleton; it used to be covered implicitly by the "src-tauri" entry above. Build scripts
      // (tauri-build here) generate JS artifacts under target/**/build/**/out/, which ESLint would
      // otherwise lint as source.
      "target",
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
    // This one script also contains Playwright `page.evaluate()`/`addInitScript()` callbacks that
    // execute in a browser page, not in this file's own Node process — they need browser globals
    // the rest of scripts/**/*.mjs correctly has no reason to declare.
    files: ["scripts/ui-redesign/capture-baseline.mjs"],
    languageOptions: {
      globals: {
        window: "readonly",
        document: "readonly",
        performance: "readonly",
        EventTarget: "readonly",
        localStorage: "readonly",
        HTMLElement: "readonly",
        setTimeout: "readonly",
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
  // 导出给 scripts/architecture/frontend-rules.node-test.mjs 用,断言这份清单不会静默增长,
  // 也断言 redesign-unified-workbench-ui 新增的 src/ui/ 文件不会出现在这里。
  ...LEGACY_LINE_BUDGET_EXEMPTIONS.map(([file, max]) => ({
    files: [file],
    rules: {
      "max-lines": ["error", { max, skipBlankLines: false, skipComments: false }],
    },
  })),
);
