## ADDED Requirements

### Requirement: Every local-media user-visible string SHALL be localized in every registered locale

All composer actions, recording/transcription/synthesis states, OCR review text, settings navigation, fields, readiness states, privacy explanations, permission guidance, operation labels, warnings, and stable error messages introduced by this change SHALL exist in `zh-CN`, `en`, `zh-TW`, `ja`, and `ko` using the repository's existing locale catalog and key conventions.

#### Scenario: A local-media string is added

* WHEN a developer adds or changes a user-visible local-media string
* THEN equivalent keys SHALL exist in all five registered locales
* AND locale parity tests SHALL fail when a key is missing

#### Scenario: An icon-only action renders

* WHEN OCR, microphone, TTS, append, copy, cancel, check, or stop renders without visible text
* THEN its tooltip and accessible label SHALL come from locale keys
* AND the component SHALL not hard-code an English or Chinese fallback

### Requirement: Stable local-media errors SHALL resolve through localized message keys

Native and worker layers SHALL return stable error codes and message keys rather than user-facing raw text. The frontend SHALL render localized titles, descriptions, and recovery guidance from those keys.

#### Scenario: Microphone access is denied

* WHEN the native host returns `MIC_PERMISSION_DENIED`
* THEN the composer/settings UI SHALL show the current locale's microphone-permission guidance
* AND it SHALL not display a raw operating-system or Python exception as the primary user message

#### Scenario: A model is missing

* WHEN an engine returns `MODEL_NOT_FOUND`
* THEN the UI SHALL show localized local-configuration guidance
* AND it SHALL not imply that VaneHub will download the model automatically

### Requirement: Dynamic local-media status text SHALL remain localization-safe

Durations, counts, engine names, devices, and safe versions SHALL be formatted through structured interpolation and locale-aware formatting rather than concatenating translated sentence fragments.

#### Scenario: Recording duration is displayed

* WHEN the recording indicator shows elapsed time
* THEN the numeric duration SHALL use the shared duration formatter or a stable `mm:ss` representation
* AND the localized status text SHALL not depend on English word order

#### Scenario: OCR metadata is displayed

* WHEN the review dialog shows page and character counts
* THEN counts SHALL use locale-aware plural/number formatting supported by the repository
