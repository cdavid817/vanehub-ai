## ADDED Requirements

### Requirement: The primary chat composer SHALL expose a stable local-media action group

The primary API-session chat composer SHALL expose compact OCR, hold-to-talk, and text-to-speech actions adjacent to existing enhance/send controls. The action group SHALL use the existing composer service/state boundaries, semantic styles, and fixed control geometry without moving media lifecycle logic into `ButtonArea`.

#### Scenario: The desktop composer has ready engines

* WHEN the native runtime reports one or more ready local-media capabilities
* THEN the corresponding media actions SHALL be enabled
* AND each action SHALL identify that processing is local
* AND the send/stop action SHALL remain visible and retain its existing behavior

#### Scenario: A capability is unavailable

* WHEN an engine is disabled, unconfigured, probing, unavailable, or native-only in the current runtime
* THEN its composer action SHALL be disabled or expose only the valid recovery action
* AND its tooltip/status SHALL explain the reason and Local media settings destination

#### Scenario: Action state changes

* WHEN an action changes from icon to opening/processing/playing state
* THEN it SHALL retain fixed geometry
* AND the composer row SHALL not shift due solely to the state icon change

### Requirement: Composer OCR SHALL require review before draft insertion

The OCR action SHALL allow the user to choose one supported local image or PDF, submit it to local PaddleOCR, and review/edit the recognized text before explicitly appending it to the current draft. OCR SHALL never send a message automatically.

#### Scenario: The user cancels the picker

* WHEN the OCR source picker is closed without selecting a file
* THEN no staging or OCR operation SHALL start
* AND the draft SHALL remain unchanged

#### Scenario: OCR succeeds

* WHEN a typed OCR result is available for the active composer scope
* THEN an editable review dialog SHALL display the derived text, source metadata, warnings, character count, and local engine/provenance summary
* AND the draft SHALL remain unchanged until the user selects Append

#### Scenario: The user edits and appends OCR text

* WHEN the user confirms edited non-empty OCR text
* THEN the composer SHALL append it to the latest draft
* AND it SHALL insert a blank-line separator unless the current draft is empty or already ends with a newline
* AND it SHALL update the existing completion/draft state path
* AND it SHALL NOT send the message

#### Scenario: OCR output cannot fit the composer

* WHEN appending the reviewed text would violate an existing composer input limit
* THEN Append SHALL be disabled or fail recoverably
* AND the application SHALL preserve copy/cancel access to the result
* AND it SHALL NOT truncate or overwrite the draft silently

#### Scenario: OCR returns no text

* WHEN the OCR outcome is `NO_TEXT_DETECTED`
* THEN the review/result UI SHALL explain that no text was found
* AND the draft SHALL remain unchanged

### Requirement: The microphone action SHALL implement press-and-hold whole-utterance capture

The microphone action SHALL start recording only while a pointer or keyboard activation is held, SHALL show explicit local recording state, and SHALL finish and transcribe the complete utterance on a valid release. V1 SHALL not show or merge streaming partial transcripts.

#### Scenario: Pointer hold and release

* WHEN the user presses the microphone action and native recording starts successfully
* THEN the action SHALL capture that pointer and enter the recording state
* AND releasing that pointer SHALL finalize recording and start local transcription
* AND the synthetic click following the hold SHALL NOT start another recording

#### Scenario: Keyboard hold and release

* WHEN the focused microphone action receives a non-repeated Space or Enter keydown
* THEN it SHALL start recording
* AND the corresponding keyup SHALL finalize and transcribe

#### Scenario: Recording is cancelled

* WHEN the user presses Escape, pointer capture is cancelled/lost, the application window blurs before valid release, the composer is disposed, or the owning session changes
* THEN the recording SHALL be cancelled
* AND transcription SHALL not start implicitly
* AND the draft SHALL remain unchanged

#### Scenario: Recording is active

* WHEN the native host confirms active recording
* THEN the composer SHALL show an active pressed/recording visual, elapsed duration, local-processing indication, and an Escape-to-cancel hint
* AND screen readers SHALL not receive an elapsed-time announcement every second

#### Scenario: Recording reaches its maximum duration

* WHEN the native runtime auto-stops at the configured maximum
* THEN the composer SHALL transition to transcription
* AND it SHALL explain that the recording limit was reached
* AND it SHALL not discard the completed utterance

### Requirement: A final transcript SHALL append to the latest active draft without auto-send

After successful transcription, the composer controller SHALL re-read the latest draft for the originating composer scope and append the normalized final transcript. It SHALL not replace the draft snapshot from recording start and SHALL not call send.

#### Scenario: The user types during transcription

* WHEN the draft changes after recording release but before the final transcript arrives
* THEN the transcript SHALL append to that latest draft
* AND the intervening user text SHALL be preserved

#### Scenario: The draft is empty

* WHEN a non-empty transcript completes for the active scope and the latest draft is empty
* THEN the draft SHALL become the normalized transcript

#### Scenario: The draft does not end with whitespace

* WHEN a non-empty transcript is appended to non-empty text that does not end in whitespace
* THEN exactly one separating space SHALL be inserted before the transcript

#### Scenario: The draft ends with whitespace

* WHEN a non-empty transcript is appended to text that already ends in whitespace
* THEN no additional separator SHALL be inserted

#### Scenario: The session changes before completion

* WHEN the transcription result's `composerScopeId` no longer matches the active composer
* THEN the result SHALL NOT mutate the new or old session draft through the disposed controller
* AND it SHALL NOT be sent

#### Scenario: Transcription is empty or cancelled

* WHEN the result is `NO_SPEECH_DETECTED`, cancelled, failed, expired, or stale-scope
* THEN the draft SHALL remain unchanged

### Requirement: The text-to-speech action SHALL read the current selection or draft locally

The text-to-speech action SHALL synthesize the current textarea selection when it is non-empty; otherwise it SHALL synthesize the complete latest draft. It SHALL play locally, expose generating/playing/stop states, and SHALL not read assistant messages automatically.

#### Scenario: Text is selected

* WHEN the user activates TTS with a non-empty textarea selection
* THEN only the selected substring SHALL be submitted for local synthesis

#### Scenario: No text is selected

* WHEN the selection is empty and the draft is non-empty
* THEN the complete latest draft SHALL be submitted for local synthesis

#### Scenario: No source text exists

* WHEN both selection and draft are empty
* THEN the TTS action SHALL be disabled or return an accessible empty-state explanation
* AND no operation SHALL start

#### Scenario: Speech is playing

* WHEN native playback is active
* THEN the action SHALL expose a stop state with `aria-pressed=true`
* AND activating it SHALL stop playback

#### Scenario: Draft text changes

* WHEN the user edits the draft without activating the TTS control
* THEN synthesis SHALL NOT start automatically

#### Scenario: An assistant message arrives

* WHEN a new assistant message is rendered
* THEN this change SHALL NOT synthesize or play it automatically

### Requirement: Local-media actions SHALL be accessible and responsive

All local-media controls and result surfaces SHALL be keyboard-operable, localized, screen-reader-labelled, reduced-motion aware, and usable at the composer’s supported narrow widths.

#### Scenario: A keyboard-only user operates media controls

* WHEN the user navigates by Tab and activates OCR, microphone, TTS, dialog, append, cancel, or stop controls
* THEN every required action SHALL be reachable without a pointer
* AND focus SHALL return predictably after dialogs and successful insertion

#### Scenario: Status changes

* WHEN opening, recording, finalizing, transcribing, OCR, generating, playing, success, or failure state changes
* THEN one localized `aria-live="polite"` region SHALL announce the meaningful transition
* AND repeated elapsed-time ticks SHALL not create announcement spam

#### Scenario: Reduced motion is requested

* WHEN the operating system/browser reports `prefers-reduced-motion`
* THEN pulsing/animated recording indicators SHALL use a static active alternative

#### Scenario: The composer is narrow

* WHEN the action row reaches its supported compact breakpoint
* THEN controls SHALL wrap/collapse according to existing layout policy
* AND send/stop and all media actions SHALL remain reachable without overlap

### Requirement: Existing composer input and send semantics SHALL not regress

The integration SHALL preserve current IME composition handling, Enter/Shift+Enter semantics, slash/file-reference completion, prompt enhancement, textarea sizing/selection, and send/stop priority.

#### Scenario: An IME composition is active

* WHEN the user is composing text through an IME
* THEN media event handlers SHALL not cause the composition to submit or corrupt text

#### Scenario: Existing send behavior is used

* WHEN the user invokes the established send action or keyboard shortcut
* THEN its behavior SHALL remain unchanged by media readiness or media state except where the existing composer already blocks sending

#### Scenario: Media result is inserted

* WHEN OCR or STT updates the draft
* THEN slash-command and file-reference suggestion logic SHALL receive the same updated value through the existing setter path
