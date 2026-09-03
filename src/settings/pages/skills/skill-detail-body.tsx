import { CircleCheck, CircleSlash2, Info, LockKeyhole, TriangleAlert } from "lucide-react";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { Badge } from "../../../components/ui/badge";
import type { Skill, SkillLoadOutcome, SkillShadowSummary } from "../../../types/skill";
import type { SkillOverlayDetail, SkillOverlayTargetInput } from "../../../types/skill-overlay";
import { useTabList } from "../../../ui/runtime-panel/use-tab-list";
import { SkillOverlayOverview } from "./skill-overlay-overview";
import { SkillEvolutionEvidence } from "./skill-evolution-evidence";
import { SkillToolsPanel } from "./skill-tools-panel";

const skillDetailTabs = ["overview", "tools"] as const;
type SkillDetailTab = (typeof skillDetailTabs)[number];

export function SkillDetailBody({
  skill,
  loadOutcome,
  loading,
  overlayDetail,
  overlayError,
  overlayLoading = false,
  onRetryOverlay,
  overlayTarget,
  onOverlayCommitted,
}: {
  skill: Skill;
  loadOutcome?: SkillLoadOutcome;
  loading: boolean;
  overlayDetail?: SkillOverlayDetail;
  overlayError?: string | null;
  overlayLoading?: boolean;
  onRetryOverlay?: () => Promise<unknown> | void;
  overlayTarget: SkillOverlayTargetInput;
  onOverlayCommitted: () => void;
}) {
  const { t } = useTranslation();
  const defaults = skill.metadata.compatibilityDefaults;
  const compatibilityDefaulted = Boolean(defaults?.skillType || defaults?.delivery);
  const [tab, setTab] = useState<SkillDetailTab>("overview");
  const detailTabs = useTabList(skillDetailTabs.map((value) => ({ id: value })), tab, (id) => setTab(id as SkillDetailTab));

  return <div className="space-y-5 text-sm">
    <div aria-label={t("skills.details.tabs")} className="grid grid-cols-2 gap-1 rounded-md bg-muted p-1" onKeyDown={detailTabs.handleKeyDown} role="tablist">
      {skillDetailTabs.map((value) => <button aria-controls={`skill-detail-${value}-panel`} aria-selected={tab === value} className={`min-h-10 rounded px-3 py-2 text-sm ${tab === value ? "bg-background font-semibold shadow-xs" : "text-muted-foreground"}`} id={`skill-detail-${value}-tab`} key={value} onClick={() => setTab(value)} ref={detailTabs.registerTabRef(value)} role="tab" tabIndex={tab === value ? 0 : -1} type="button">{t(`skills.details.tab.${value}`)}</button>)}
    </div>
    {tab === "tools" ? <div aria-labelledby="skill-detail-tools-tab" id="skill-detail-tools-panel" role="tabpanel"><SkillToolsPanel skill={skill} /></div> : <div aria-labelledby="skill-detail-overview-tab" className="space-y-5" id="skill-detail-overview-panel" role="tabpanel">
    <section aria-labelledby="skill-detail-runtime">
      <h4 className="text-sm font-semibold" id="skill-detail-runtime">{t("skills.details.runtime")}</h4>
      <div className="mt-3 flex flex-wrap gap-2">
        <Badge tone={skill.enabled ? "success" : "muted"}>{t(skill.enabled ? "skills.globalStatus.enabled" : "skills.globalStatus.paused")}</Badge>
        <Badge tone="muted">{t(`skills.type.${skill.metadata.type ?? "role"}`)}</Badge>
        <Badge tone={skill.availability === "available" ? "success" : "warning"}>{t(`skills.availability.${skill.availability}`)}</Badge>
      </div>
      <dl className="mt-4 grid gap-3 sm:grid-cols-2">
        <Detail label={t("skills.details.effectiveLayer")} value={t(`skills.layer.${skill.layer}`)} />
        <Detail label={t("skills.details.source")} value={t(`skills.source.${skill.source}`)} />
        <Detail label={t("skills.details.delivery")} value={t(`skills.delivery.${skill.metadata.delivery ?? "eager"}`)} />
        <Detail label={t("skills.details.origin")} value={t(`skills.origin.${skill.origin}`)} />
        <Detail label={t("skills.details.trust")} value={t(`skills.trust.${skill.trust}`)} />
        <Detail label={t("skills.details.version")} value={`v${skill.metadata.version}`} />
      </dl>
    </section>

    <section aria-labelledby="skill-detail-usage">
      <h4 className="text-sm font-semibold" id="skill-detail-usage">{t("skills.details.usage")}</h4>
      <p className="mt-2 tabular-nums text-xs text-muted-foreground">
        {t("skills.usage.summary", { views: skill.usage.viewCount, uses: skill.usage.useCount })}
      </p>
    </section>

    <SkillEvolutionEvidence skill={skill} />

    <SkillOverlayOverview
      detail={overlayDetail}
      error={overlayError}
      loading={overlayLoading}
      onRetry={onRetryOverlay}
      onCommitted={onOverlayCommitted}
      onRefresh={onRetryOverlay ?? (() => undefined)}
      target={overlayTarget}
    />
    <ResourceSummary loadOutcome={loadOutcome} loading={loading} />
    <Guidance skill={skill} compatibilityDefaulted={compatibilityDefaulted} />
    <PrecedenceTimeline skill={skill} />
    </div>}
  </div>;
}

function ResourceSummary({ loadOutcome, loading }: { loadOutcome?: SkillLoadOutcome; loading: boolean }) {
  const { t } = useTranslation();
  if (loading) return <section aria-labelledby="skill-detail-resources"><h4 className="text-sm font-semibold" id="skill-detail-resources">{t("skills.details.resources")}</h4><p className="mt-2 text-xs text-muted-foreground" role="status">{t("skills.details.loadingResources")}</p></section>;
  if (!loadOutcome || loadOutcome.status === "refused") return <section aria-labelledby="skill-detail-resources"><h4 className="text-sm font-semibold" id="skill-detail-resources">{t("skills.details.resources")}</h4><p className="mt-2 text-xs leading-5 text-muted-foreground">{loadOutcome ? refusalMessage(loadOutcome, t) : t("skills.resources.unavailable", { reason: "unknown" })}</p></section>;
  const resources = loadOutcome.result.resources;
  const count = resources.scripts.length + resources.references.length + resources.templates.length + resources.assets.length;
  return <section aria-labelledby="skill-detail-resources">
    <h4 className="text-sm font-semibold" id="skill-detail-resources">{t("skills.details.resources")}</h4>
    <p className="mt-2 text-xs leading-5 text-muted-foreground">{t("skills.resources.summary", { count, scripts: resources.scripts.length, references: resources.references.length, templates: resources.templates.length, assets: resources.assets.length })}{resources.truncated ? ` ${t("skills.resources.truncated")}` : ""}</p>
  </section>;
}

function Guidance({ skill, compatibilityDefaulted }: { skill: Skill; compatibilityDefaulted: boolean }) {
  const { t } = useTranslation();
  const items = [
    skill.immutable ? { icon: LockKeyhole, text: t("skills.immutable.explanation") } : null,
    skill.metadata.type === "utility" && skill.delegationCapability?.supported ? { icon: CircleCheck, text: t("skills.utility.available") } : null,
    skill.metadata.type === "utility" && !skill.delegationCapability?.supported ? { icon: TriangleAlert, text: t("skills.utility.unavailable") } : null,
    compatibilityDefaulted ? { icon: Info, text: t("skills.compatibility.explanation") } : null,
    skill.origin === "migrated" ? { icon: Info, text: t("skills.migration.preservedOverride") } : null,
  ].filter((item): item is { icon: typeof Info; text: string } => item !== null);
  if (items.length === 0) return null;
  return <section aria-labelledby="skill-detail-guidance">
    <h4 className="text-sm font-semibold" id="skill-detail-guidance">{t("skills.details.guidance")}</h4>
    <div className="mt-2 space-y-2">{items.map(({ icon: Icon, text }) => <p className="flex items-start gap-2 rounded-md border border-border bg-muted/30 p-3 text-xs leading-5 text-muted-foreground" key={text}><Icon className="mt-0.5 h-4 w-4 shrink-0" />{text}</p>)}</div>
  </section>;
}

function PrecedenceTimeline({ skill }: { skill: Skill }) {
  const { t } = useTranslation();
  const definitions: Array<SkillShadowSummary & { state: "effective" | "shadowed" }> = [
    { layer: skill.layer, origin: skill.origin, version: skill.metadata.version, availability: skill.availability, state: "effective" },
    ...skill.shadowedDefinitions.map((definition) => ({ ...definition, state: "shadowed" as const })),
  ];
  return <section aria-labelledby="skill-detail-precedence">
    <div className="flex flex-wrap items-baseline justify-between gap-2">
      <h4 className="text-sm font-semibold" id="skill-detail-precedence">{t("skills.details.timeline")}</h4>
      <span className="text-[11px] text-muted-foreground">{t("skills.details.precedenceOrder")}</span>
    </div>
    <ol className="mt-3 space-y-0">{definitions.map((definition, index) => {
      const effective = definition.state === "effective";
      const StateIcon = effective ? CircleCheck : CircleSlash2;
      return <li className="relative flex gap-3 pb-4 last:pb-0" data-definition-state={definition.state} key={`${definition.state}-${definition.layer}-${definition.version}-${index}`}>
        {index < definitions.length - 1 ? <span aria-hidden className="absolute bottom-0 left-[9px] top-5 w-px bg-border" /> : null}
        <StateIcon className={`relative mt-0.5 h-5 w-5 shrink-0 ${effective ? "text-primary" : "text-muted-foreground"}`} />
        <div className="min-w-0 flex-1 rounded-md border border-border bg-background px-3 py-2">
          <div className="flex flex-wrap items-center justify-between gap-2"><span className="font-medium">{t(`skills.layer.${definition.layer}`)}</span><span className="text-xs font-medium">{t(`skills.details.${definition.state}`)}</span></div>
          <p className="mt-1 text-xs text-muted-foreground">{t(`skills.origin.${definition.origin}`)} · v{definition.version} · {t(`skills.availability.${definition.availability}`)}</p>
        </div>
      </li>;
    })}</ol>
    {skill.shadowedDefinitions.length > 0 ? <p className="mt-3 text-xs leading-5 text-muted-foreground">{t("skills.shadowed.explanation", { layer: t(`skills.layer.${skill.layer}`) })}</p> : null}
  </section>;
}

function Detail({ label, value }: { label: string; value: string }) {
  return <div className="min-w-0"><dt className="text-xs text-muted-foreground">{label}</dt><dd className="mt-1 break-words text-sm font-medium" title={value}>{value}</dd></div>;
}

function refusalMessage(outcome: Extract<SkillLoadOutcome, { status: "refused" }>, t: (key: string, options?: Record<string, unknown>) => string) {
  if (outcome.refusal.reason === "stale-revision") return t("skills.resources.stale");
  if (outcome.refusal.reason === "utility-not-loadable" || outcome.refusal.reason === "unsupported") return t("skills.resources.notLoadable");
  return t("skills.resources.unavailable", { reason: outcome.refusal.reason });
}
