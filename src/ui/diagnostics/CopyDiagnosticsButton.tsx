import { useEffect, useState } from "react";
import { Check, ClipboardCopy } from "lucide-react";
import { useTranslation } from "react-i18next";
import { Button } from "../../components/ui/button";
import { formatDiagnosticSummary, type DiagnosticField } from "./diagnostic-field";

export interface CopyDiagnosticsButtonProps {
  fields: readonly DiagnosticField[];
  className?: string;
}

/**
 * One button, reused across every page's own copy-diagnostics action (spec.md "Copyable safe
 * settings diagnostics") -- the redaction judgment call belongs to each page's own field builder,
 * this only formats and copies whatever bounded fields it is given. Copied-state pattern matches
 * `src/ui/evidence/EvidenceLink.tsx`'s existing one-string copy affordance.
 */
export function CopyDiagnosticsButton({ fields, className }: CopyDiagnosticsButtonProps) {
  const { t } = useTranslation();
  const [copied, setCopied] = useState(false);

  useEffect(() => {
    if (!copied) return;
    const timer = window.setTimeout(() => setCopied(false), 2000);
    return () => window.clearTimeout(timer);
  }, [copied]);

  return (
    <Button
      className={className}
      onClick={() => {
        const summary = formatDiagnosticSummary(fields, t("workbenchUi.diagnostics.unavailable"));
        void navigator.clipboard.writeText(summary).then(() => setCopied(true));
      }}
      size="sm"
      type="button"
      variant="outline"
    >
      {copied ? <Check aria-hidden="true" className="h-3.5 w-3.5" /> : <ClipboardCopy aria-hidden="true" className="h-3.5 w-3.5" />}
      {t(copied ? "workbenchUi.diagnostics.copied" : "workbenchUi.diagnostics.copy")}
    </Button>
  );
}
