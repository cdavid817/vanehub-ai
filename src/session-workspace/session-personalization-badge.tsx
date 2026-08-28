import { useTranslation } from "react-i18next";
import { Badge } from "../components/ui/badge";
import type { SessionPersonalizationMode } from "../types/personalization";

/**
 * A standing statement of what this session does and does not keep.
 *
 * Only for the two restricted modes: a badge on every session would become furniture, and the one
 * that matters is the one saying this conversation is not being remembered the way the others are.
 * Persistent rather than a toast, because the fact is true for the whole session and a user
 * scrolling back has no way to re-read a message that has gone.
 */
export function SessionPersonalizationBadge({ mode }: { mode: SessionPersonalizationMode }) {
  const { t } = useTranslation();
  if (mode === "standard") return null;

  return (
    <Badge
      data-testid={`session-personalization-badge-${mode}`}
      title={t(`session.personalization.retention.${mode}`)}
      tone="warning"
    >
      {t(`personalization.preview.modeValue.${mode}`)}
    </Badge>
  );
}
