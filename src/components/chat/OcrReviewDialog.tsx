import { useTranslation } from "react-i18next";

import type { OcrReviewState } from "../../session-workspace/local-media/local-media-composer-types";
import { ApplicationDialog } from "../ui/application-dialog";
import { Button } from "../ui/button";

/**
 * Editable review of recognized text, shown before anything reaches the draft.
 *
 * Append is the primary action but never the automatic one. OCR output is routinely imperfect --
 * a misread column, a stray header -- and inserting it directly would turn the composer into a
 * place where the user's first task is undoing an edit they did not make.
 */
export function OcrReviewDialog({
  review,
  onAppend,
  onCancel,
  onChange,
}: {
  review: OcrReviewState;
  onAppend: () => void;
  onCancel: () => void;
  onChange: (text: string) => void;
}) {
  const { t } = useTranslation();
  const empty = review.text.trim().length === 0;
  const { provenance, source, warnings } = review.result;

  return (
    <ApplicationDialog
      description={t("localMedia.review.description")}
      footer={
        <div className="flex items-center justify-end gap-2">
          <Button onClick={onCancel} type="button" variant="ghost">
            {t("localMedia.review.cancel")}
          </Button>
          {/* Copy is the escape hatch for text that belongs somewhere other than this composer.
              Offered only where the platform has a clipboard: a button that silently did nothing
              would be worse than its absence. */}
          {navigator.clipboard ? (
            <Button
              data-testid="composer-ocr-copy"
              disabled={empty}
              onClick={() => {
                // Access can still be denied at the prompt; the text stays selectable in the
                // textarea either way, so a failure here needs no error surface of its own.
                void navigator.clipboard.writeText(review.text).catch(() => undefined);
              }}
              type="button"
              variant="outline"
            >
              {t("localMedia.review.copy")}
            </Button>
          ) : null}
          <Button
            data-testid="composer-ocr-append"
            disabled={empty}
            onClick={onAppend}
            type="button"
          >
            {t("localMedia.review.append")}
          </Button>
        </div>
      }
      onClose={onCancel}
      title={t("localMedia.review.title")}
    >
      <div className="flex flex-col gap-3" data-testid="composer-ocr-review">
        <dl className="grid grid-cols-[auto_1fr] gap-x-4 gap-y-1 text-xs text-muted-foreground">
          <dt>{t("localMedia.review.source")}</dt>
          <dd className="truncate">{source.displayName}</dd>
          <dt>{t("localMedia.review.pages")}</dt>
          <dd>{t("localMedia.review.pageCount", { count: source.pageCount })}</dd>
          <dt>{t("localMedia.review.engine")}</dt>
          {/* The engine name and version are product identifiers, not translatable copy. */}
          <dd>
            {provenance.engine}
            {provenance.engineVersion ? ` ${provenance.engineVersion}` : ""}
          </dd>
        </dl>

        <p className="text-xs text-muted-foreground">{t("localMedia.review.localBadge")}</p>

        {warnings.length > 0 ? (
          <ul className="ucd-status-warning rounded-lg px-3 py-2 text-xs">
            {warnings.map((warning) => (
              <li key={`${warning.code}-${warning.pageNumber ?? "all"}`}>
                {t(warning.messageKey)}
              </li>
            ))}
          </ul>
        ) : null}

        {empty ? (
          <p className="ucd-status-warning rounded-lg px-3 py-2 text-xs" role="status">
            {t("localMedia.errors.noTextDetected")}
          </p>
        ) : null}

        <textarea
          aria-label={t("localMedia.review.textLabel")}
          className="ucd-input min-h-64 w-full rounded-lg px-3 py-2 font-mono text-sm outline-hidden focus-visible:ring-2 focus-visible:ring-ring"
          data-dialog-autofocus
          data-testid="composer-ocr-text"
          onChange={(event) => onChange(event.target.value)}
          value={review.text}
        />

        <p className="text-right text-xs text-muted-foreground">
          {t("localMedia.review.characterCount", { count: review.text.length })}
        </p>
      </div>
    </ApplicationDialog>
  );
}
