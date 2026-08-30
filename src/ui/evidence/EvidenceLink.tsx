import { useEffect, useState } from "react";
import { Check, Copy, ExternalLink } from "lucide-react";
import { useTranslation } from "react-i18next";
import { Link } from "react-router";
import { cn } from "../../lib/utils";
import { StatusBadge } from "../status/StatusBadge";

export type EvidenceAvailability = "available" | "unavailable" | "restricted";

export interface EvidenceReturnContext {
  label: string;
  path: string;
}

export interface EvidenceLinkProps {
  label: string;
  /** Route to the authoritative page — never renders duplicated content inline. */
  to: string;
  availability: EvidenceAvailability;
  /** Caller-supplied, already-localized explanation shown when not available. */
  reason?: string;
  returnTo?: EvidenceReturnContext;
  /** A reference (id/link), never raw evidence content — enables the copy affordance. */
  copyValue?: string;
  className?: string;
}

export function EvidenceLink({ label, to, availability, reason, returnTo, copyValue, className }: EvidenceLinkProps) {
  const { t } = useTranslation();
  const [copied, setCopied] = useState(false);

  useEffect(() => {
    if (!copied) return;
    const timer = window.setTimeout(() => setCopied(false), 2000);
    return () => window.clearTimeout(timer);
  }, [copied]);

  if (availability !== "available") {
    return (
      <span className={cn("inline-flex flex-wrap items-center gap-1.5 text-sm text-muted-foreground", className)}>
        {label}
        <StatusBadge
          label={t(availability === "unavailable" ? "workbenchUi.evidence.unavailable" : "workbenchUi.evidence.restricted")}
          tone={availability === "unavailable" ? "neutral" : "blocked"}
        />
        {reason ? <span className="text-xs">{reason}</span> : null}
      </span>
    );
  }

  return (
    <span className={cn("inline-flex items-center gap-1.5", className)}>
      <Link
        className="inline-flex items-center gap-1 text-sm font-medium text-primary hover:underline"
        state={returnTo ? { returnTo } : undefined}
        to={to}
      >
        <ExternalLink aria-hidden="true" className="h-3.5 w-3.5" />
        {label}
      </Link>
      {copyValue ? (
        <button
          aria-label={t(copied ? "workbenchUi.evidence.copied" : "workbenchUi.evidence.copy")}
          onClick={() => { void navigator.clipboard.writeText(copyValue).then(() => setCopied(true)); }}
          type="button"
        >
          {copied ? <Check aria-hidden="true" className="h-3.5 w-3.5" /> : <Copy aria-hidden="true" className="h-3.5 w-3.5" />}
        </button>
      ) : null}
    </span>
  );
}
