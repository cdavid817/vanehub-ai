import { Mic } from "lucide-react";
import { useTranslation } from "react-i18next";

import type {
  AudioDeviceCatalog,
  EngineStatus,
  FasterWhisperProfile,
  LocalMediaDevice,
  WhisperComputeType,
} from "../../../types/local-media";
import { EngineCard } from "./engine-card";
import { AdvancedFields } from "./advanced-fields";
import {
  DeviceField,
  NumberField,
  PathField,
  SelectField,
  TextField,
  ToggleField,
} from "./profile-fields";
import { COMPUTE_TYPE_OPTIONS, DEVICE_OPTIONS, type FieldIssueLookup } from "./shared";

export function SttCard({
  devices,
  issueFor,
  onProbe,
  onUpdate,
  probeDisabledReasonKey,
  probing,
  profile,
  status,
}: {
  devices: AudioDeviceCatalog;
  issueFor: FieldIssueLookup;
  onProbe: () => void;
  onUpdate: (mutate: (current: FasterWhisperProfile) => FasterWhisperProfile) => void;
  probeDisabledReasonKey: string | null;
  probing: boolean;
  profile: FasterWhisperProfile;
  status: EngineStatus | undefined;
}) {
  const { t } = useTranslation();
  const issue = (field: string) => issueFor("stt", field);

  return (
    <EngineCard
      description={t("localMedia.settings.stt.description")}
      enabled={profile.enabled}
      engine="stt"
      icon={Mic}
      onProbe={onProbe}
      onToggle={(enabled) => onUpdate((current) => ({ ...current, enabled }))}
      probeDisabledReasonKey={probeDisabledReasonKey}
      probing={probing}
      status={status}
      title={t("localMedia.settings.stt.title")}
    >
      <PathField
        hintKey="localMedia.settings.hint.whisperModelDirectory"
        id="local-media-stt-model"
        issueKey={issue("modelDirectory")}
        kind="directory"
        label={t("localMedia.settings.field.modelDirectory")}
        onChange={(modelDirectory) => onUpdate((current) => ({ ...current, modelDirectory }))}
        value={profile.modelDirectory}
      />
      <AdvancedFields
        hasError={Boolean(issue("language") || issue("device") || issue("computeType") || issue("beamSize") || issue("maxRecordingSeconds"))}
        id="local-media-stt-advanced"
      >
      <DeviceField
        devices={devices.inputs}
        emptyKey="localMedia.settings.hint.noInputDevices"
        id="local-media-stt-microphone"
        label={t("localMedia.settings.field.microphone")}
        onChange={(microphoneDeviceId) => onUpdate((current) => ({ ...current, microphoneDeviceId }))}
        value={profile.microphoneDeviceId}
      />
      <TextField
        hintKey="localMedia.settings.hint.sttLanguage"
        id="local-media-stt-language"
        issueKey={issue("language")}
        label={t("localMedia.settings.field.language")}
        onChange={(language) => onUpdate((current) => ({ ...current, language }))}
        value={profile.language}
      />
      <SelectField<LocalMediaDevice>
        hintKey="localMedia.settings.hint.device"
        id="local-media-stt-device"
        issueKey={issue("device")}
        label={t("localMedia.settings.field.device")}
        onChange={(device) => onUpdate((current) => ({ ...current, device }))}
        options={DEVICE_OPTIONS}
        value={profile.device}
      />
      <SelectField<WhisperComputeType>
        hintKey="localMedia.settings.hint.computeType"
        id="local-media-stt-compute"
        issueKey={issue("computeType")}
        label={t("localMedia.settings.field.computeType")}
        onChange={(computeType) => onUpdate((current) => ({ ...current, computeType }))}
        options={COMPUTE_TYPE_OPTIONS}
        value={profile.computeType}
      />
      <NumberField
        hintKey="localMedia.settings.hint.beamSize"
        id="local-media-stt-beam"
        issueKey={issue("beamSize")}
        label={t("localMedia.settings.field.beamSize")}
        max={10}
        min={1}
        onChange={(beamSize) => onUpdate((current) => ({ ...current, beamSize }))}
        value={profile.beamSize}
      />
      <NumberField
        hintKey="localMedia.settings.hint.maxRecordingSeconds"
        id="local-media-stt-max-seconds"
        issueKey={issue("maxRecordingSeconds")}
        label={t("localMedia.settings.field.maxRecordingSeconds")}
        max={600}
        min={5}
        onChange={(maxRecordingSeconds) => onUpdate((current) => ({ ...current, maxRecordingSeconds }))}
        value={profile.maxRecordingSeconds}
      />
      <div className="grid gap-1.5">
        <span className="text-xs font-medium leading-5 text-foreground">
          {t("localMedia.settings.field.vadFilter")}
        </span>
        <div>
          <ToggleField
            checked={profile.vadFilter}
            label={t("localMedia.settings.field.vadFilter")}
            onChange={(vadFilter) => onUpdate((current) => ({ ...current, vadFilter }))}
          />
        </div>
        <p className="text-xs leading-5 text-muted-foreground">
          {t("localMedia.settings.hint.vadFilter")}
        </p>
      </div>
      </AdvancedFields>
    </EngineCard>
  );
}
