import type { CuratorResult, SaveCuratorDraftInput } from "../types/skill-curator";
import { failure, type WebCuratorCandidate } from "./web-skill-curator-state";

export function webDraftText(input: SaveCuratorDraftInput): string {
  return input.mutation.kind === "learned_guidance"
    ? input.mutation.guidance
    : `${input.mutation.oldString}\n${input.mutation.newString}`;
}

export function validateWebDraft(
  candidate: WebCuratorCandidate,
  input: SaveCuratorDraftInput,
): CuratorResult<never> | undefined {
  if (
    (input.targetSkillId !== undefined && input.targetSkillId !== candidate.detail.targetSkillId)
    || (input.targetRevision !== undefined && input.targetRevision !== candidate.detail.targetRevision)
    || (input.overlayScope !== undefined && input.overlayScope !== candidate.detail.overlayScope)
  ) return failure("invalid_input", "target_override_prohibited", candidate, "target_override");
  const body = webDraftText(input);
  if (input.rationale.length > 2048 || body.length > (input.mutation.kind === "learned_guidance" ? 8192 : 16_384)) {
    return failure("invalid_input", "draft_size_limit", candidate, "size_limit");
  }
  if (/ignore previous|<script|```(?:bash|sh)|rm\s+-rf|permission expansion/i.test(body)) {
    return failure("unsafe_content", "unsafe_content", candidate, "unsafe_instruction");
  }
  if (input.mutation.kind === "exact_patch" && (!input.mutation.oldString || input.mutation.oldString === input.mutation.newString)) {
    return failure("invalid_input", "invalid_exact_patch", candidate, "exact_patch_invalid");
  }
  return undefined;
}
