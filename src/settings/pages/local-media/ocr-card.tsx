import { ScanText } from "lucide-react";
import { useTranslation } from "react-i18next";

import type {
  EngineStatus,
  LocalMediaDevice,
  PaddleOcrProfile,
} from "../../../types/local-media";
import { EngineCard } from "./engine-card";
import { NumberField, PathField, SelectField, TextField } from "./profile-fields";
import { DEVICE_OPTIONS, type FieldIssueLookup } from "./shared";

export function OcrCard({
  issueFor,
  onProbe,
  onUpdate,
  probeDisabledReasonKey,
  probing,
  profile,
  status,
}: {
  issueFor: FieldIssueLookup;
  onProbe: () => void;
  onUpdate: (mutate: (current: PaddleOcrProfile) => PaddleOcrProfile) => void;
  probeDisabledReasonKey: string | null;
  probing: boolean;
  profile: PaddleOcrProfile;
  status: EngineStatus | undefined;
}) {
  const { t } = useTranslation();
  const issue = (field: string) => issueFor("ocr", field);

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
      <PathField
        hintKey="localMedia.settings.hint.pythonExecutable"
        id="local-media-ocr-python"
        issueKey={issue("pythonExecutable")}
        kind="file"
        label={t("localMedia.settings.field.pythonExecutable")}
        onChange={(pythonExecutable) => onUpdate((current) => ({ ...current, pythonExecutable }))}
        value={profile.pythonExecutable}
      />
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
      <PathField
        hintKey="localMedia.settings.hint.textLineOrientationModelDir"
        id="local-media-ocr-orientation"
        issueKey={issue("textLineOrientationModelDir")}
        kind="directory"
        label={t("localMedia.settings.field.textLineOrientationModelDir")}
        onChange={(value) =>
          onUpdate((current) => ({ ...current, textLineOrientationModelDir: value || null }))
        }
        optional
        value={profile.textLineOrientationModelDir ?? ""}
      />
      <TextField
        hintKey="localMedia.settings.hint.ocrLanguage"
        id="local-media-ocr-language"
        issueKey={issue("language")}
        label={t("localMedia.settings.field.language")}
        onChange={(language) => onUpdate((current) => ({ ...current, language }))}
        value={profile.language}
      />
      <SelectField<LocalMediaDevice>
        hintKey="localMedia.settings.hint.device"
        id="local-media-ocr-device"
        issueKey={issue("device")}
        label={t("localMedia.settings.field.device")}
        onChange={(device) => onUpdate((current) => ({ ...current, device }))}
        options={DEVICE_OPTIONS}
        value={profile.device}
      />
      <NumberField
        hintKey="localMedia.settings.hint.maxPdfPages"
        id="local-media-ocr-max-pdf-pages"
        issueKey={issue("maxPdfPages")}
        label={t("localMedia.settings.field.maxPdfPages")}
        max={200}
        min={1}
        onChange={(maxPdfPages) => onUpdate((current) => ({ ...current, maxPdfPages }))}
        value={profile.maxPdfPages}
      />
    </EngineCard>
  );
}
