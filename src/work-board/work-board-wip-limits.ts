import type { WorkItemStage } from "../types/work-board";
import { workItemStages } from "../types/work-board";

/**
 * 14.14: presentation-only, client-local per-stage soft limits. No WIP-limit or stage-
 * configuration concept exists anywhere in `WorkItem`, the Rust model, or either service client
 * (checked alongside 14.3's own confirmed-absent Agent field) -- so this is never sent across a
 * service boundary and never consulted by `use-work-board-actions.ts`'s move/create/update path.
 * `isOverWipLimit` is read only by column/group headers to render a visual badge; nothing here can
 * block a real mutation, which is the "clear distinction from enforced domain rules" the task's
 * own wording asks for.
 */
export type WorkBoardWipLimits = Partial<Record<WorkItemStage, number>>;

const STORAGE_KEY = "vanehub.work-board.wip-limits.v1";
const CURRENT_VERSION = 1;

interface StoredPayload {
  version: number;
  limits: WorkBoardWipLimits;
}

function isValidLimits(value: unknown): value is WorkBoardWipLimits {
  if (!value || typeof value !== "object") return false;
  return Object.entries(value as Record<string, unknown>).every(([stage, limit]) =>
    (workItemStages as readonly string[]).includes(stage) && typeof limit === "number" && limit > 0);
}

/**
 * Versioned the same way as `work-board-saved-views.ts` (see that file's own top-of-file comment):
 * a whole-payload version mismatch discards every stored limit rather than guessing at a
 * migration, and fails closed to `{}` (no limits configured, so nothing reads as "over limit")
 * instead of throwing -- a soft guideline is not something a parse error should be allowed to
 * crash the board over.
 */
export function readWorkBoardWipLimits(): WorkBoardWipLimits {
  if (typeof localStorage === "undefined") return {};
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) return {};
    const parsed = JSON.parse(raw) as Partial<StoredPayload>;
    if (parsed.version !== CURRENT_VERSION || !isValidLimits(parsed.limits)) return {};
    return parsed.limits;
  } catch {
    return {};
  }
}

export function writeWorkBoardWipLimits(limits: WorkBoardWipLimits): void {
  if (typeof localStorage === "undefined") return;
  const payload: StoredPayload = { version: CURRENT_VERSION, limits };
  localStorage.setItem(STORAGE_KEY, JSON.stringify(payload));
}

/** A limit of 0, a negative number, or an absent entry all mean "no limit" -- this is
 *  presentation-only, so there is no meaningful "block everything" state to represent. */
export function isOverWipLimit(count: number, limit: number | undefined): boolean {
  return typeof limit === "number" && limit > 0 && count > limit;
}
