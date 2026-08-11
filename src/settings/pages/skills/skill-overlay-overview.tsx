import { GitMerge, Layers3, LockKeyhole, ShieldCheck } from "lucide-react";
import { useTranslation } from "react-i18next";
import { Badge } from "../../../components/ui/badge";
import { Button } from "../../../components/ui/button";
import { Card } from "../../../components/ui/card";
import type {
  SkillOverlayDetail,
  SkillOverlayMutationKind,
  SkillOverlayStatus,
  SkillOverlayTargetInput,
} from "../../../types/skill-overlay";
import { SkillOverlayDiffView } from "./skill-overlay-diff-view";
import { SkillOverlayHistory } from "./skill-overlay-history";
import { SkillOverlayMutationActions } from "./skill-overlay-mutation-actions";
import { SkillOverlayPinnedNotice } from "./skill-overlay-pinned-notice";
import { SkillOverlayReconciliationAction } from "./skill-overlay-reconciliation";
import { SkillOverlayResourceList } from "./skill-overlay-resource-list";

interface SkillOverlayOverviewProps {
  detail?: SkillOverlayDetail;
  error?: string | null;
  loading: boolean;
  onRetry?: () => Promise<unknown> | void;
  onCommitted: () => void;
  onRefresh: () => Promise<unknown> | void;
  target: SkillOverlayTargetInput;
}

const statusTones: Record<SkillOverlayStatus, "success" | "warning" | "danger" | "muted"> = {
  none: "muted",
  healthy: "success",
  untrusted: "warning",
  needsReconciliation: "warning",
  blocked: "danger",
  integrityFailure: "danger",
};

const mutationKinds: SkillOverlayMutationKind[] = ["patch", "learnedGuidance", "supportingFile"];

export function SkillOverlayOverview({ detail, error, loading, onRetry, onCommitted, onRefresh, target }: SkillOverlayOverviewProps) {
  const { t } = useTranslation();

  return <section aria-labelledby="skill-detail-overlay">
    <div className="flex flex-wrap items-center justify-between gap-2">
      <div>
        <h4 className="flex items-center gap-2 text-sm font-semibold" id="skill-detail-overlay">
          <Layers3 className="h-4 w-4 text-primary" />
          {t("skills.overlay.title")}
        </h4>
        <p className="mt-1 text-xs leading-5 text-muted-foreground">{t("skills.overlay.description")}</p>
      </div>
      {detail ? <OverlayBadges detail={detail} /> : null}
    </div>

    {loading ? <p className="mt-3 text-xs text-muted-foreground" role="status">{t("skills.overlay.loading")}</p> : null}
    {!loading && error ? <div className="mt-3 rounded-md border border-destructive/40 bg-destructive/10 p-3 text-xs text-destructive" role="alert">
      <p>{t("skills.overlay.loadError")}</p>
      {onRetry ? <Button className="mt-2" onClick={onRetry} size="sm" variant="outline">{t("featureLoad.retry")}</Button> : null}
    </div> : null}
    {!loading && !error && detail ? <OverlayContent detail={detail} onCommitted={onCommitted} onRefresh={onRefresh} target={target} /> : null}
  </section>;
}

function OverlayBadges({ detail }: { detail: SkillOverlayDetail }) {
  const { t } = useTranslation();
  const { summary } = detail;
  const trust = summary.scopes.some((scope) => scope.trust === "untrusted") ? "untrusted" : "trusted";
  return <div className="flex flex-wrap gap-2" aria-label={t("skills.overlay.statusSummary")}>
    <Badge tone={statusTones[summary.status]}>{t(`skills.overlay.status.${summary.status}`)}</Badge>
    {summary.scopes.length > 0 ? <Badge tone={trust === "trusted" ? "success" : "warning"}>{t(`skills.overlay.trust.${trust}`)}</Badge> : null}
    {summary.pinned ? <Badge tone="muted"><LockKeyhole className="mr-1 h-3 w-3" />{t("skills.overlay.pinned")}</Badge> : null}
  </div>;
}

function OverlayContent({ detail, target, onCommitted, onRefresh }: {
  detail: SkillOverlayDetail;
  target: SkillOverlayTargetInput;
  onCommitted: () => void;
  onRefresh: () => Promise<unknown> | void;
}) {
  const { t } = useTranslation();
  const { summary } = detail;
  if (summary.status === "none") {
    return <Card className="mt-3 border-dashed p-3">
      <p className="flex items-start gap-2 text-xs leading-5 text-muted-foreground">
        <ShieldCheck className="mt-0.5 h-4 w-4 shrink-0 text-primary" />
        {t("skills.overlay.empty")}
      </p>
      <HashGrid detail={detail} />
      {summary.pinned ? <div className="mt-3"><SkillOverlayPinnedNotice /></div> : null}
      <div className="mt-3 border-t border-border pt-3">
        <SkillOverlayMutationActions detail={detail} onCommitted={onCommitted} onRefresh={onRefresh} target={target} />
      </div>
      <div className="mt-3">
        <SkillOverlayHistory detail={detail} onCommitted={onCommitted} onRefresh={onRefresh} target={target} />
      </div>
    </Card>;
  }

  const activeMutations = detail.mutations.filter((mutation) => mutation.state === "active");
  const activeConflicts = detail.conflicts.filter((conflict) => conflict.state === "active").length;
  const shadowedResources = detail.resources.reduce((count, resource) => count + resource.shadowed.length, 0);

  return <div className="mt-3 space-y-3">
    {summary.pinned ? <SkillOverlayPinnedNotice /> : null}
    <SkillOverlayMutationActions detail={detail} onCommitted={onCommitted} onRefresh={onRefresh} target={target} />
    <SkillOverlayReconciliationAction detail={detail} onCommitted={onCommitted} onRefresh={onRefresh} target={target} />
    <HashGrid detail={detail} />
    <SkillOverlayDiffView detail={detail} />
    <div className="grid gap-2 sm:grid-cols-3">
      {mutationKinds.map((kind) => <Metric
        key={kind}
        label={t(`skills.overlay.mutations.${kind}`)}
        value={activeMutations.filter((mutation) => mutation.kind === kind).length}
      />)}
      <Metric label={t("skills.overlay.conflicts")} value={activeConflicts} warning={activeConflicts > 0} />
      <Metric label={t("skills.overlay.resources")} value={detail.resources.length} />
      <Metric label={t("skills.overlay.shadowedResources")} value={shadowedResources} />
    </div>
    <ScopeList detail={detail} />
    <SkillOverlayResourceList detail={detail} onCommitted={onCommitted} onRefresh={onRefresh} target={target} />
    <SkillOverlayHistory detail={detail} onCommitted={onCommitted} onRefresh={onRefresh} target={target} />
  </div>;
}

function HashGrid({ detail }: { detail: SkillOverlayDetail }) {
  const { t } = useTranslation();
  const hashes = [
    [t("skills.overlay.baseInstructionHash"), detail.summary.baseInstructionHash],
    [t("skills.overlay.basePackageHash"), detail.summary.basePackageHash],
    [t("skills.overlay.effectiveHash"), detail.summary.effectiveHash],
  ];
  return <dl className="mt-3 grid gap-2 sm:grid-cols-2">
    {hashes.map(([label, value]) => <div className="min-w-0 rounded-md border border-border bg-background px-3 py-2" key={label}>
      <dt className="text-[11px] text-muted-foreground">{label}</dt>
      <dd className="mt-1 truncate font-mono text-xs" title={value}>{shortHash(value)}</dd>
    </div>)}
  </dl>;
}

function ScopeList({ detail }: { detail: SkillOverlayDetail }) {
  const { t } = useTranslation();
  return <div>
    <h5 className="text-xs font-semibold">{t("skills.overlay.activeScopes")}</h5>
    <ul className="mt-2 space-y-2">
      {detail.summary.scopes.map((scope) => <li className="rounded-md border border-border bg-muted/20 p-3" key={scope.scope}>
        <div className="flex flex-wrap items-center justify-between gap-2">
          <span className="font-medium">{t(`skills.overlay.scope.${scope.scope}`)}</span>
          <Badge tone={scope.status === "applied" ? "success" : scope.status === "integrityFailure" ? "danger" : "warning"}>{t(`skills.overlay.scopeStatus.${scope.status}`)}</Badge>
        </div>
        <p className="mt-1 text-xs text-muted-foreground">{t("skills.overlay.scopeSummary", { revision: scope.revision, mutations: scope.activeMutationCount, conflicts: scope.conflictCount })}</p>
        {scope.baseHashChanged ? <p className="mt-1 text-xs text-warning">{t("skills.overlay.baseChanged")}</p> : null}
      </li>)}
    </ul>
  </div>;
}

function Metric({ label, value, warning = false }: { label: string; value: number; warning?: boolean }) {
  return <Card className="p-3">
    <p className="text-[11px] text-muted-foreground">{label}</p>
    <p className={`mt-1 flex items-center gap-1 text-lg font-semibold tabular-nums ${warning ? "text-destructive" : ""}`}>
      {warning ? <GitMerge className="h-4 w-4" /> : null}{value}
    </p>
  </Card>;
}

function shortHash(value: string) {
  return value.length > 18 ? `${value.slice(0, 10)}…${value.slice(-6)}` : value;
}
