//! Deterministic stand-ins for the hardware and interaction boundaries CI cannot provide.
//!
//! Compiled only under `desktop-e2e`, and even then assembled only when the runtime variable is
//! set, so a build without the feature contains none of these types. Everything *between* the
//! composer and the engine stays production code: the commands, `LocalMediaApi`, the application
//! service, the operation store, the temp store, the supervisor, the launcher, the child-process
//! transport, the JSON Lines protocol, and every cancel, grace, crash and restart rule.
//!
//! What is replaced here is only what a headless runner genuinely cannot supply: a microphone, a
//! speaker, a device enumerator, and a human choosing a file.

mod scenario;

pub(crate) mod audio;

pub(crate) use audio::{FixtureAudioCapture, FixtureAudioDeviceCatalog, FixtureAudioPlayback};
pub(crate) use scenario::{fixture_activation, FixtureActivation};
