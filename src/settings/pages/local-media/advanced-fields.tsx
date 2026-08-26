import { ChevronDown } from "lucide-react";
import { type ReactNode, useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";

export function AdvancedFields({ children, hasError, id }: { children: ReactNode; hasError: boolean; id: string }) {
  const { t } = useTranslation();
  const [open, setOpen] = useState(false);
  const contentRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!hasError) return;
    setOpen(true);
    requestAnimationFrame(() => {
      contentRef.current?.querySelector<HTMLElement>("[aria-invalid='true'], [role='alert']")?.focus();
    });
  }, [hasError]);

  return (
    <div className="grid gap-3 sm:col-span-2">
      <button
        aria-controls={id}
        aria-expanded={open}
        className="flex min-h-9 items-center justify-between rounded-md border border-border px-3 text-sm font-medium hover:bg-muted/50 focus-visible:outline-hidden focus-visible:ring-2 focus-visible:ring-ring"
        onClick={() => setOpen((current) => !current)}
        type="button"
      >
        {t("localMedia.settings.advanced")}
        <ChevronDown className={`h-4 w-4 transition-transform ${open ? "rotate-180" : ""}`} />
      </button>
      {open ? <div className="grid gap-4 sm:grid-cols-2" id={id} ref={contentRef}>{children}</div> : null}
    </div>
  );
}
