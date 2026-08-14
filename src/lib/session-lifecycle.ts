import type { InteractionMode, SessionLifecycleState } from "../types/agent";

/**
 * The session list and the info panel used to carry independent label maps for the same
 * `lifecycleState`, so a `failed` session read as "needs input" in one place and "failed" in
 * the other. Both surfaces now resolve through here; add a state in one place or not at all.
 */
export function lifecycleLabelKey(state: SessionLifecycleState) {
  return `layout.lifecycle.${state}` as const;
}

/** Tone for the status dot/pill that accompanies a lifecycle label. */
export type LifecycleTone = "active" | "pending" | "danger" | "neutral";

const lifecycleTones: Record<SessionLifecycleState, LifecycleTone> = {
  idle: "neutral",
  starting: "pending",
  running: "active",
  failed: "danger",
  stopped: "neutral",
};

export function lifecycleTone(state: SessionLifecycleState): LifecycleTone {
  return lifecycleTones[state];
}

/** Status dot colour per tone. The dot used to be green for every state, including `failed`. */
export const lifecycleDotClass: Record<LifecycleTone, string> = {
  active: "bg-[hsl(var(--success))]",
  pending: "bg-[hsl(var(--warning))]",
  danger: "bg-[hsl(var(--danger))]",
  neutral: "bg-muted-foreground",
};

export function interactionModeLabelKey(mode: InteractionMode) {
  return `session.interactionMode.${mode}` as const;
}
