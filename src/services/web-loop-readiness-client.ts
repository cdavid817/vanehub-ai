import { mockAgents } from "./mock-agent-data";
import { nowIso } from "./web-mock-clock";
import { webKnownWorkspaceClient } from "./web-known-workspace-client";
import { findLoopDefinition, findLoopRun, listWebLoopDefinitions, listWebLoopRuns } from "./web-loop-state";
import { acknowledgeWebLoopAudit, prepareWebLoopAdmission, simulateLoopAssessment } from "./web-loop-scope";
import { i18n } from "../i18n";
import type {
  LoopBranchChoice,
  LoopProjectChoice,
  LoopReadinessCheck,
  LoopReadinessReport,
  LoopRun,
} from "../types/loop";
import type { LoopReadinessService } from "./loop-service";

export const activeLoopStatuses: LoopRun["status"][] = ["queued", "running", "paused", "awaiting-acceptance"];

function readinessCheck(
  code: LoopReadinessCheck["code"],
  category: LoopReadinessCheck["category"],
  passed: boolean,
  remediationTarget: LoopReadinessCheck["remediationTarget"],
  detail: string | null = null,
): LoopReadinessCheck {
  return { code, category, status: passed ? "passed" : "blocked", blocking: true, detail: passed ? null : detail, remediationTarget: passed ? null : remediationTarget };
}

type ReadinessService = LoopReadinessService;

export const webLoopReadinessClient: ReadinessService = {
  async listLoopProjectChoices() {
    const known = (await webKnownWorkspaceClient.listKnownProjects()).filter((project) => project.isGit);
    const paths = new Map<string, LoopProjectChoice>();
    paths.set("D:/example-workspace", { path: "D:/example-workspace", displayName: "example-workspace", available: true, simulated: true });
    for (const project of known) {
      paths.set(project.path, { path: project.path, displayName: project.displayName, available: true, simulated: true });
    }
    for (const definition of listWebLoopDefinitions()) {
      if (!paths.has(definition.projectPath)) {
        const displayName = definition.projectPath.split(/[\\/]/).at(-1) ?? definition.projectPath;
        paths.set(definition.projectPath, { path: definition.projectPath, displayName, available: false, simulated: true });
      }
    }
    return [...paths.values()];
  },
  async listLoopBranches(projectPath) {
    const saved = listWebLoopDefinitions().filter((definition) => definition.projectPath === projectPath).map((definition) => definition.baseBranch);
    const discovered = new Set(["main", "develop", "origin/main"]);
    return [...new Set([...discovered, ...saved])].map<LoopBranchChoice>((name) => ({
      name,
      kind: name.startsWith("origin/") ? "remote" : "local",
      available: discovered.has(name),
      simulated: true,
    }));
  },
  async checkLoopReadiness(definitionId): Promise<LoopReadinessReport> {
    const definition = findLoopDefinition(definitionId);
    const projects = await this.listLoopProjectChoices();
    const branches = await this.listLoopBranches(definition.projectPath);
    const hasActiveRun = listWebLoopRuns().some((run) => run.definitionId === definitionId && activeLoopStatuses.includes(run.status));
    const commandsValid = definition.verificationCommands.length > 0 && definition.verificationCommands.every((command) => Boolean(command.program.trim()) && command.timeoutSeconds > 0);
    const scopeSupported = definition.scopeState === "verified";
    const pathScopeValid = scopeSupported && definition.allowedPaths.length > 0 && !definition.allowedPaths.some((path) => definition.protectedPaths.includes(path));
    const assessment = simulateLoopAssessment(definition);
    const checks = [
      readinessCheck("definition-enabled", "definition", definition.enabled, "definition"),
      readinessCheck("project-available", "workspace", projects.some((project) => project.path === definition.projectPath && project.available), "project"),
      readinessCheck("branch-available", "workspace", branches.some((branch) => branch.name === definition.baseBranch && branch.available), "branch"),
      readinessCheck("worker-eligible", "agent", mockAgents.some((agent) => agent.id === definition.workerAgentId), "worker"),
      readinessCheck("verifier-eligible", "agent", mockAgents.some((agent) => agent.id === definition.verifierAgentId), "verifier"),
      readinessCheck("verification-valid", "verification", commandsValid, "verification"),
      readinessCheck("scope-version-supported", "definition", scopeSupported, "definition", i18n.t("loops.web.assessment.legacy")),
      readinessCheck("path-scope-valid", "verification", pathScopeValid, "definition"),
      readinessCheck("execution-coverage", "agent", assessment.satisfiesRequestedMode, "worker", assessment.blockers.join(" ")),
      readinessCheck("no-active-run", "runtime", !hasActiveRun, "runs"),
    ];
    return {
      definitionId,
      ready: checks.every((check) => check.status === "passed"),
      simulated: true,
      checks,
      checkedAt: nowIso(),
      requestedMode: definition.requestedMode,
      scopeState: definition.scopeState,
      assessment,
      acknowledgementRequired: assessment.acknowledgementRequired,
      definitionRevision: definition.version,
    };
  },
  async prepareLoopAdmission(input) {
    if (input.action === "start") {
      const definition = findLoopDefinition(input.definitionId ?? "");
      return prepareWebLoopAdmission(input, definition, definition.version, definition.id);
    }
    const run = findLoopRun(input.runId ?? "");
    return prepareWebLoopAdmission(input, run.definitionSnapshot, run.revision, run.id);
  },
  async acknowledgeLoopAudit(challengeId) {
    return acknowledgeWebLoopAudit(challengeId);
  },
};
