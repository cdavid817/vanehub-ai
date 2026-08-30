import assert from "node:assert/strict";
import path from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";
import { analyzeFrontendSource, physicalLines, subtreeBudgetDiagnostics } from "./frontend-rules.mjs";

const projectRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../..");

test("accepts a React surface that uses a service contract", () => {
  const diagnostics = analyzeFrontendSource("src/components/safe.tsx", 'import { useAgentService } from "../services/agent-service-context";\nexport function Safe() { return useAgentService() ? null : null; }');
  assert.deepEqual(diagnostics, []);
});

for (const [name, source, id, line] of [
  ["Tauri import and invocation", 'import { invoke as nativeInvoke } from "@tauri-apps/api/core";\nexport function Unsafe() { nativeInvoke("run"); return null; }', "ARCH-FE-001", 1],
  ["native adapter import", 'import { tauriAgentClient } from "../services/tauri-agent-client";\nexport function Unsafe() { return tauriAgentClient ? null : null; }', "ARCH-FE-001", 1],
  ["runtime selector", 'import { isTauri } from "../services/runtime-mode";\nexport function Unsafe() { return isTauri ? null : null; }', "ARCH-FE-002", 1],
  ["native global", 'export function Unsafe() { return window.__TAURI_INTERNALS__ ? null : null; }', "ARCH-FE-002", 1],
  ["Zustand", 'import { create } from "zustand";\nexport const store = create(() => ({}));', "ARCH-REPO-001", 1],
  ["Redux Toolkit", 'import { configureStore } from "@reduxjs/toolkit";\nexport const store = configureStore({ reducer: {} });', "ARCH-REPO-001", 1],
  ["MobX", 'import { observable } from "mobx";\nexport const store = observable({});', "ARCH-REPO-001", 1],
]) {
  test(`rejects ${name} with an actionable location`, () => {
    const diagnostics = analyzeFrontendSource("src/components/unsafe.tsx", source);
    assert.ok(diagnostics.some((value) => value.includes(`[${id}] src/components/unsafe.tsx:${line}:`)));
    assert.ok(diagnostics.every((value) => value.includes("Repair:")));
  });
}

for (const [name, file, source] of [
  ["a feature service", "src/ui/inspector/inspector.tsx", 'import { agentService } from "../../services/agent-service";\nexport function Inspector() { return agentService ? null : null; }'],
  ["a Tauri API", "src/ui/inspector/inspector.ts", 'import { invoke } from "@tauri-apps/api/core";\nexport function run() { return invoke("x"); }'],
  ["a feature domain directory", "src/ui/status/status-badge.tsx", 'import { workspacePath } from "../../main-layout/workspace-route";\nexport function StatusBadge() { return workspacePath ? null : null; }'],
  ["src/features/", "src/ui/inspector/inspector.tsx", 'import { thing } from "../../features/sessions/thing";\nexport function Inspector() { return thing ? null : null; }'],
]) {
  test(`rejects a src/ui/ primitive importing ${name}`, () => {
    const diagnostics = analyzeFrontendSource(file, source);
    assert.ok(diagnostics.some((value) => value.includes("[ARCH-FE-005]")));
    assert.ok(diagnostics.every((value) => value.includes("Repair:")));
  });
}

test("accepts a src/ui/ primitive that only depends on other src/ui/ modules and npm packages", () => {
  const diagnostics = analyzeFrontendSource(
    "src/ui/inspector/inspector.tsx",
    'import { cn } from "../lib/utils";\nimport { StatusBadge } from "../status/status-badge";\nimport { Info } from "lucide-react";\nexport function Inspector() { return cn && StatusBadge && Info ? null : null; }',
  );
  assert.deepEqual(diagnostics, []);
});

for (const [name, source] of [
  ["an inline style prop", 'export function Badge() { return <span style={{ color: "red" }} />; }'],
  ["a literal hex color class", 'const badgeClass = "bg-[#ff0000] rounded-md";\nexport function Badge() { return <span className={badgeClass} />; }'],
  ["a literal rgb color class", 'export function Badge() { return <span className="text-[rgb(0,0,0)]" />; }'],
  ["a literal hsl color class in a cn() call", 'export function Badge() { return <span className={cn("rounded-md", active && "border-[hsl(0,0%,0%)]")} />; }'],
]) {
  test(`rejects a src/ui/ primitive using ${name}`, () => {
    const diagnostics = analyzeFrontendSource("src/ui/status/status-badge.tsx", source);
    assert.ok(diagnostics.some((value) => value.includes("[ARCH-FE-006]")));
    assert.ok(diagnostics.every((value) => value.includes("Repair:")));
  });
}

test("accepts a src/ui/ primitive using layout-only inline style (no token equivalent exists for a runtime pixel size)", () => {
  const diagnostics = analyzeFrontendSource(
    "src/ui/split-pane/SplitPane.tsx",
    'export function Pane({ size }) { return <div style={{ width: size, transform: `translateX(${size}px)` }} />; }',
  );
  assert.deepEqual(diagnostics, []);
});

test("rejects a src/ui/ primitive mixing a layout-only style with a color style property", () => {
  const diagnostics = analyzeFrontendSource(
    "src/ui/split-pane/SplitPane.tsx",
    'export function Pane({ size }) { return <div style={{ width: size, backgroundColor: "red" }} />; }',
  );
  assert.ok(diagnostics.some((value) => value.includes("[ARCH-FE-006]") && value.includes("backgroundColor")));
});

test("accepts a src/ui/ primitive using only semantic-token classes", () => {
  const diagnostics = analyzeFrontendSource(
    "src/ui/status/status-badge.tsx",
    'export function Badge() { return <span className={cn("bg-canvas text-attention border-[var(--border-subtle)]", "rounded-md")} />; }',
  );
  assert.deepEqual(diagnostics, []);
});

test("rejects a src/ui/ primitive using a Tailwind default-palette class", () => {
  const diagnostics = analyzeFrontendSource(
    "src/ui/status/status-badge.tsx",
    'export function Badge() { return <span className="text-red-500 dark:text-red-300" />; }',
  );
  assert.ok(diagnostics.some((value) => value.includes("[ARCH-FE-006]") && value.includes("text-red-500")));
});

test("accepts a color function wrapping a CSS variable, matching the existing button.tsx pattern", () => {
  const diagnostics = analyzeFrontendSource(
    "src/ui/status/status-badge.tsx",
    'export function Badge() { return <span className="bg-[hsl(var(--panel-glass))] shadow-[hsl(var(--shadow-color))]" />; }',
  );
  assert.deepEqual(diagnostics, []);
});

test("does not apply the non-semantic-color check outside src/ui/", () => {
  const diagnostics = analyzeFrontendSource(
    "src/main-layout/main-layout.tsx",
    'export function Legacy() { return <span style={{ color: "red" }} className="bg-[#ff0000]" />; }',
  );
  assert.deepEqual(diagnostics, []);
});

test("does not apply src/ui/ isolation to files outside src/ui/", () => {
  const diagnostics = analyzeFrontendSource(
    "src/main-layout/main-layout.tsx",
    'import { agentService } from "../services/agent-service";\nexport function MainLayout() { return agentService ? null : null; }',
  );
  assert.deepEqual(diagnostics, []);
});

test("does not treat test fixtures as production selection in the repository walker", () => {
  const diagnostics = analyzeFrontendSource("fixture.test.ts", 'import { create } from "zustand";', { requiresServiceBoundary: false });
  assert.equal(diagnostics.length, 1);
});

test("rejects a .ts hook (not just .tsx components) calling Tauri invoke directly", () => {
  const diagnostics = analyzeFrontendSource(
    "src/main-layout/use-something.ts",
    'import { invoke } from "@tauri-apps/api/core";\nexport function useSomething() { return invoke("x"); }',
  );
  assert.ok(diagnostics.some((value) => value.includes("[ARCH-FE-001] src/main-layout/use-something.ts:1:")));
});

test("accepts a real tauri-*-client.ts adapter calling Tauri invoke directly", () => {
  const diagnostics = analyzeFrontendSource(
    "src/services/tauri-example-client.ts",
    'import { invoke } from "@tauri-apps/api/core";\nexport const tauriExampleClient = { run: () => invoke("x") };',
  );
  assert.deepEqual(diagnostics, []);
});

test("counts physical lines the way wc -l does, with and without a trailing newline", () => {
  assert.equal(physicalLines("a\nb\nc\n"), 3);
  assert.equal(physicalLines("a\nb\nc"), 3);
  assert.equal(physicalLines(""), 0);
});

test("accepts a subtree within its recorded budget", () => {
  assert.deepEqual(subtreeBudgetDiagnostics(projectRoot, [{ root: "src/services", budget: 1_000_000, owner: "some-change" }]), []);
});

test("rejects a subtree over budget, naming the subtree rather than a file", () => {
  const [diagnostic, ...rest] = subtreeBudgetDiagnostics(projectRoot, [{ root: "src/services", budget: 1, owner: "split-web-agent-client" }]);
  assert.equal(rest.length, 0);
  assert.ok(diagnostic.startsWith("[ARCH-FE-004] src/services: "));
  assert.ok(diagnostic.includes("aggregate physical lines exceeds budget 1."));
  assert.ok(diagnostic.includes("Owner: split-web-agent-client."));
  assert.ok(diagnostic.includes("Repair:"));
  assert.ok(!diagnostic.includes(".ts"));
});
