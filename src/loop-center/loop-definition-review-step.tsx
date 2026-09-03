import { useTranslation } from "react-i18next";
import { FormSection } from "../ui/forms/FormSection";
import type { AgentRegistryEntry } from "../types/agent";
import type { LoopDefinitionDraft } from "./loop-definition-form";

export function ReviewStep({ agents, draft }: { agents: AgentRegistryEntry[]; draft: LoopDefinitionDraft }) {
  const { t } = useTranslation();
  const name = (id: string) => agents.find((agent) => agent.id === id)?.displayName ?? id;
  const commands = draft.verificationCommands.map((command) => `${command.program} ${command.arguments.split(/\r?\n/).filter(Boolean).join(" ")}`.trim()).join("; ");
  const rows = [
    ["name", draft.name], ["enabled", t(draft.enabled ? "loops.definition.enabled" : "loops.definition.disabled")],
    ["project", draft.projectPath], ["branch", draft.baseBranch], ["goal", draft.goal], ["acceptance", draft.acceptanceCriteria],
    ["allowedPaths", draft.allowedPaths], ["protectedPaths", draft.protectedPaths], ["worker", name(draft.workerAgentId)],
    ["verifier", name(draft.verifierAgentId)], ["commands", commands], ["maxIterations", String(draft.limits.maxIterations)],
    ["stepTimeoutSeconds", String(draft.limits.stepTimeoutSeconds)], ["totalTimeoutSeconds", String(draft.limits.totalTimeoutSeconds)],
    ["maxConsecutiveRuntimeErrors", String(draft.limits.maxConsecutiveRuntimeErrors)], ["maxConsecutiveNoProgress", String(draft.limits.maxConsecutiveNoProgress)],
  ];
  return (
    <FormSection title={t("loops.editor.step.review")}>
      <div className="grid gap-4">
        <dl className="grid gap-x-6 gap-y-3 sm:grid-cols-[minmax(8rem,auto)_1fr]">{rows.map(([key, value]) => <div className="contents" key={key}><dt className="text-xs font-medium text-muted-foreground">{t(`loops.editor.field.${key}`)}</dt><dd className="wrap-break-word whitespace-pre-line text-sm">{value}</dd></div>)}</dl>
        <div className="border-t border-border pt-3 text-xs leading-5 text-muted-foreground"><p>{t("loops.editor.review.worktree")}</p><p className="mt-1 font-medium text-foreground">{t("loops.editor.review.humanGate")}</p></div>
      </div>
    </FormSection>
  );
}
