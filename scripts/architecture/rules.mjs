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
});

export function architectureDiagnostic(rule, file, line, message) {
  return `[${rule.id}] ${file}:${line}: ${message} Repair: ${rule.repair}`;
}
