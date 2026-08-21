import type { Goal, GoalInput, GoalLink, GoalLinkTarget, GoalStatus } from "../contracts/goal";
import type { GoalService } from "./goal-service";
import { canTransitionTo, deriveProgress, probeTarget } from "./web-goal-progress";

interface StoredGoal {
  id: string;
  title: string;
  description: string;
  acceptanceNotes: string;
  status: GoalStatus;
  projectPath: string | null;
  createdAt: string;
  updatedAt: string;
  links: { targetKind: GoalLinkTarget; targetId: string }[];
}

const goals = new Map<string, StoredGoal>();
let sequence = 0;

const now = () => new Date().toISOString();
const nextId = () => `goal-web-${++sequence}`;

function requireTitle(title: string): string {
  const value = title.trim();
  if (!value) throw new Error("Goal title is required.");
  return value;
}

function requireGoal(goalId: string): StoredGoal {
  const goal = goals.get(goalId);
  if (!goal) throw new Error("The goal was not found.");
  return goal;
}

function normalizeProjectPath(projectPath: string | null | undefined): string | null {
  const value = projectPath?.trim();
  return value ? value : null;
}

function project(stored: StoredGoal): Goal {
  const links: GoalLink[] = stored.links.map((link) => ({
    targetKind: link.targetKind,
    targetId: link.targetId,
    progress: probeTarget(link.targetKind, link.targetId),
  }));
  const totals = deriveProgress(stored.status, links);

  return {
    id: stored.id,
    title: stored.title,
    description: stored.description,
    acceptanceNotes: stored.acceptanceNotes,
    status: stored.status,
    derivedStatus: totals.derivedStatus,
    projectPath: stored.projectPath,
    createdAt: stored.createdAt,
    updatedAt: stored.updatedAt,
    counted: totals.counted,
    terminal: totals.terminal,
    unresolvable: totals.unresolvable,
    links,
  };
}

function moveTo(goalId: string, next: GoalStatus): Goal {
  const goal = requireGoal(goalId);
  if (!canTransitionTo(goal.status, next)) {
    throw new Error(`A goal cannot move from "${goal.status}" to "${next}".`);
  }
  goal.status = next;
  goal.updatedAt = now();
  return project(goal);
}

export const webGoalClient: GoalService = {
  async listGoals() {
    return [...goals.values()]
      .sort((left, right) =>
        right.createdAt.localeCompare(left.createdAt) || right.id.localeCompare(left.id),
      )
      .map(project);
  },

  async getGoal(goalId) {
    return project(requireGoal(goalId));
  },

  async createGoal(input) {
    const timestamp = now();
    const stored: StoredGoal = {
      id: nextId(),
      title: requireTitle(input.title),
      description: input.description?.trim() ?? "",
      acceptanceNotes: input.acceptanceNotes?.trim() ?? "",
      status: "draft",
      projectPath: normalizeProjectPath(input.projectPath),
      createdAt: timestamp,
      updatedAt: timestamp,
      links: [],
    };
    goals.set(stored.id, stored);
    return project(stored);
  },

  async updateGoal(goalId, input: GoalInput) {
    const goal = requireGoal(goalId);
    goal.title = requireTitle(input.title);
    goal.description = input.description?.trim() ?? "";
    goal.acceptanceNotes = input.acceptanceNotes?.trim() ?? "";
    goal.projectPath = normalizeProjectPath(input.projectPath);
    goal.updatedAt = now();
    return project(goal);
  },

  async deleteGoal(goalId) {
    requireGoal(goalId);
    goals.delete(goalId);
  },

  async linkGoalTarget(goalId, targetKind, targetId) {
    const goal = requireGoal(goalId);
    if (targetKind === "plan") throw new Error("Plan targets are retired and cannot be linked.");
    const target = targetId.trim();
    if (!target) throw new Error("A link target id is required.");
    if (goal.links.some((link) => link.targetKind === targetKind && link.targetId === target)) {
      throw new Error(`This ${targetKind} is already linked to the goal.`);
    }
    goal.links.push({ targetKind, targetId: target });
    return project(goal);
  },

  async unlinkGoalTarget(goalId, targetKind, targetId) {
    const goal = requireGoal(goalId);
    goal.links = goal.links.filter(
      (link) => !(link.targetKind === targetKind && link.targetId === targetId),
    );
    return project(goal);
  },

  async activateGoal(goalId) {
    return moveTo(goalId, "active");
  },

  async acceptGoal(goalId) {
    const goal = requireGoal(goalId);
    if (project(goal).derivedStatus !== "awaiting_acceptance") {
      throw new Error("A goal can only be accepted while it is awaiting acceptance.");
    }
    return moveTo(goalId, "achieved");
  },

  async reopenGoal(goalId) {
    return moveTo(goalId, "active");
  },

  async abandonGoal(goalId) {
    return moveTo(goalId, "abandoned");
  },
};

/** Test seam: the mock is module-level state shared across a page session. */
export function resetWebGoalClient(): void {
  goals.clear();
  sequence = 0;
}
