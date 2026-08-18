/// <reference types="vitest" />

import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";

// The displayed version used to be hand-copied into src/services/about-service.ts, which
// check-version-sync.mjs does not cover — so it silently drifted to 0.1.0 while the project
// moved to 0.1.0-preview.1. Reading package.json here makes drift impossible.
const packageVersion = JSON.parse(
  readFileSync(fileURLToPath(new URL("./package.json", import.meta.url)), "utf8"),
).version;

export default defineConfig({
  define: {
    __APP_VERSION__: JSON.stringify(packageVersion),
  },
  plugins: [tailwindcss(), react()],
  build: {
    manifest: true,
    rolldownOptions: {
      output: {
        codeSplitting: {
          groups: [
            {
              // 这条规则匹配的是 package-lock.json 钉住的嵌套安装路径,而不是任何
              // package.json 约束能保证的位置——rehype-katex 要 ^0.16.0、mermaid 要
              // ^0.16.45,两者解析到同一个 0.16.47,是锁文件把副本留在了各自目录下。
              // 重新解析锁文件、或换成 pnpm 那种布局,这个路径就会移位、规则静默失配
              // (pnpm 已经这么坑过一次)。构建末尾的 lazy chunk 计数是兜底检测。
              name: "rich-markdown-katex",
              test: /node_modules[\\/]rehype-katex[\\/]node_modules[\\/]katex[\\/]/,
            },
          ],
        },
      },
    },
  },
  clearScreen: false,
  server: {
    host: "127.0.0.1",
    port: 5175,
    strictPort: true,
    warmup: {
      clientFiles: [
        "./src/main.tsx",
        "./src/settings/pages/basic-settings-page.tsx",
        "./src/settings/pages/agent-configurations-page.tsx",
      ],
    },
    watch: {
      ignored: [
        "**/src-tauri/**",
        "**/.docs-build/**",
        "**/.docs-screenshots/**",
        "**/.docs-target/**",
      ],
    },
  },
  test: {
    testTimeout: 15_000,
    exclude: [
      "node_modules/**",
      "dist/**",
      "src-tauri/**",
      "tests/docs/**",
      "tests/e2e/**",
      // Nested git worktrees under .claude ship their own node_modules and e2e
      // specs; keep the test runner from descending into those copies.
      "**/.claude/**",
    ],
    setupFiles: ["./src/test/setup.ts"],
    coverage: {
      provider: "v8",
      reportsDirectory: "./coverage/frontend",
      reporter: ["text-summary", "json-summary", "lcov", "html"],
      include: ["src/**/*.{ts,tsx}"],
      exclude: [
        "src/**/*.test.{ts,tsx}",
        "src/**/*.d.ts",
        "src/test/**",
      ],
    },
  },
});
