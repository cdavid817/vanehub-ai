import type { ExpertRole, SaveExpertRoleInput } from "../types/expert-role";
import type { ExpertRoleService } from "./session-organization-service";
import { builtinExpertRoles } from "../config/builtin-expert-roles";
import { validateExpertRoleInput } from "./expert-role-runtime";
import { nowIso } from "./web-mock-clock";

let webExpertRoles: ExpertRole[] = builtinExpertRoles.map((role) => structuredClone(role));
let nextExpertRoleId = 1;

/** `createSession` snapshots seats against the role catalogue, so it reads it through behavior
 * rather than importing the binding. */
export function listWebExpertRoles(): ExpertRole[] {
  return webExpertRoles;
}

export const webExpertRoleClient: ExpertRoleService = {
  async listExpertRoles(): Promise<ExpertRole[]> {
    return structuredClone(webExpertRoles);
  },

  async saveExpertRole(input: SaveExpertRoleInput): Promise<ExpertRole> {
    const errors = validateExpertRoleInput(input);
    if (errors.length > 0) throw new Error(errors.join("; "));
    const timestamp = nowIso();
    const existing = input.id ? webExpertRoles.find((role) => role.id === input.id) : undefined;
    if (input.id && !existing) throw new Error(`Expert role not found: ${input.id}`);
    // Built-in roles are read-only; the UI copies them into a user role instead of editing.
    if (existing?.origin === "builtin") throw new Error("Built-in expert roles cannot be edited.");
    const role: ExpertRole = {
      id: existing?.id ?? `web-expert-role-${nextExpertRoleId++}`,
      displayName: input.displayName.trim(),
      avatar: input.avatar,
      color: input.color,
      responsibility: input.responsibility.trim(),
      instruction: input.instruction,
      skillIds: [...input.skillIds],
      reviewPolicy: { ...input.reviewPolicy },
      preferredProviders: [...input.preferredProviders],
      origin: "user",
      createdAt: existing?.createdAt ?? timestamp,
      updatedAt: timestamp,
    };
    webExpertRoles = existing
      ? webExpertRoles.map((candidate) => (candidate.id === role.id ? role : candidate))
      : [...webExpertRoles, role];
    return structuredClone(role);
  },

  async deleteExpertRole(roleId: string): Promise<void> {
    const role = webExpertRoles.find((candidate) => candidate.id === roleId);
    if (!role) throw new Error(`Expert role not found: ${roleId}`);
    if (role.origin === "builtin") throw new Error("Built-in expert roles cannot be deleted.");
    webExpertRoles = webExpertRoles.filter((candidate) => candidate.id !== roleId);
  },
};
