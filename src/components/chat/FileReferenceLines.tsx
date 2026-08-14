import { useTranslation } from "react-i18next";
import { describeLineRange } from "../../services/composer-mention";
import type { ChatFileReference } from "../../types/chat";

/**
 * Compact `L10-50` marker on a reference chip, rendering nothing for a whole-file reference.
 * The notation is kept short because chip width is shared with the file name; the localized
 * wording lives in the tooltip.
 */
export function FileReferenceLines({ reference }: { reference: ChatFileReference }) {
  const { t } = useTranslation();
  const range = describeLineRange(reference);
  if (range === null) return null;
  return (
    <span className="shrink-0 tabular-nums text-muted-foreground" title={t("chat.fileReferenceLines", { range })}>
      L{range}
    </span>
  );
}
