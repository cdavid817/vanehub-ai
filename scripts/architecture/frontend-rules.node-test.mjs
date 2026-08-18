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

test("does not treat test fixtures as production selection in the repository walker", () => {
  const diagnostics = analyzeFrontendSource("fixture.test.ts", 'import { create } from "zustand";', { reactSurface: false });
  assert.equal(diagnostics.length, 1);
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
