import { LockKeyhole } from "lucide-react";
import { useTranslation } from "react-i18next";

export const SKILL_OVERLAY_PINNED_DESCRIPTION_ID = "skill-overlay-pinned-description";

export function SkillOverlayPinnedNotice() {
  const { t } = useTranslation();
  return <aside className="rounded-md border border-border bg-muted/30 p-3" id={SKILL_OVERLAY_PINNED_DESCRIPTION_ID} role="note">
    <p className="flex items-center gap-2 text-sm font-semibold"><LockKeyhole className="h-4 w-4 text-primary" />{t("skills.overlay.pinnedReadOnlyTitle")}</p>
    <p className="mt-1 text-xs leading-5 text-muted-foreground">{t("skills.overlay.pinnedReadOnlyDescription")}</p>
  </aside>;
}
