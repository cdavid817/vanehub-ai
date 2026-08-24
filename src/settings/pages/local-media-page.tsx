import { AudioLines } from "lucide-react";
import { useTranslation } from "react-i18next";

import { Button } from "../../components/ui/button";
import { localMediaMessageKey } from "../../session-workspace/local-media/local-media-errors";
import type { LocalMediaEngine } from "../../types/local-media";
import { OcrCard } from "./local-media/ocr-card";
import { SttCard } from "./local-media/stt-card";
import { TtsCard } from "./local-media/tts-card";
import { ToggleField } from "./local-media/profile-fields";
import { issueKeyFor, useLocalMediaSettings } from "./local-media/use-local-media-settings";
import { PageHeader, SectionPanel } from "./page-parts";
import type { SettingsPageContext } from "../settings-pages";

/**
 * Local media configuration: OCR, speech recognition, and speech synthesis.
 *
 * The page configures software the user installed themselves and never installs any of it. There
 * is no download button, no "get models" link, and no automatic retry that would reach the
 * network -- an engine that is not present stays not present, and the page says so.
 */
export function LocalMediaPage({ isActive }: SettingsPageContext) {
  const { t } = useTranslation();
  const model = useLocalMediaSettings(isActive);
  const { draft, status } = model;

  const issueFor = (engine: LocalMediaEngine | null, field: string) =>
    model.issues.get(issueKeyFor(engine, field))?.messageKey;
  const statusFor = (engine: LocalMediaEngine) =>
    status?.engines.find((entry) => entry.engine === engine);

  // Probing the draft would answer a question about a configuration that is not stored, so a
  // dirty form disables the check rather than silently probing the previous values.
  const probeBlockedKey = model.dirty
    ? "localMedia.settings.probeBlockedDirty"
    : !model.nativeAvailable
      ? "localMedia.composer.nativeOnly"
      : null;

  return (
    <div className="mx-auto w-full max-w-5xl">
      <PageHeader
        actions={
          <div className="flex items-center gap-2">
            <Button
              disabled={!model.dirty || model.saveState.kind === "saving"}
              onClick={model.discard}
              type="button"
              variant="outline"
            >
              {t("localMedia.settings.discard")}
            </Button>
            <Button
              data-testid="local-media-save"
              disabled={!model.dirty || model.saveState.kind === "saving"}
              // Wrapped, not passed: `save` now takes an optional profile, and handing it straight
              // to onClick would offer it the MouseEvent as one.
              onClick={() => model.save()}
              type="button"
            >
              {t(model.saveState.kind === "saving" ? "localMedia.settings.saving" : "localMedia.settings.save")}
            </Button>
          </div>
        }
        description={t("localMedia.settings.description")}
        icon={AudioLines}
        title={t("localMedia.settings.title")}
      />

      <div className="grid gap-5">
        {!model.nativeAvailable && !model.loading ? (
          <p className="ucd-status-warning rounded-lg px-4 py-3 text-sm" role="status">
            {t("localMedia.settings.nativeOnly")}
          </p>
        ) : null}
        {status && status.platformSupport !== "supported" ? (
          <p className="ucd-status-warning rounded-lg px-4 py-3 text-sm">
            {t(`localMedia.settings.platform.${status.platformSupport}`)}
          </p>
        ) : null}
        <SaveFeedback model={model} />

        {model.loadError ? (
          <p className="ucd-status-danger rounded-lg px-4 py-3 text-sm" role="alert">
            {t(localMediaMessageKey(model.loadError))}
          </p>
        ) : null}

        {draft ? (
          <>
            <SectionPanel
              description={t("localMedia.settings.master.description")}
              title={t("localMedia.settings.master.title")}
              variant="settings"
            >
              <div className="flex items-center justify-between gap-4 px-5 py-4 sm:px-6">
                <p className="text-xs leading-5 text-muted-foreground">
                  {t("localMedia.settings.master.hint")}
                </p>
                <ToggleField
                  checked={draft.enabled}
                  label={t("localMedia.settings.master.title")}
                  onChange={(enabled) => model.update((current) => ({ ...current, enabled }))}
                />
              </div>
            </SectionPanel>

            <OcrCard
              issueFor={issueFor}
              // Saved through the ordinary save path, then re-probed. Nothing here retries or
              // degrades on its own: this runs only when the user presses the confirm button.
              onDisableAcceleration={() => {
                model.save({ ...draft, ocr: { ...draft.ocr, cpuAcceleration: "disabled" } });
                model.probe("ocr");
              }}
              onProbe={() => model.probe("ocr")}
              onUpdate={(mutate) => model.update((current) => ({ ...current, ocr: mutate(current.ocr) }))}
              probeDisabledReasonKey={probeBlockedKey}
              probing={model.probing === "ocr"}
              profile={draft.ocr}
              status={statusFor("ocr")}
            />
            <SttCard
              devices={model.devices}
              issueFor={issueFor}
              onProbe={() => model.probe("stt")}
              onUpdate={(mutate) => model.update((current) => ({ ...current, stt: mutate(current.stt) }))}
              probeDisabledReasonKey={probeBlockedKey}
              probing={model.probing === "stt"}
              profile={draft.stt}
              status={statusFor("stt")}
            />
            <TtsCard
              devices={model.devices}
              issueFor={issueFor}
              onProbe={() => model.probe("tts")}
              onUpdate={(mutate) => model.update((current) => ({ ...current, tts: mutate(current.tts) }))}
              probeDisabledReasonKey={probeBlockedKey}
              probing={model.probing === "tts"}
              profile={draft.tts}
              status={statusFor("tts")}
            />

            <p className="text-xs leading-5 text-muted-foreground">
              {t("localMedia.settings.privacyNote")}
            </p>
          </>
        ) : null}
      </div>
    </div>
  );
}

function SaveFeedback({ model }: { model: ReturnType<typeof useLocalMediaSettings> }) {
  const { t } = useTranslation();
  const { saveState } = model;

  if (saveState.kind === "conflict") {
    return (
      <div className="ucd-status-warning flex flex-wrap items-center gap-3 rounded-lg px-4 py-3 text-sm" role="alert">
        <span>{t("localMedia.settings.conflict")}</span>
        <Button onClick={model.reloadFromNative} size="sm" type="button" variant="outline">
          {t("localMedia.settings.reload")}
        </Button>
      </div>
    );
  }
  if (saveState.kind === "failed") {
    return (
      <p className="ucd-status-danger rounded-lg px-4 py-3 text-sm" role="alert">
        {t(localMediaMessageKey(saveState.code))}
      </p>
    );
  }
  if (saveState.kind === "saved") {
    return (
      <p className="ucd-status-success rounded-lg px-4 py-3 text-sm" role="status">
        {t("localMedia.settings.saved")}
      </p>
    );
  }
  return null;
}
