import { useTranslation } from "react-i18next";

import type { EngineStatus, LocalMediaErrorCode } from "../../../types/local-media";

/**
 * The remediation a vendor-compatibility code offers, or nothing.
 *
 * Keyed on the code rather than on the engine: the same encoding failure reaches OCR and TTS, and
 * the answer to it is the same sentence in both.
 */
const REMEDIATION_KEYS: Partial<Record<LocalMediaErrorCode, string>> = {
  PADDLE_ONEDNN_MODEL_INCOMPATIBLE:
    "localMedia.compatibility.remediation.disableCpuAcceleration",
  MODEL_PATH_ENCODING_UNSUPPORTED: "localMedia.compatibility.remediation.relocateToAsciiPath",
  TTS_DATA_PATH_ENCODING_UNSUPPORTED: "localMedia.compatibility.remediation.relocateToAsciiPath",
  TTS_PHONEMIZER_DATA_UNAVAILABLE: "localMedia.compatibility.remediation.configureDataDirectory",
};

const ERROR_KEYS: Partial<Record<LocalMediaErrorCode, string>> = {
  PADDLE_ONEDNN_MODEL_INCOMPATIBLE: "localMedia.errors.paddleOnednnModelIncompatible",
  MODEL_PATH_ENCODING_UNSUPPORTED: "localMedia.errors.modelPathEncodingUnsupported",
  TTS_DATA_PATH_ENCODING_UNSUPPORTED: "localMedia.errors.ttsDataPathEncodingUnsupported",
  TTS_PHONEMIZER_DATA_UNAVAILABLE: "localMedia.errors.ttsPhonemizerDataUnavailable",
};

/** The localized message key for a vendor-compatibility code, or `undefined` for anything else. */
export function compatibilityMessageKey(code: LocalMediaErrorCode): string | undefined {
  return ERROR_KEYS[code];
}

export function compatibilityCodeOf(status: EngineStatus | undefined): LocalMediaErrorCode | null {
  if (!status || status.readiness.state !== "unavailable") return null;
  return status.readiness.code in ERROR_KEYS ? status.readiness.code : null;
}

/**
 * A vendor-compatibility failure, its affected field, and what the user can do about it.
 *
 * `onConfirm` is present only for the acceleration case, which is the only one this application can
 * act on: the others are answered by the user moving files, and offering a button that silently
 * relocated a model would be doing something to their files they did not ask for.
 */
export function CompatibilityNotice({
  code,
  field,
  onConfirm,
  pending,
}: {
  code: LocalMediaErrorCode;
  field?: string | null;
  onConfirm?: () => void;
  pending?: boolean;
}) {
  const { t } = useTranslation();
  const errorKey = ERROR_KEYS[code];
  const remediationKey = REMEDIATION_KEYS[code];
  if (!errorKey || !remediationKey) return null;

  return (
    <div
      className="rounded-md border border-amber-500/40 bg-amber-500/10 p-3 text-sm"
      data-testid={`local-media-compatibility-${code}`}
      role="status"
    >
      <p className="font-medium">{t(errorKey)}</p>
      {field ? (
        <p className="mt-1 text-muted-foreground">{t(`localMedia.fields.${field}`, field)}</p>
      ) : null}
      <p className="mt-1 text-muted-foreground">{t(remediationKey)}</p>
      {onConfirm ? (
        <button
          className="mt-2 rounded-md border px-2 py-1 text-xs font-medium disabled:opacity-50"
          disabled={pending}
          onClick={onConfirm}
          type="button"
        >
          {t("localMedia.compatibility.confirmDisableAcceleration")}
        </button>
      ) : null}
    </div>
  );
}
