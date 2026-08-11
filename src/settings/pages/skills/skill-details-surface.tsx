import { X } from "lucide-react";
import { useTranslation } from "react-i18next";
import { ApplicationDialog } from "../../../components/ui/application-dialog";
import { Button } from "../../../components/ui/button";
import type { Skill, SkillLoadOutcome } from "../../../types/skill";
import type { SkillOverlayDetail, SkillOverlayTargetInput } from "../../../types/skill-overlay";
import { SkillDetailBody } from "./skill-detail-body";

interface SkillDetailsSurfaceProps {
  skill: Skill;
  loadOutcome?: SkillLoadOutcome;
  loading: boolean;
  overlayDetail?: SkillOverlayDetail;
  overlayError?: string | null;
  overlayLoading: boolean;
  wide: boolean;
  returnFocus: HTMLElement | null;
  onClose: () => void;
  onRetryOverlay: () => Promise<unknown> | void;
  onOverlayCommitted: () => void;
  overlayTarget: SkillOverlayTargetInput;
}

export function SkillDetailsSurface(props: SkillDetailsSurfaceProps) {
  const { t } = useTranslation();
  const title = t("skills.details.title", { name: props.skill.metadata.name });
  if (!props.wide) {
    return <ApplicationDialog
      description={props.skill.metadata.description}
      maxWidth="max-w-xl"
      onClose={props.onClose}
      returnFocus={props.returnFocus}
      title={title}
    >
      <SkillDetailBody
        loadOutcome={props.loadOutcome}
        loading={props.loading}
        onRetryOverlay={props.onRetryOverlay}
        onOverlayCommitted={props.onOverlayCommitted}
        overlayDetail={props.overlayDetail}
        overlayError={props.overlayError}
        overlayLoading={props.overlayLoading}
        overlayTarget={props.overlayTarget}
        skill={props.skill}
      />
      <div className="mt-5 flex justify-end border-t border-border pt-4">
        <Button onClick={props.onClose} variant="outline">{t("skills.dialog.close")}</Button>
      </div>
    </ApplicationDialog>;
  }

  return <aside
    aria-label={title}
    className="ucd-panel sticky top-4 max-h-[calc(100vh-2rem)] min-w-0 overflow-y-auto rounded-lg p-4"
    data-testid="skill-detail-inspector"
  >
    <div className="mb-4 flex items-start justify-between gap-3 border-b border-border pb-4">
      <div className="min-w-0">
        <h3 className="break-words text-base font-semibold">{title}</h3>
        <p className="mt-1 text-xs leading-5 text-muted-foreground">{props.skill.metadata.description}</p>
      </div>
      <Button aria-label={t("skills.details.close")} onClick={() => closeWide(props)} size="icon" variant="ghost"><X /></Button>
    </div>
    <SkillDetailBody
      loadOutcome={props.loadOutcome}
      loading={props.loading}
      onRetryOverlay={props.onRetryOverlay}
      onOverlayCommitted={props.onOverlayCommitted}
      overlayDetail={props.overlayDetail}
      overlayError={props.overlayError}
      overlayLoading={props.overlayLoading}
      overlayTarget={props.overlayTarget}
      skill={props.skill}
    />
  </aside>;
}

function closeWide({ onClose, returnFocus }: SkillDetailsSurfaceProps) {
  onClose();
  queueMicrotask(() => returnFocus?.focus());
}
