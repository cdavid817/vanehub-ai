import { Diff, Minus, Plus } from "lucide-react";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { Badge } from "../../../components/ui/badge";
import type { SkillLayer } from "../../../types/skill";
import type { SkillOverlayDetail, SkillOverlayDiff, SkillOverlayScope } from "../../../types/skill-overlay";

type DiffSelection = "effective" | SkillOverlayScope;

export function SkillOverlayDiffView({ detail }: { detail: SkillOverlayDetail }) {
  const { t } = useTranslation();
  const [requestedSelection, setRequestedSelection] = useState<DiffSelection>("effective");
  const selectedScope = detail.scopeDiffs.find((scopeDiff) => scopeDiff.scope === requestedSelection);
  const selection: DiffSelection = requestedSelection === "effective" || selectedScope ? requestedSelection : "effective";
  const selected = selection === "effective" ? detail.diff : selectedScope?.diff ?? detail.diff;

  return <section aria-labelledby="skill-overlay-diff-title" className="rounded-md border border-border bg-muted/10 p-3">
    <div className="flex flex-wrap items-start justify-between gap-2">
      <div>
        <h5 className="flex items-center gap-2 text-xs font-semibold" id="skill-overlay-diff-title">
          <Diff className="h-4 w-4" />{t("skills.overlay.diff.title")}
        </h5>
        <p className="mt-1 text-[11px] leading-4 text-muted-foreground">{t("skills.overlay.diff.description")}</p>
      </div>
      <Badge tone="muted">{t("skills.overlay.diff.characterSummary", { added: selected.addedCharacters, removed: selected.removedCharacters })}</Badge>
    </div>

    <div aria-label={t("skills.overlay.diff.comparisonPicker")} className="mt-3 flex flex-wrap gap-2">
      <ComparisonButton active={selection === "effective"} onClick={() => setRequestedSelection("effective")}>{t("skills.overlay.diff.effective")}</ComparisonButton>
      {detail.scopeDiffs.map((scopeDiff) => <ComparisonButton active={selection === scopeDiff.scope} key={`${scopeDiff.scope}-${scopeDiff.revision}`} onClick={() => setRequestedSelection(scopeDiff.scope)}>
        {t(`skills.overlay.scope.${scopeDiff.scope}`)}
      </ComparisonButton>)}
    </div>

    <ComparisonLabel baseLayer={detail.summary.baseLayer} scope={selection === "effective" ? null : selection} />
    <SkillOverlayDiffContent diff={selected} />
    {detail.scopeDiffsTruncated || selected.hunksTruncated ? <p className="mt-2 text-xs text-warning" role="status">{t("skills.overlay.diff.truncated")}</p> : null}
  </section>;
}

function ComparisonButton({ active, children, onClick }: { active: boolean; children: string; onClick: () => void }) {
  return <button
    aria-pressed={active}
    className={`min-h-11 rounded-md border px-3 py-1.5 text-xs font-medium focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring sm:min-h-9 ${active ? "border-primary bg-primary text-primary-foreground" : "border-border bg-background text-muted-foreground hover:text-foreground"}`}
    onClick={onClick}
    type="button"
  >{children}</button>;
}

function ComparisonLabel({ baseLayer, scope }: { baseLayer: SkillLayer; scope: SkillOverlayScope | null }) {
  const { t } = useTranslation();
  const before = scope ? t("skills.overlay.diff.scopeInput") : t("skills.overlay.diff.basePackage", { layer: t(`skills.layer.${baseLayer}`) });
  const after = scope ? t(`skills.overlay.scope.${scope}`) : t("skills.overlay.diff.effectiveContent");
  return <p className="mt-3 text-xs font-medium">{before} <span aria-hidden className="mx-1 text-muted-foreground">→</span> {after}</p>;
}

export function SkillOverlayDiffContent({ diff }: { diff: SkillOverlayDiff }) {
  const { t } = useTranslation();
  if (diff.hunks.length === 0) return <p className="mt-3 rounded-md border border-dashed border-border p-3 text-xs text-muted-foreground">{t("skills.overlay.diff.noChanges")}</p>;
  return <div className="mt-2 space-y-3">
    {diff.hunks.map((hunk, index) => <article className="overflow-hidden rounded-md border border-border bg-background" key={`${hunk.label}-${index}`}>
      <DiffSide content={hunk.before.content} label={t("skills.overlay.diff.before")} marker="minus" truncated={hunk.before.truncated} />
      <DiffSide content={hunk.after.content} label={t("skills.overlay.diff.after")} marker="plus" truncated={hunk.after.truncated} />
    </article>)}
  </div>;
}

function DiffSide({ content, label, marker, truncated }: { content: string; label: string; marker: "minus" | "plus"; truncated: boolean }) {
  const { t } = useTranslation();
  const Icon = marker === "minus" ? Minus : Plus;
  return <div className={`border-b border-border p-3 last:border-b-0 ${marker === "minus" ? "bg-destructive/5" : "bg-primary/5"}`}>
    <p className="mb-2 flex items-center gap-1 text-[11px] font-semibold"><Icon className="h-3 w-3" />{label}</p>
    <pre className="max-h-56 overflow-auto whitespace-pre-wrap break-words font-mono text-xs leading-5">{content || t("skills.overlay.diff.emptyContent")}</pre>
    {truncated ? <p className="mt-2 text-[11px] text-warning">{t("skills.overlay.diff.contentTruncated")}</p> : null}
  </div>;
}
