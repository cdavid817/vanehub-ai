import { ScanText } from "lucide-react";
import { useTranslation } from "react-i18next";

import type {
  EngineStatus,
  LocalMediaDevice,
  OcrCpuAcceleration,
  PaddleOcrProfile,
} from "../../../types/local-media";
import { CompatibilityNotice, compatibilityCodeOf } from "./compatibility-notice";
import { AdvancedFields } from "./advanced-fields";
import { EngineCard } from "./engine-card";
import { NumberField, PathField, SelectField, TextField } from "./profile-fields";
import { CPU_ACCELERATION_OPTIONS, DEVICE_OPTIONS, type FieldIssueLookup } from "./shared";

export function OcrCard({
  issueFor,
  onDisableAcceleration,
  onProbe,
  onUpdate,
  probeDisabledReasonKey,
  probing,
  profile,
  status,
}: {
  issueFor: FieldIssueLookup;
  onDisableAcceleration: () => void;
  onProbe: () => void;
  onUpdate: (mutate: (current: PaddleOcrProfile) => PaddleOcrProfile) => void;
  probeDisabledReasonKey: string | null;
  probing: boolean;
  profile: PaddleOcrProfile;
  status: EngineStatus | undefined;
}) {
  const { t } = useTranslation();
  const issue = (field: string) => issueFor("ocr", field);
  const compatibility = compatibilityCodeOf(status);

  return (
    <EngineCard
      description={t("localMedia.settings.ocr.description")}
      enabled={profile.enabled}
      engine="ocr"
      icon={ScanText}
      onProbe={onProbe}
      onToggle={(enabled) => onUpdate((current) => ({ ...current, enabled }))}
      probeDisabledReasonKey={probeDisabledReasonKey}
      probing={probing}
      status={status}
      title={t("localMedia.settings.ocr.title")}
    >
      {compatibility ? (
        <CompatibilityNotice
          code={compatibility}
          // Only the acceleration case is actionable from here. The encoding cases are answered by
          // the user moving files, and a button that relocated a model on their behalf would be
          // touching files they chose without being asked.
          onConfirm={
            compatibility === "PADDLE_ONEDNN_MODEL_INCOMPATIBLE" ? onDisableAcceleration : undefined
          }
          pending={probing}
        />
      ) : null}
      <PathField
        hintKey="localMedia.settings.hint.paddleXConfigPath"
        id="local-media-ocr-paddlex"
        issueKey={issue("paddleXConfigPath")}
        kind="file"
        label={t("localMedia.settings.field.paddleXConfigPath")}
        onChange={(value) =>
          onUpdate((current) => ({ ...current, paddleXConfigPath: value || null }))
        }
        optional
        value={profile.paddleXConfigPath ?? ""}
      />
      <PathField
        hintKey="localMedia.settings.hint.modelDirectories"
        id="local-media-ocr-detection"
        issueKey={issue("textDetectionModelDir")}
        kind="directory"
        label={t("localMedia.settings.field.textDetectionModelDir")}
        onChange={(value) =>
          onUpdate((current) => ({ ...current, textDetectionModelDir: value || null }))
        }
        optional
        value={profile.textDetectionModelDir ?? ""}
      />
      <PathField
        id="local-media-ocr-recognition"
        issueKey={issue("textRecognitionModelDir")}
        kind="directory"
        label={t("localMedia.settings.field.textRecognitionModelDir")}
        onChange={(value) =>
          onUpdate((current) => ({ ...current, textRecognitionModelDir: value || null }))
        }
        optional
        value={profile.textRecognitionModelDir ?? ""}
      />
      <AdvancedFields
        hasError={Boolean(issue("textLineOrientationModelDir") || issue("language") || issue("device") || issue("cpuAcceleration") || issue("maxPdfPages"))}
        id="local-media-ocr-advanced"
      >
        <PathField
          hintKey="localMedia.settings.hint.textLineOrientationModelDir"
          id="local-media-ocr-orientation"
          issueKey={issue("textLineOrientationModelDir")}
          kind="directory"
          label={t("localMedia.settings.field.textLineOrientationModelDir")}
          onChange={(value) => onUpdate((current) => ({ ...current, textLineOrientationModelDir: value || null }))}
          optional
          value={profile.textLineOrientationModelDir ?? ""}
        />
        <TextField hintKey="localMedia.settings.hint.ocrLanguage" id="local-media-ocr-language" issueKey={issue("language")} label={t("localMedia.settings.field.language")} onChange={(language) => onUpdate((current) => ({ ...current, language }))} value={profile.language} />
        <SelectField<LocalMediaDevice> hintKey="localMedia.settings.hint.device" id="local-media-ocr-device" issueKey={issue("device")} label={t("localMedia.settings.field.device")} onChange={(device) => onUpdate((current) => ({ ...current, device }))} options={DEVICE_OPTIONS} value={profile.device} />
        <SelectField<OcrCpuAcceleration> hintKey="localMedia.ocr.cpuAcceleration.hint" id="local-media-ocr-cpu-acceleration" issueKey={issue("cpuAcceleration")} label={t("localMedia.ocr.cpuAcceleration.label")} onChange={(cpuAcceleration) => onUpdate((current) => ({ ...current, cpuAcceleration }))} options={CPU_ACCELERATION_OPTIONS} value={profile.cpuAcceleration} />
        <NumberField hintKey="localMedia.settings.hint.maxPdfPages" id="local-media-ocr-max-pdf-pages" issueKey={issue("maxPdfPages")} label={t("localMedia.settings.field.maxPdfPages")} max={200} min={1} onChange={(maxPdfPages) => onUpdate((current) => ({ ...current, maxPdfPages }))} value={profile.maxPdfPages} />
      </AdvancedFields>
    </EngineCard>
  );
}
