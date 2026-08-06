import type { SaveExpertRoleInput } from "../types/expert-role";

const hexColor = /^#[0-9a-fA-F]{6}$/;

/**
 * Shared by the Tauri and Web adapters so both reject the same inputs. Returns every problem at
 * once rather than the first, because a role form has several required fields and fixing them one
 * round-trip at a time is needlessly slow.
 */
export function validateExpertRoleInput(input: SaveExpertRoleInput): string[] {
  const errors: string[] = [];
  if (!input.displayName.trim()) errors.push("displayName is required");
  // Published to other Agents as the basis for handoff decisions, so a blank one breaks routing.
  if (!input.responsibility.trim()) errors.push("responsibility is required");
  if (!input.instruction.trim()) errors.push("instruction is required");
  if (!hexColor.test(input.color)) errors.push("color must be a hex value");
  if (new Set(input.skillIds).size !== input.skillIds.length) errors.push("skillIds must not repeat");
  if (input.reviewPolicy.requireDifferentFamily && !input.reviewPolicy.peerReviewer) {
    errors.push("requireDifferentFamily needs peerReviewer");
  }
  return errors;
}
