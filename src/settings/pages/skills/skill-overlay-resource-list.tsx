import { FileStack, RotateCcw, SquarePen, Unplug } from "lucide-react";
import { useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { Badge } from "../../../components/ui/badge";
import { Button } from "../../../components/ui/button";
import type { SkillOverlayDetail, SkillOverlayResourceSummary, SkillOverlayTargetInput } from "../../../types/skill-overlay";
import { SkillOverlayResourceDialog } from "./skill-overlay-resource-dialog";
import { SKILL_OVERLAY_PINNED_DESCRIPTION_ID } from "./skill-overlay-pinned-notice";
import { formatBytes } from "./skill-overlay-resource-preview";
import { SkillOverlayResourceStateDialog, type ResourceStateAction } from "./skill-overlay-resource-state-dialog";

interface ResourceAction {
  kind: "replace" | ResourceStateAction;
  resource: SkillOverlayResourceSummary;
  trigger: HTMLElement | null;
}

export function SkillOverlayResourceList({ detail, target, onCommitted, onRefresh }: {
  detail: SkillOverlayDetail;
  target: SkillOverlayTargetInput;
  onCommitted: () => void;
  onRefresh: () => Promise<unknown> | void;
}) {
  const { t } = useTranslation();
  const [action, setAction] = useState<ResourceAction | null>(null);
  const sectionRef = useRef<HTMLElement>(null);
  if (detail.resources.length === 0) return null;

  function open(kind: ResourceAction["kind"], resource: SkillOverlayResourceSummary, trigger: HTMLElement) {
    setAction({ kind, resource, trigger });
  }

  return <section aria-labelledby="skill-overlay-resource-title" className="rounded-md border border-border bg-muted/10 p-3" ref={sectionRef}>
    <div className="flex flex-wrap items-start justify-between gap-2">
      <div>
        <h5 className="flex items-center gap-2 text-xs font-semibold" id="skill-overlay-resource-title"><FileStack className="h-4 w-4" />{t("skills.overlay.resource.title")}</h5>
        <p className="mt-1 text-[11px] leading-4 text-muted-foreground">{t("skills.overlay.resource.listDescription")}</p>
      </div>
      {detail.resourcesTruncated ? <Badge tone="warning">{t("skills.overlay.resource.listTruncated")}</Badge> : null}
    </div>
    <ul className="mt-3 space-y-2">
      {detail.resources.map((resource) => <li className="rounded-md border border-border bg-background p-3" key={resource.mutationId}>
        <div className="flex flex-col gap-3 sm:flex-row sm:items-start sm:justify-between">
          <div className="min-w-0">
            <p className="break-all font-mono text-xs font-semibold">{resource.logicalPath}</p>
            <div className="mt-2 flex flex-wrap gap-1.5">
              <Badge tone={resource.state === "active" ? "success" : "muted"}>{t(`skills.overlay.resource.state.${resource.state}`)}</Badge>
              <Badge tone="muted">{t(`skills.overlay.scope.${resource.effectiveScope}`)}</Badge>
              <Badge tone="muted">{resource.mediaType}</Badge>
              <Badge tone="muted">{formatBytes(resource.sizeBytes)}</Badge>
            </div>
          </div>
          <div className="flex flex-wrap gap-2">
            {resource.state === "active" ? <Button aria-describedby={detail.summary.pinned ? SKILL_OVERLAY_PINNED_DESCRIPTION_ID : undefined} disabled={detail.summary.pinned} onClick={(event) => open("replace", resource, event.currentTarget)} size="sm" variant="outline"><SquarePen />{t("skills.overlay.resource.replace")}</Button> : null}
            {resource.state === "active" ? <Button aria-describedby={detail.summary.pinned ? SKILL_OVERLAY_PINNED_DESCRIPTION_ID : undefined} disabled={detail.summary.pinned} onClick={(event) => open("disable", resource, event.currentTarget)} size="sm" variant="outline"><Unplug />{t("skills.overlay.resource.disable")}</Button> : null}
            {resource.state !== "reverted" ? <Button aria-describedby={detail.summary.pinned ? SKILL_OVERLAY_PINNED_DESCRIPTION_ID : undefined} disabled={detail.summary.pinned} onClick={(event) => open("revert", resource, event.currentTarget)} size="sm" variant="outline"><RotateCcw />{t("skills.overlay.resource.revert")}</Button> : null}
          </div>
        </div>
        <p className="mt-2 truncate font-mono text-[11px] text-muted-foreground" title={resource.contentHash}>{t("skills.overlay.resource.hash")}: {resource.contentHash}</p>
        {resource.shadowed.length > 0 ? <ShadowChain resource={resource} /> : <p className="mt-2 text-[11px] text-muted-foreground">{t("skills.overlay.resource.noShadowing")}</p>}
      </li>)}
    </ul>
    {action?.kind === "replace" ? <SkillOverlayResourceDialog detail={detail} initialPath={action.resource.logicalPath} onClose={() => setAction(null)} onCommitted={onCommitted} onRefresh={onRefresh} returnFocus={action.trigger} target={targetForResource(target, action.resource)} /> : null}
    {action && action.kind !== "replace" ? <SkillOverlayResourceStateDialog action={action.kind} detail={detail} onClose={() => setAction(null)} onCommitted={onCommitted} onRefresh={onRefresh} resource={action.resource} returnFocus={action.trigger} target={targetForResource(target, action.resource)} /> : null}
  </section>;
}

function ShadowChain({ resource }: { resource: SkillOverlayResourceSummary }) {
  const { t } = useTranslation();
  return <div className="mt-3 rounded-md border border-dashed border-border p-2.5 text-[11px]">
    <p className="font-semibold">{t("skills.overlay.resource.shadowChain", { count: resource.shadowed.length })}</p>
    <ul className="mt-1 space-y-1 text-muted-foreground">
      {resource.shadowed.map((shadow, index) => <li className="flex flex-wrap justify-between gap-2" key={`${shadow.contentHash}-${index}`}>
        <span>{shadow.scope ? t(`skills.overlay.scope.${shadow.scope}`) : t("skills.overlay.resource.baseLayer", { layer: shadow.baseLayer ? t(`skills.layer.${shadow.baseLayer}`) : "—" })}</span>
        <span className="font-mono">{shortHash(shadow.contentHash)}</span>
      </li>)}
    </ul>
    {resource.shadowedTruncated ? <p className="mt-1 text-warning">{t("skills.overlay.resource.shadowTruncated")}</p> : null}
  </div>;
}

function targetForResource(target: SkillOverlayTargetInput, resource: SkillOverlayResourceSummary): SkillOverlayTargetInput {
  return {
    skillId: target.skillId,
    scope: resource.effectiveScope,
    workspacePath: resource.effectiveScope === "project" ? target.workspacePath ?? null : null,
  };
}

function shortHash(value: string) {
  return value.length > 18 ? `${value.slice(0, 10)}…${value.slice(-6)}` : value;
}
