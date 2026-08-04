import { useEffect, useRef, useState } from "react";
import { Activity, LoaderCircle } from "lucide-react";
import { useTranslation } from "react-i18next";
import type { ProviderCredentialValidationResult } from "../../types/provider-credential-validation";
import { Button } from "../ui/button";

export function ProviderCredentialValidation({
  disabled = false,
  onValidate,
  resetKey,
  size = "default",
}: {
  disabled?: boolean;
  onValidate: () => Promise<ProviderCredentialValidationResult>;
  resetKey: string | number;
  size?: "default" | "sm";
}) {
  const { t } = useTranslation();
  const [pending, setPending] = useState(false);
  const [result, setResult] = useState<ProviderCredentialValidationResult | null>(null);
  const [error, setError] = useState<string | null>(null);
  const requestSequence = useRef(0);

  useEffect(() => {
    requestSequence.current += 1;
    setPending(false);
    setResult(null);
    setError(null);
  }, [resetKey]);

  async function validate() {
    const requestId = ++requestSequence.current;
    setPending(true);
    setResult(null);
    setError(null);
    try {
      const next = await onValidate();
      if (requestId === requestSequence.current) setResult(next);
    } catch (reason) {
      if (requestId === requestSequence.current) {
        setError(reason instanceof Error ? reason.message : String(reason));
      }
    } finally {
      if (requestId === requestSequence.current) setPending(false);
    }
  }

  const statusClass = result?.status === "valid"
    ? "ucd-status-success"
    : result
      ? "ucd-status-warning"
      : "text-muted-foreground";

  return (
    <div className="flex flex-wrap items-center gap-2">
      <Button disabled={disabled || pending} onClick={() => void validate()} size={size} type="button" variant="outline">
        {pending ? <LoaderCircle className="h-4 w-4 animate-spin" /> : <Activity className="h-4 w-4" />}
        {pending ? t("providerCredentialValidation.checking") : t("providerCredentialValidation.action")}
      </Button>
      <span aria-live="polite" className={`text-xs ${error ? "ucd-status-warning" : statusClass}`} role={error ? "alert" : "status"}>
        {error ?? (result ? t(`providerCredentialValidation.status.${result.status}`) : t("providerCredentialValidation.hint"))}
      </span>
    </div>
  );
}
