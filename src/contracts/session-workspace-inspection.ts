import { z } from "zod";
import type { WorkspaceInspectionCapabilities } from "../types/session-workspace-inspection";

export const capabilityStateSchema = z.object({
  available: z.boolean(),
  reasonCode: z.string().optional(),
  remediation: z.string().optional(),
});

export const workspaceInspectionCapabilitiesSchema = z.object({
  provider: z.enum(["local", "ssh", "simulated"]),
  listFiles: capabilityStateSchema,
  readTextFiles: capabilityStateSchema,
  searchFiles: capabilityStateSchema,
  gitStatus: capabilityStateSchema,
  gitDiff: capabilityStateSchema,
  watchMode: z.enum(["native", "polling", "event-derived", "none"]),
});

export function parseWorkspaceInspectionCapabilities(value: unknown): WorkspaceInspectionCapabilities {
  return workspaceInspectionCapabilitiesSchema.parse(value);
}
