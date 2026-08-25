import { useTranslation } from "react-i18next";

import { ApplicationDialog } from "../ui/application-dialog";
import { Button } from "../ui/button";

/**
 * The recoverable surface for a result that will not fit the draft.
 *
 * It exists so that "too long to insert" never becomes "silently truncated" or "draft replaced".
 * The text stays selectable and copyable here, which is the one thing the user needs in order not
 * to lose work the engine already did.
 */
export function LocalMediaResultDialog({
  engine,
  text,
  onClose,
}: {
  engine: "ocr" | "stt";
  text: string;
  onClose: () => void;
}) {
  const { t } = useTranslation();

  return (
    <ApplicationDialog
      description={t("localMedia.overflow.description")}
      footer={
        <div className="flex items-center justify-end gap-2">
          <Button
            onClick={() => {
              // Clipboard access can be denied; the text is still selectable in the textarea, so a
              // failure here is not worth an error surface of its own.
              void navigator.clipboard?.writeText(text).catch(() => undefined);
            }}
            type="button"
            variant="outline"
          >
            {t("localMedia.overflow.copy")}
          </Button>
          <Button onClick={onClose} type="button">
            {t("localMedia.overflow.close")}
          </Button>
        </div>
      }
      onClose={onClose}
      title={t(
        engine === "ocr" ? "localMedia.overflow.ocrTitle" : "localMedia.overflow.sttTitle",
      )}
    >
      <textarea
        aria-label={t("localMedia.overflow.textLabel")}
        className="ucd-input min-h-64 w-full rounded-lg px-3 py-2 font-mono text-sm outline-hidden focus-visible:ring-2 focus-visible:ring-ring"
        data-dialog-autofocus
        data-testid="composer-media-overflow"
        readOnly
        value={text}
      />
    </ApplicationDialog>
  );
}
