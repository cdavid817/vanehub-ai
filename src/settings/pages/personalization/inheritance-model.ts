import type {
  InstructionMergeMode,
  PersonalizationPolicy,
  PersonalizationPolicyRef,
} from "../../../types/personalization";

// What the layers under the one being edited currently say, and what the chosen merge mode will do
// to them.
//
// Read straight off the stored layers rather than recomputed: precedence is the native side's rule,
// and a second implementation here would drift from it the first time the rule changed. What this
// owns is *which* layers sit below a scope, which is the order the spec fixes:
// built-in defaults, global, Agent, workspace, workspace-Agent.

export interface InheritedLayer {
  scopeKind: PersonalizationPolicyRef["scopeKind"];
  scopeKey: string;
  revision: number;
  aboutUser: string;
  styleRules: string;
  mergeMode: InstructionMergeMode;
}

function find(
  policies: readonly PersonalizationPolicy[],
  scopeKind: PersonalizationPolicyRef["scopeKind"],
  scopeKey: string,
): InheritedLayer | null {
  const policy = policies.find(
    (candidate) => candidate.scopeKind === scopeKind && candidate.scopeKey === scopeKey,
  );
  if (!policy) return null;
  return {
    scopeKind: policy.scopeKind,
    scopeKey: policy.scopeKey,
    revision: policy.revision,
    aboutUser: policy.aboutUser,
    styleRules: policy.styleRules,
    mergeMode: policy.instructionMergeMode,
  };
}

/** The written layers under `scope`, lowest precedence first. Unwritten layers are simply absent. */
export function layersBelow(
  scope: PersonalizationPolicyRef,
  policies: readonly PersonalizationPolicy[],
): InheritedLayer[] {
  const below: (InheritedLayer | null)[] = [];
  if (scope.scopeKind !== "global") below.push(find(policies, "global", ""));
  if (scope.scopeKind === "workspace-agent") {
    below.push(find(policies, "agent", scope.agentId ?? ""));
    below.push(find(policies, "workspace", scope.workspaceKey ?? ""));
  }
  return below.filter((layer): layer is InheritedLayer => layer !== null);
}

/**
 * Whether an Agent layer also sits below this scope without being nameable here.
 *
 * True for a workspace layer: an Agent layer applies between global and workspace, but which one
 * depends on the Agent that runs. Listing none would read as "nothing else applies", and listing
 * every Agent's would claim they all do.
 */
export function agentLayerVaries(scope: PersonalizationPolicyRef): boolean {
  return scope.scopeKind === "workspace";
}

export type MergeOutcome = "appended" | "replaced" | "nothing" | "inherited";

/**
 * What saving would do, stated before the save rather than after.
 *
 * `inherit` with text typed is the case worth naming: the text is stored but contributes nothing
 * until the mode changes, and a user who typed a paragraph and saw no effect would reasonably
 * conclude the save failed.
 */
export function mergeOutcome(mode: InstructionMergeMode, hasText: boolean): MergeOutcome {
  if (mode === "disabled") return "nothing";
  if (mode === "inherit") return "inherited";
  if (!hasText) return "inherited";
  return mode === "replace" ? "replaced" : "appended";
}

export function hasInstructionText(values: { aboutUser: string; styleRules: string }): boolean {
  return values.aboutUser.trim().length > 0 || values.styleRules.trim().length > 0;
}
