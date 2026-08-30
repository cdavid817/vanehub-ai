export const RULES = Object.freeze({
  tauriBoundary: {
    id: "ARCH-FE-001",
    repair: "Move native access behind AgentService and the Tauri adapter.",
  },
  runtimeBranch: {
    id: "ARCH-FE-002",
    repair: "Move runtime selection behind the shared service adapter boundary.",
  },
  stateManagement: {
    id: "ARCH-REPO-001",
    repair: "Use React state, reducers, or context instead of an external state store.",
  },
  adapterParity: {
    id: "ARCH-FE-003",
    repair: "Type both runtime clients explicitly as AgentService.",
  },
  lineBudget: {
    id: "ARCH-FE-004",
    repair:
      "A split must move code, not duplicate it; raise the budget in the same commit only with a stated reason.",
  },
  uiPrimitiveIsolation: {
    id: "ARCH-FE-005",
    repair:
      "Keep src/ui/ primitives feature-agnostic: depend only on other src/ui/ modules or npm packages, and pass feature data in via props.",
  },
});

export function architectureDiagnostic(rule, file, line, message) {
  return `[${rule.id}] ${file}:${line}: ${message} Repair: ${rule.repair}`;
}

// 子树预算说的是整个目录,没有哪一行可指——硬塞个行号只会把读者引到无关的位置。
export function architectureSummaryDiagnostic(rule, subject, message) {
  return `[${rule.id}] ${subject}: ${message} Repair: ${rule.repair}`;
}
