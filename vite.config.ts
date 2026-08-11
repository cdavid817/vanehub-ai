/// <reference types="vitest" />

import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";

export default defineConfig({
  plugins: [tailwindcss(), react()],
  build: {
    manifest: true,
    rolldownOptions: {
      output: {
        codeSplitting: {
          groups: [
            {
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
    port: 5174,
    strictPort: true,
    warmup: {
      clientFiles: ["./src/main.tsx"],
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
