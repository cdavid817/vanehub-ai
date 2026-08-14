import { useState, type ReactNode } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { FileDiff, Network } from "lucide-react";
import { useTranslation } from "react-i18next";
import { Button } from "../components/ui/button";
import type { AgentService } from "../services/agent-service";
import { agentService as defaultAgentService } from "../services/runtime-agent-client";

export function DelegationPanel({
  defaultTargetRoot,
  service = defaultAgentService,
  sessionId,
}: {
  defaultTargetRoot: string;
  service?: AgentService;
  sessionId: string;
}) {
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  const [provider, setProvider] = useState<"claude_code" | "codex_cli">("claude_code");
  const [mode, setMode] = useState<"analyze" | "edit">("analyze");
  const [prompt, setPrompt] = useState("");
  const [selectedAttemptId, setSelectedAttemptId] = useState<string | null>(null);
  const [applyAcknowledged, setApplyAcknowledged] = useState(false);
  const [applyOperationId, setApplyOperationId] = useState<string | null>(null);
  const attempts = useQuery({
    queryKey: ["sessions", sessionId, "delegation-attempts"],
    queryFn: () => service.listDelegationAttempts(sessionId),
  });
  const report = useQuery({
    enabled: Boolean(selectedAttemptId),
    queryKey: ["delegation-attempts", selectedAttemptId, "report"],
    queryFn: () => service.getDelegationReport(selectedAttemptId ?? ""),
  });
  const changeSetId = report.data?.attempt.changeSetArtifactId ?? null;
  const review = useQuery({
    enabled: Boolean(changeSetId),
    queryKey: ["artifacts", changeSetId, "change-set-review"],
    queryFn: () => service.getChangeSetReview(changeSetId ?? ""),
  });
  const recovery = useQuery({
    enabled: Boolean(applyOperationId),
    queryKey: ["delegation-apply", applyOperationId, "recovery"],
    queryFn: () => service.getDelegationRecovery(applyOperationId ?? ""),
  });
  const start = useMutation({
    mutationFn: () => service.startDelegation({
      agentId: "onepiece",
      sessionId,
      provider,
      mode,
      prompt: prompt.trim(),
      artifactIds: [],
    }),
    onSuccess: (attempt) => {
      setPrompt("");
      setSelectedAttemptId(attempt.id);
      void queryClient.invalidateQueries({ queryKey: ["sessions", sessionId, "delegation-attempts"] });
    },
  });
  const apply = useMutation({
    mutationFn: async () => {
      if (!review.data) throw new Error("change_set_review_unavailable");
      return service.applyDelegationChanges({
        agentId: "onepiece",
        sessionId,
        artifactId: review.data.artifact.id,
        expectedContentHash: review.data.artifact.contentHash,
        expectedDiffHash: review.data.diffHash,
        repositoryIdentity: review.data.repositoryIdentity,
        baseCommit: review.data.baseCommit,
        acknowledgement: true,
      });
    },
    onSuccess: (operation) => {
      setApplyAcknowledged(false);
      setApplyOperationId(operation.id);
    },
  });
  const safeError = attempts.isError || report.isError || review.isError || start.isError || apply.isError || recovery.isError;

  return (
    <section aria-labelledby="delegation-panel-heading" className="mb-4 rounded-lg border border-border bg-background p-3">
      <h3 className="flex items-center gap-2 text-sm font-semibold" id="delegation-panel-heading">
        <Network aria-hidden="true" className="h-4 w-4 text-primary" />{t("sessionTabs.delegation.title")}
      </h3>
      <div className="mt-3 grid gap-2 md:grid-cols-3">
        <Field label={t("sessionTabs.delegation.provider")}>
          <select className="ucd-input h-9 w-full rounded-md px-2 text-sm" onChange={(event) => setProvider(event.target.value as typeof provider)} value={provider}>
            <option value="claude_code">Claude Code</option><option value="codex_cli">Codex CLI</option>
          </select>
        </Field>
        <Field label={t("sessionTabs.delegation.mode")}>
          <select className="ucd-input h-9 w-full rounded-md px-2 text-sm" onChange={(event) => setMode(event.target.value as typeof mode)} value={mode}>
            <option value="analyze">{t("sessionTabs.delegation.mode.analyze")}</option><option value="edit">{t("sessionTabs.delegation.mode.edit")}</option>
          </select>
        </Field>
        <Field label={t("sessionTabs.delegation.target")}>
          <input className="ucd-input h-9 w-full rounded-md px-2 text-sm" readOnly value={defaultTargetRoot} />
        </Field>
      </div>
      <label className="mt-2 block text-xs font-medium text-muted-foreground">
        {t("sessionTabs.delegation.prompt")}
        <textarea className="ucd-input mt-1 min-h-20 w-full rounded-md p-2 text-sm" maxLength={8_000} onChange={(event) => setPrompt(event.target.value)} value={prompt} />
      </label>
      <Button className="mt-2" disabled={!defaultTargetRoot.trim() || !prompt.trim() || start.isPending} onClick={() => start.mutate()} size="sm">
        {t("sessionTabs.delegation.start")}
      </Button>

      {attempts.data?.length ? (
        <div className="mt-4 grid gap-3 lg:grid-cols-[minmax(11rem,0.6fr)_minmax(0,1.4fr)]">
          <ul aria-label={t("sessionTabs.delegation.history")} className="space-y-1">
            {attempts.data.map((attempt) => (
              <li key={attempt.id}><button className="w-full rounded-md border border-border/70 p-2 text-left text-xs hover:bg-muted" onClick={() => setSelectedAttemptId(attempt.id)} type="button"><span className="block font-medium">{attempt.provider} · {t(`sessionTabs.delegation.mode.${attempt.mode}`)}</span><span className="text-muted-foreground">{t(`sessionTabs.tools.status.${attempt.status}`)}</span></button></li>
            ))}
          </ul>
          <DelegationEvidence applyAcknowledged={applyAcknowledged} applyPending={apply.isPending} onApply={() => apply.mutate()} onApplyAcknowledged={setApplyAcknowledged} report={report.data} review={review.data} />
        </div>
      ) : <p className="mt-3 text-xs text-muted-foreground">{t("sessionTabs.delegation.empty")}</p>}
      {recovery.data ? <p className="mt-3 rounded-md border border-border p-2 text-xs" role="status">{t(`sessionTabs.delegation.recovery.${recovery.data.state}`)}{recovery.data.capsuleReference ? ` · ${recovery.data.capsuleReference}` : ""}</p> : null}
      {safeError ? <p className="mt-3 text-xs text-destructive" role="alert">{t("sessionTabs.delegation.safeError")}</p> : null}
    </section>
  );
}

function DelegationEvidence({ applyAcknowledged, applyPending, onApply, onApplyAcknowledged, report, review }: {
  applyAcknowledged: boolean;
  applyPending: boolean;
  onApply: () => void;
  onApplyAcknowledged: (value: boolean) => void;
  report: Awaited<ReturnType<AgentService["getDelegationReport"]>> | undefined;
  review: Awaited<ReturnType<AgentService["getChangeSetReview"]>> | undefined;
}) {
  const { t } = useTranslation();
  if (!report) return <p className="text-xs text-muted-foreground">{t("sessionTabs.delegation.select")}</p>;
  return <article className="min-w-0 rounded-md border border-border/70 p-3"><h4 className="text-sm font-semibold">{report.summary}</h4><p className="mt-2 text-xs text-muted-foreground">{t("sessionTabs.delegation.hostEvidence")}</p><ul className="list-disc pl-5 text-xs">{report.hostEvidence.map((item) => <li key={item}>{item}</li>)}</ul>{report.warnings.length ? <p className="mt-2 text-xs text-destructive">{report.warnings.join(" · ")}</p> : null}{review ? <div className="mt-3"><dl className="grid gap-2 text-xs sm:grid-cols-2"><Metadata label={t("sessionTabs.delegation.repository")} value={review.repositoryIdentity} /><Metadata label={t("sessionTabs.delegation.base")} value={review.baseCommit} /><Metadata label={t("sessionTabs.delegation.contentHash")} value={review.artifact.contentHash} /><Metadata label={t("sessionTabs.delegation.diffHash")} value={review.diffHash} /></dl><pre className="mt-2 max-h-64 overflow-auto whitespace-pre-wrap rounded-md bg-muted p-2 text-xs">{review.diffText}</pre><label className="mt-2 flex gap-2 text-xs text-muted-foreground"><input checked={applyAcknowledged} onChange={(event) => onApplyAcknowledged(event.target.checked)} type="checkbox" />{t("sessionTabs.delegation.applyAcknowledgement")}</label><Button className="mt-2" disabled={!review.applyable || !applyAcknowledged || applyPending} onClick={onApply} size="sm"><FileDiff aria-hidden="true" className="h-3.5 w-3.5" />{t("sessionTabs.delegation.apply")}</Button></div> : null}</article>;
}

function Field({ children, label }: { children: ReactNode; label: string }) {
  return <label className="text-xs font-medium text-muted-foreground">{label}{children}</label>;
}

function Metadata({ label, value }: { label: string; value: string }) {
  return <div className="min-w-0"><dt className="text-muted-foreground">{label}</dt><dd className="break-all font-mono">{value}</dd></div>;
}
