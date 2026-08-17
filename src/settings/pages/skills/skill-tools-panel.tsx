import { useQuery } from "@tanstack/react-query";
import { ShieldAlert, Wrench } from "lucide-react";
import { useTranslation } from "react-i18next";
import { Badge } from "../../../components/ui/badge";
import { Button } from "../../../components/ui/button";
import { agentService } from "../../../services/runtime-agent-client";
import type { Skill } from "../../../types/skill";
import type { SkillToolRevision } from "../../../types/skill-tools";
import { SkillToolActions } from "./skill-tool-actions";

export function SkillToolsPanel({ skill }: { skill: Skill }) {
  const { t } = useTranslation();
  const query = useQuery({
    queryKey: ["skill-tools", skill.id, skill.workspacePath ?? "", skill.contentHash],
    queryFn: () => agentService.listSkillTools({
      skillId: skill.id,
      scope: skill.workspacePath ? "workspace" : "global",
      workspacePath: skill.workspacePath,
    }),
    staleTime: 30_000,
  });

  if (query.isLoading) return <PanelStatus>{t("skills.tools.loading")}</PanelStatus>;
  if (query.isError) return <PanelStatus danger action={<Button onClick={() => void query.refetch()} size="sm" variant="outline">{t("featureLoad.retry")}</Button>}>{query.error.message}</PanelStatus>;
  if (!query.data?.length) return <PanelStatus><Wrench className="mx-auto mb-2 h-5 w-5" />{t("skills.tools.empty")}</PanelStatus>;

  return <div className="space-y-3" aria-label={t("skills.tools.inventory")}>
    {query.data.map((tool) => <ToolInventoryRow key={tool.revision} onRefresh={() => query.refetch()} tool={tool} />)}
  </div>;
}

function ToolInventoryRow({ onRefresh, tool }: { onRefresh: () => Promise<unknown>; tool: SkillToolRevision }) {
  const { t } = useTranslation();
  const recent = tool.diagnostics.at(-1);
  return <article className="min-w-0 rounded-lg border border-border bg-background p-3" data-tool-revision={tool.revision}>
    <div className="flex min-w-0 flex-wrap items-start justify-between gap-2">
      <div className="min-w-0"><h5 className="break-all font-semibold">{tool.toolId}</h5><p className="mt-1 break-all font-mono text-[11px] text-muted-foreground">{tool.canonicalId}</p></div>
      <div className="flex flex-wrap gap-1"><StateBadge label={t(`skills.tools.validation.${tool.validation}`)} state={tool.validation === "valid" ? "good" : tool.validation === "invalid" ? "danger" : "warning"} /><StateBadge label={t(tool.trusted ? "skills.tools.trusted" : "skills.tools.untrusted")} state={tool.trusted ? "good" : "warning"} /></div>
    </div>
    <dl className="mt-3 grid gap-2 sm:grid-cols-2">
      <Fact label={t("skills.tools.kind")} value={tool.implementationKind.toUpperCase()} />
      <Fact label={t("skills.tools.revision")} mono value={tool.revision} />
      <Fact label={t("skills.tools.sourceScope")} value={t(`skills.tools.scope.${tool.sourceScope}`)} />
      <Fact label={t("skills.tools.integrity")} mono value={shortHash(tool.implementationHash)} />
      <Fact label={t("skills.tools.capabilities")} value={capabilitySummary(tool)} />
      <Fact label={t("skills.tools.lifecycle")} value={lifecycle(tool, t)} />
      <Fact label={t("skills.tools.runtimeSupport")} value={t(`skills.tools.runtime.${tool.runtimeSupport}`)} />
      <Fact label={t("skills.tools.recentStatus")} value={recent ? `${recent.severity.toUpperCase()} · ${recent.code}` : t("skills.tools.noDiagnostics")} />
    </dl>
    {tool.quarantined && tool.quarantineReason ? <p className="mt-3 flex items-start gap-2 rounded-md border border-destructive/40 bg-destructive/10 p-2 text-xs text-destructive"><ShieldAlert className="h-4 w-4 shrink-0" />{tool.quarantineReason}</p> : null}
    <ToolDiagnostics tool={tool} />
    <SkillToolActions onRefresh={onRefresh} tool={tool} />
  </article>;
}

function Fact({ label, mono, value }: { label: string; mono?: boolean; value: string }) {
  return <div className="min-w-0 rounded-md bg-muted/40 p-2"><dt className="text-[11px] text-muted-foreground">{label}</dt><dd className={`mt-1 break-all text-xs font-medium ${mono ? "font-mono" : ""}`} title={value}>{value}</dd></div>;
}

function StateBadge({ label, state }: { label: string; state: "good" | "warning" | "danger" }) {
  return <Badge tone={state === "good" ? "success" : state === "danger" ? "danger" : "warning"}>{label}</Badge>;
}

function PanelStatus({ action, children, danger = false }: { action?: React.ReactNode; children: React.ReactNode; danger?: boolean }) {
  return <div className={`rounded-lg border border-dashed p-4 text-center text-xs ${danger ? "border-destructive/50 text-destructive" : "border-border text-muted-foreground"}`} role={danger ? "alert" : "status"}>{children}{action ? <div className="mt-3">{action}</div> : null}</div>;
}

function shortHash(value: string) { return value.length > 16 ? `${value.slice(0, 8)}…${value.slice(-8)}` : value; }

function capabilitySummary(tool: SkillToolRevision) {
  const diff = tool.capabilityDiff;
  if (!diff) return tool.capabilityDigest;
  return diff.changed ? `+${diff.added.length} / −${diff.removed.length} · ${shortHash(diff.currentDigest)}` : shortHash(diff.currentDigest);
}

function lifecycle(tool: SkillToolRevision, t: (key: string) => string) {
  if (tool.quarantined) return t("skills.tools.quarantined");
  if (!tool.enabled) return t("skills.tools.disabled");
  return t("skills.tools.enabled");
}

function ToolDiagnostics({ tool }: { tool: SkillToolRevision }) {
  const { t } = useTranslation();
  const limitBreached = tool.diagnostics.some((item) => /limit|budget|timeout|fuel|memory/i.test(item.code));
  return <section aria-labelledby={`tool-diagnostics-${tool.revision}`} className="mt-3 rounded-md border border-border p-2">
    <h6 className="text-xs font-semibold" id={`tool-diagnostics-${tool.revision}`}>{t("skills.tools.diagnostics")}</h6>
    <p className="mt-1 text-[11px] text-muted-foreground">{t(`skills.tools.enforcement.${tool.enforcementStrength}`)}</p>
    {limitBreached ? <p className="mt-2 rounded bg-destructive/10 p-2 text-xs text-destructive" role="note">{t("skills.tools.limitBreach")}</p> : null}
    {tool.diagnostics.length ? <ul className="mt-2 space-y-1">{tool.diagnostics.map((item, index) => <li className="rounded bg-muted/40 p-2 text-xs" key={`${item.code}-${index}`}><span className="font-semibold">{item.severity.toUpperCase()} · {item.code}</span><span className="mt-1 block break-words text-muted-foreground">{item.detail}</span></li>)}</ul> : <p className="mt-2 text-xs text-muted-foreground">{t("skills.tools.noDiagnostics")}</p>}
  </section>;
}
