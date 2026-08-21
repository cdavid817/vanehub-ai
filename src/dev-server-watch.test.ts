import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";

/**
 * The dev server's watch ignore list is load-bearing, and every entry on it was added after
 * something broke in a way that did not look like a watcher problem.
 *
 * Two failure modes, both already paid for:
 *
 * - Volume. A directory of build artifacts or nested worktrees pushes the watcher past the point
 *   where the app boots inside the e2e navigation timeout, and the run fails on a static shell --
 *   which reads as a deterministic UI regression.
 * - Handles. On Windows the watcher holds a directory handle on every directory it registers, and
 *   a held directory cannot be renamed. `openspec archive` renames the change directory, so
 *   watching `openspec/` makes archiving fail with EPERM for as long as any dev server is up.
 *   That one was mis-diagnosed twice as an OS restriction on the drive.
 *
 * Neither is visible from reading the config, and neither shows up in a test run, so the list has
 * no other guard. `openspec` was missing from it for the entire life of the archive workflow.
 */
describe("dev server watch ignore list", () => {
  const viteConfig = readFileSync(new URL("../vite.config.ts", import.meta.url), "utf8");

  it.each([
    ["**/src-tauri/**", "Rust sources and their build output"],
    ["**/target/**", "the workspace-root Cargo target directory"],
    ["**/.claude/**", "nested git worktrees, each with its own node_modules and target"],
    ["**/openspec/**", "the directory `openspec archive` has to rename"],
    ["**/.docs-build/**", "generated API documentation"],
    ["**/.docs-screenshots/**", "generated documentation screenshots"],
    ["**/.docs-target/**", "the documentation build's Cargo target directory"],
  ])("ignores %s (%s)", (pattern) => {
    expect(viteConfig).toContain(`"${pattern}"`);
  });

  it("keeps the ignore list ahead of what the app actually imports", () => {
    // A path on the list must not be one the app loads from, or the dev server stops reloading on
    // a real edit. `src/` and `public/` are the only roots it serves from.
    const ignored = viteConfig.match(/"\*\*\/[^"]+\/\*\*"/g) ?? [];
    expect(ignored.length).toBeGreaterThan(0);
    for (const pattern of ignored) {
      expect(pattern).not.toMatch(/\*\*\/(src|public)\//);
    }
  });
});
