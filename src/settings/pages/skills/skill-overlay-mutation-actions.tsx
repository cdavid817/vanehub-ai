import { BookOpenCheck, FileArchive, FilePlus2, Replace } from "lucide-react";
import { useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "../../../components/ui/button";
import type { SkillOverlayDetail, SkillOverlayTargetInput } from "../../../types/skill-overlay";
import { SkillOverlayMutationDialog, type OverlayDialogKind } from "./skill-overlay-mutation-dialog";
import { SKILL_OVERLAY_PINNED_DESCRIPTION_ID } from "./skill-overlay-pinned-notice";
import { SkillOverlayImportDialog } from "./skill-overlay-import-dialog";
import { SkillOverlayResourceDialog } from "./skill-overlay-resource-dialog";

export function SkillOverlayMutationActions({
  detail,
  target,
  onCommitted,
  onRefresh,
}: {
  detail: SkillOverlayDetail;
  target: SkillOverlayTargetInput;
  onCommitted: () => void;
  onRefresh: () => Promise<unknown> | void;
}) {
  const { t } = useTranslation();
  const [dialog, setDialog] = useState<OverlayDialogKind | "resource" | "import" | null>(null);
  const patchTrigger = useRef<HTMLButtonElement>(null);
  const guidanceTrigger = useRef<HTMLButtonElement>(null);
  const resourceTrigger = useRef<HTMLButtonElement>(null);
  const importTrigger = useRef<HTMLButtonElement>(null);
  const pinned = detail.summary.pinned;

  return <>
    <div className="flex flex-col gap-2 sm:flex-row sm:flex-wrap">
      <Button
        aria-describedby={pinned ? SKILL_OVERLAY_PINNED_DESCRIPTION_ID : undefined}
        className="min-h-11 sm:min-h-9"
        disabled={pinned}
        onClick={() => setDialog("patch")}
        ref={patchTrigger}
        variant="outline"
      >
        <Replace />{t("skills.overlay.actions.addPatch")}
      </Button>
      <Button
        aria-describedby={pinned ? SKILL_OVERLAY_PINNED_DESCRIPTION_ID : undefined}
        className="min-h-11 sm:min-h-9"
        disabled={pinned}
        onClick={() => setDialog("guidance")}
        ref={guidanceTrigger}
        variant="outline"
      >
        <BookOpenCheck />{t("skills.overlay.actions.addGuidance")}
      </Button>
      <Button
        aria-describedby={pinned ? SKILL_OVERLAY_PINNED_DESCRIPTION_ID : undefined}
        className="min-h-11 sm:min-h-9"
        disabled={pinned}
        onClick={() => setDialog("resource")}
        ref={resourceTrigger}
        variant="outline"
      >
        <FilePlus2 />{t("skills.overlay.actions.addResource")}
      </Button>
      <Button
        aria-describedby={pinned ? SKILL_OVERLAY_PINNED_DESCRIPTION_ID : undefined}
        className="min-h-11 sm:min-h-9"
        disabled={pinned}
        onClick={() => setDialog("import")}
        ref={importTrigger}
        variant="outline"
      >
        <FileArchive />{t("skills.overlay.actions.import")}
      </Button>
    </div>
    {dialog === "patch" || dialog === "guidance" ? <SkillOverlayMutationDialog
      detail={detail}
      kind={dialog}
      onClose={() => setDialog(null)}
      onCommitted={onCommitted}
      onRefresh={onRefresh}
      returnFocus={dialog === "patch" ? patchTrigger.current : guidanceTrigger.current}
      target={target}
    /> : null}
    {dialog === "resource" ? <SkillOverlayResourceDialog
      detail={detail}
      onClose={() => setDialog(null)}
      onCommitted={onCommitted}
      onRefresh={onRefresh}
      returnFocus={resourceTrigger.current}
      target={target}
    /> : null}
    {dialog === "import" ? <SkillOverlayImportDialog
      detail={detail}
      onClose={() => setDialog(null)}
      onCommitted={onCommitted}
      onRefresh={onRefresh}
      returnFocus={importTrigger.current}
      target={target}
    /> : null}
  </>;
}
