import { Volume2 } from "lucide-react";
import { useTranslation } from "react-i18next";

import type {
  AudioDeviceCatalog,
  EngineStatus,
  LocalMediaDevice,
  SherpaOnnxTtsProfile,
  TtsModelKind,
} from "../../../types/local-media";
import { EngineCard } from "./engine-card";
import { DeviceField, NumberField, PathField, SelectField } from "./profile-fields";
import { DEVICE_OPTIONS, TTS_MODEL_KIND_OPTIONS, type FieldIssueLookup } from "./shared";

export function TtsCard({
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
  onUpdate: (mutate: (current: SherpaOnnxTtsProfile) => SherpaOnnxTtsProfile) => void;
  probeDisabledReasonKey: string | null;
  probing: boolean;
  profile: SherpaOnnxTtsProfile;
  status: EngineStatus | undefined;
}) {
  const { t } = useTranslation();
  const issue = (field: string) => issueFor("tts", field);

  return (
    <EngineCard
      description={t("localMedia.settings.tts.description")}
      enabled={profile.enabled}
      engine="tts"
      icon={Volume2}
      onProbe={onProbe}
      onToggle={(enabled) => onUpdate((current) => ({ ...current, enabled }))}
      probeDisabledReasonKey={probeDisabledReasonKey}
      probing={probing}
      status={status}
      title={t("localMedia.settings.tts.title")}
    >
      <PathField
        hintKey="localMedia.settings.hint.pythonExecutable"
        id="local-media-tts-python"
        issueKey={issue("pythonExecutable")}
        kind="file"
        label={t("localMedia.settings.field.pythonExecutable")}
        onChange={(pythonExecutable) => onUpdate((current) => ({ ...current, pythonExecutable }))}
        value={profile.pythonExecutable}
      />
      <SelectField<TtsModelKind>
        hintKey="localMedia.settings.hint.modelKind"
        id="local-media-tts-kind"
        issueKey={issue("modelKind")}
        label={t("localMedia.settings.field.modelKind")}
        onChange={(modelKind) => onUpdate((current) => ({ ...current, modelKind }))}
        options={TTS_MODEL_KIND_OPTIONS}
        value={profile.modelKind}
      />
      <PathField
        hintKey="localMedia.settings.hint.ttsModelPath"
        id="local-media-tts-model"
        issueKey={issue("modelPath")}
        kind="file"
        label={t("localMedia.settings.field.modelPath")}
        onChange={(modelPath) => onUpdate((current) => ({ ...current, modelPath }))}
        value={profile.modelPath}
      />
      <PathField
        id="local-media-tts-tokens"
        issueKey={issue("tokensPath")}
        kind="file"
        label={t("localMedia.settings.field.tokensPath")}
        onChange={(tokensPath) => onUpdate((current) => ({ ...current, tokensPath }))}
        value={profile.tokensPath}
      />
      {/* Only the kind that requires it is shown: an idle `voicesPath` input beside a VITS model
          invites filling it in, and sherpa-onnx would then reject the pair at load time. */}
      {profile.modelKind === "kokoro" ? (
        <PathField
          id="local-media-tts-voices"
          issueKey={issue("voicesPath")}
          kind="file"
          label={t("localMedia.settings.field.voicesPath")}
          onChange={(value) => onUpdate((current) => ({ ...current, voicesPath: value || null }))}
          value={profile.voicesPath ?? ""}
        />
      ) : null}
      {profile.modelKind === "matcha" ? (
        <PathField
          id="local-media-tts-vocoder"
          issueKey={issue("vocoderPath")}
          kind="file"
          label={t("localMedia.settings.field.vocoderPath")}
          onChange={(value) => onUpdate((current) => ({ ...current, vocoderPath: value || null }))}
          value={profile.vocoderPath ?? ""}
        />
      ) : null}
      <PathField
        hintKey="localMedia.settings.hint.lexiconPath"
        id="local-media-tts-lexicon"
        issueKey={issue("lexiconPath")}
        kind="file"
        label={t("localMedia.settings.field.lexiconPath")}
        onChange={(value) => onUpdate((current) => ({ ...current, lexiconPath: value || null }))}
        optional
        value={profile.lexiconPath ?? ""}
      />
      <PathField
        hintKey="localMedia.settings.hint.dataDir"
        id="local-media-tts-data-dir"
        issueKey={issue("dataDir")}
        kind="directory"
        label={t("localMedia.settings.field.dataDir")}
        onChange={(value) => onUpdate((current) => ({ ...current, dataDir: value || null }))}
        optional
        value={profile.dataDir ?? ""}
      />
      <PathField
        hintKey="localMedia.settings.hint.dictDir"
        id="local-media-tts-dict-dir"
        issueKey={issue("dictDir")}
        kind="directory"
        label={t("localMedia.settings.field.dictDir")}
        onChange={(value) => onUpdate((current) => ({ ...current, dictDir: value || null }))}
        optional
        value={profile.dictDir ?? ""}
      />
      <NumberField
        hintKey="localMedia.settings.hint.speakerId"
        id="local-media-tts-speaker"
        issueKey={issue("speakerId")}
        label={t("localMedia.settings.field.speakerId")}
        max={4096}
        min={0}
        onChange={(speakerId) => onUpdate((current) => ({ ...current, speakerId }))}
        value={profile.speakerId}
      />
      <NumberField
        hintKey="localMedia.settings.hint.speed"
        id="local-media-tts-speed"
        issueKey={issue("speed")}
        label={t("localMedia.settings.field.speed")}
        max={2}
        min={0.5}
        onChange={(speed) => onUpdate((current) => ({ ...current, speed }))}
        step={0.1}
        value={profile.speed}
      />
      <NumberField
        hintKey="localMedia.settings.hint.numThreads"
        id="local-media-tts-threads"
        issueKey={issue("numThreads")}
        label={t("localMedia.settings.field.numThreads")}
        max={16}
        min={1}
        onChange={(numThreads) => onUpdate((current) => ({ ...current, numThreads }))}
        value={profile.numThreads}
      />
      <SelectField<LocalMediaDevice>
        hintKey="localMedia.settings.hint.device"
        id="local-media-tts-device"
        issueKey={issue("device")}
        label={t("localMedia.settings.field.device")}
        onChange={(device) => onUpdate((current) => ({ ...current, device }))}
        options={DEVICE_OPTIONS}
        value={profile.device}
      />
      <DeviceField
        devices={devices.outputs}
        emptyKey="localMedia.settings.hint.noOutputDevices"
        id="local-media-tts-output"
        label={t("localMedia.settings.field.outputDevice")}
        onChange={(outputDeviceId) => onUpdate((current) => ({ ...current, outputDeviceId }))}
        value={profile.outputDeviceId}
      />
      {/* One path per line rather than a repeating row control. sherpa-onnx applies rule FSTs in
          order, and a plain text list is the one editor where that order is visible and editable
          at a glance. Blank lines are dropped so a trailing newline is not a validation error. */}
      <div className="grid gap-1.5 sm:col-span-2">
        <label className="text-xs font-medium leading-5 text-foreground" htmlFor="local-media-tts-rule-fsts">
          {t("localMedia.settings.field.ruleFsts")}
        </label>
        <textarea
          aria-invalid={issue("ruleFsts") ? true : undefined}
          className="min-h-20 w-full rounded-md border border-border bg-background px-3 py-2 font-mono text-sm focus-visible:outline-hidden focus-visible:ring-2 focus-visible:ring-ring"
          id="local-media-tts-rule-fsts"
          onChange={(event) =>
            onUpdate((current) => ({
              ...current,
              ruleFsts: event.target.value.split("\n").filter((line) => line.trim().length > 0),
            }))
          }
          spellCheck={false}
          value={profile.ruleFsts.join("\n")}
        />
        <p className="text-xs leading-5 text-muted-foreground">
          {t("localMedia.settings.hint.ruleFsts")}
        </p>
        {issue("ruleFsts") ? (
          <p className="text-xs leading-5 text-danger" role="alert">
            {t(issue("ruleFsts") ?? "")}
          </p>
        ) : null}
      </div>
    </EngineCard>
  );
}
