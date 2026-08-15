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
  // Inherits the chip's own text colour instead of naming one: this marker sits on a
  // muted chip in the composer and on a primary-tinted chip in a user message, and a
  // fixed muted colour fails contrast on the second. The tint comes from a background
  // derived from the current colour, so the text itself keeps full contrast.
  return (
    <span
      className="shrink-0 rounded bg-current/15 px-1 tabular-nums"
      title={t("chat.fileReferenceLines", { range })}
    >
      L{range}
    </span>
  );
}
