import { ChevronDown, ChevronRight, LoaderCircle, RefreshCw } from "lucide-react";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import type { AgentService } from "../../../services/agent-service";
import type { ContextEvidenceManifest } from "../../../types/context-engine";

export function ContextInspector({ service }: { service: AgentService }) {
  const { t } = useTranslation();
  const [open, setOpen] = useState(false);
  const [manifest, setManifest] = useState<ContextEvidenceManifest | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  function load() {
    setLoading(true);
    setError(null);
    void service.listContextEvidenceManifests({ cursor: null, limit: 1 })
      .then((page) => setManifest(page.items[0] ?? null))
      .catch((reason: unknown) => setError(reason instanceof Error ? reason.message : String(reason)))
      .finally(() => setLoading(false));
  }

  useEffect(() => {
    if (open && !manifest && !loading && !error) load();
    // Opening is the only automatic load trigger; retries are explicit.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [open]);

  return <div className="mt-4 border-t border-border/70 pt-4">
    <button aria-expanded={open} className="flex min-h-11 w-full items-center justify-between rounded-md px-2 text-left text-sm font-semibold hover:bg-muted focus-visible:outline-hidden focus-visible:ring-2 focus-visible:ring-ring" onClick={() => setOpen((value) => !value)} type="button">
      <span>{t("onepiece.contextInspector.title")}</span>
      {open ? <ChevronDown aria-hidden="true" className="h-4 w-4" /> : <ChevronRight aria-hidden="true" className="h-4 w-4" />}
    </button>
    {open ? <div className="mt-2 rounded-lg border border-border/70 bg-background/60 p-3" data-testid="context-inspector">
      {loading ? <p className="flex items-center gap-2 text-sm text-muted-foreground" role="status"><LoaderCircle className="h-4 w-4 animate-spin" />{t("onepiece.contextInspector.loading")}</p> : null}
      {error ? <div className="flex items-center justify-between gap-2 text-sm ucd-status-warning" role="alert"><span>{error}</span><button aria-label={t("onepiece.contextInspector.retry")} className="flex h-11 w-11 items-center justify-center rounded-md focus-visible:ring-2 focus-visible:ring-ring" onClick={load} type="button"><RefreshCw className="h-4 w-4" /></button></div> : null}
      {!loading && !error && !manifest ? <p className="text-sm text-muted-foreground">{t("onepiece.contextInspector.empty")}</p> : null}
      {manifest ? <>
        <dl className="grid grid-cols-2 gap-2 text-xs sm:grid-cols-4">
          <Metric label={t("onepiece.contextInspector.budget")} value={`${manifest.occupiedTokens} / ${manifest.evidenceBudget}`} />
          <Metric label={t("onepiece.contextInspector.duplicates")} value={String(manifest.duplicateTokensSaved)} />
          <Metric label={t("onepiece.contextInspector.collection")} value={manifest.collectionLatencyBucket} />
          <Metric label={t("onepiece.contextInspector.compaction")} value={manifest.compactionTriggered ? t("onepiece.contextInspector.yes") : t("onepiece.contextInspector.no")} />
        </dl>
        <h5 className="mt-3 text-xs font-semibold uppercase tracking-wide text-muted-foreground">{t("onepiece.contextInspector.selected")}</h5>
        <ul className="mt-2 space-y-2">{manifest.selected.map((item) => <li className="rounded-md border border-border/70 p-2 text-xs" key={item.id}><div className="flex flex-wrap justify-between gap-2"><span className="font-medium">{item.sourceRef}{item.startLine ? `:${item.startLine}-${item.endLine}` : ""}</span><span className="font-mono text-muted-foreground">{item.tokenEstimate}</span></div><p className="mt-1 text-muted-foreground">{item.sourceKind} · {item.reasonCodes.join(", ")}</p></li>)}</ul>
        <h5 className="mt-3 text-xs font-semibold uppercase tracking-wide text-muted-foreground">{t("onepiece.contextInspector.rejected")}</h5>
        <p className="mt-1 text-xs text-muted-foreground">{manifest.rejected.length ? manifest.rejected.map((item) => `${item.id}: ${item.reasonCode}`).join(" · ") : "—"}</p>
        <h5 className="mt-3 text-xs font-semibold uppercase tracking-wide text-muted-foreground">{t("onepiece.contextInspector.sources")}</h5>
        <p className="mt-1 text-xs text-muted-foreground">{Object.entries(manifest.sourceOutcomes).map(([source, outcome]) => `${source}: ${outcome}`).join(" · ")}</p>
      </> : null}
    </div> : null}
  </div>;
}

function Metric({ label, value }: { label: string; value: string }) {
  return <div className="rounded-md border border-border/70 p-2"><dt className="text-muted-foreground">{label}</dt><dd className="mt-1 break-words font-mono tabular-nums">{value}</dd></div>;
}
