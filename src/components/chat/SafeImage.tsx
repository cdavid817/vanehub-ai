import { useState } from "react";
import { createPortal } from "react-dom";
import { ImageOff, Maximize2, X } from "lucide-react";
import { useTranslation } from "react-i18next";
import { ApplicationDialog } from "../ui/application-dialog";
import { cn } from "../../lib/utils";

const SAFE_DATA_IMAGE = /^data:image\/(?:avif|gif|jpeg|png|svg\+xml|webp)(?:;[^,]*)?,/i;

export function safeImageSource(value: string | undefined): string | null {
  if (!value) return null;
  const source = value.trim();
  if (!source) return null;
  if (SAFE_DATA_IMAGE.test(source)) return source;
  if (/^(?:\.{0,2}\/|\/)(?!\/)/.test(source) || /^[^:/?#]+(?:[/?#]|$)/.test(source)) return source;

  try {
    const url = new URL(source);
    if (url.protocol === "https:" || url.protocol === "asset:") return source;
    if (url.protocol === "http:" && url.hostname === "asset.localhost") return source;
  } catch {
    return null;
  }
  return null;
}

export function SafeImage({
  src,
  alt,
  className,
}: {
  src?: string;
  alt?: string;
  className?: string;
}) {
  const { t } = useTranslation();
  const safeSource = safeImageSource(src);
  const [failed, setFailed] = useState(false);
  const [previewOpen, setPreviewOpen] = useState(false);
  const label = alt?.trim() || t("chat.richMedia.imageAlt");

  if (!safeSource || failed) {
    return (
      <span className="my-2 inline-flex max-w-full items-center gap-2 rounded-md border border-border bg-muted px-3 py-2 text-xs text-muted-foreground" role="img" aria-label={label}>
        <ImageOff className="h-4 w-4 shrink-0" aria-hidden="true" />
        <span className="truncate">{t("chat.richMedia.imageUnavailable")}</span>
      </span>
    );
  }

  const preview = previewOpen && typeof document !== "undefined"
    ? createPortal(
        <ApplicationDialog
          description={label}
          maxWidth="max-w-6xl"
          onClose={() => setPreviewOpen(false)}
          title={t("chat.richMedia.imagePreviewTitle")}
        >
          <div className="relative flex min-h-48 items-center justify-center overflow-auto rounded-lg bg-muted/50 p-2">
            <img
              alt={label}
              className="max-h-[70vh] max-w-full object-contain"
              referrerPolicy="no-referrer"
              src={safeSource}
            />
          </div>
          <button
            aria-label={t("chat.richMedia.closeImagePreview")}
            className="absolute right-6 top-6 inline-flex h-8 w-8 items-center justify-center rounded-md border border-border bg-background text-muted-foreground hover:bg-muted hover:text-foreground"
            data-dialog-autofocus
            onClick={() => setPreviewOpen(false)}
            type="button"
          >
            <X className="h-4 w-4" aria-hidden="true" />
          </button>
        </ApplicationDialog>,
        document.body,
      )
    : null;

  return (
    <>
      <button
        aria-label={t("chat.richMedia.openImagePreview", { label })}
        className="group relative my-2 block max-w-full cursor-zoom-in overflow-hidden rounded-md border border-border bg-muted/40"
        onClick={() => setPreviewOpen(true)}
        type="button"
      >
        <img
          alt={label}
          className={cn("max-h-96 max-w-full object-contain", className)}
          decoding="async"
          loading="lazy"
          onError={() => setFailed(true)}
          referrerPolicy="no-referrer"
          src={safeSource}
        />
        <span className="absolute right-2 top-2 inline-flex h-7 w-7 items-center justify-center rounded-md bg-black/60 text-white opacity-0 transition-opacity group-hover:opacity-100 group-focus-visible:opacity-100" aria-hidden="true">
          <Maximize2 className="h-3.5 w-3.5" />
        </span>
      </button>
      {preview}
    </>
  );
}
