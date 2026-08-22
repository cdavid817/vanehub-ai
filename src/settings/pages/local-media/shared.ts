import type {
  LocalMediaDevice,
  LocalMediaEngine,
  TtsModelKind,
  WhisperComputeType,
} from "../../../types/local-media";

/** Returns the locale key for a field's current issue, or `undefined` when the field is fine. */
export type FieldIssueLookup = (
  engine: LocalMediaEngine | null,
  field: string,
) => string | undefined;

export const DEVICE_OPTIONS: ReadonlyArray<{ value: LocalMediaDevice; labelKey: string }> = [
  { value: "auto", labelKey: "localMedia.settings.device.auto" },
  { value: "cpu", labelKey: "localMedia.settings.device.cpu" },
  { value: "cuda", labelKey: "localMedia.settings.device.cuda" },
];

export const COMPUTE_TYPE_OPTIONS: ReadonlyArray<{
  value: WhisperComputeType;
  labelKey: string;
}> = [
  { value: "auto", labelKey: "localMedia.settings.computeType.auto" },
  { value: "int8", labelKey: "localMedia.settings.computeType.int8" },
  { value: "int8_float16", labelKey: "localMedia.settings.computeType.int8Float16" },
  { value: "float16", labelKey: "localMedia.settings.computeType.float16" },
  { value: "float32", labelKey: "localMedia.settings.computeType.float32" },
];

export const TTS_MODEL_KIND_OPTIONS: ReadonlyArray<{ value: TtsModelKind; labelKey: string }> = [
  { value: "vits", labelKey: "localMedia.settings.modelKind.vits" },
  { value: "piper", labelKey: "localMedia.settings.modelKind.piper" },
  { value: "kokoro", labelKey: "localMedia.settings.modelKind.kokoro" },
  { value: "matcha", labelKey: "localMedia.settings.modelKind.matcha" },
];
