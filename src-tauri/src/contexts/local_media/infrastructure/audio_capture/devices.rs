//! Device enumeration and resolution.
//!
//! cpal exposes a persistable `DeviceId` with `Display`/`FromStr` and a `device_by_id` lookup,
//! which is exactly the shape a stored profile needs. `name()` is deprecated in favour of
//! `description()`, so the label and the identity come from different accessors on purpose: the
//! label is for the settings page, the id is what gets persisted and echoed in runtime status.

use std::str::FromStr;

use cpal::traits::{DeviceTrait, HostTrait};

use crate::contexts::local_media::application::ports::AudioDeviceCatalogPort;
use crate::contexts::local_media::domain::{AudioDevice, AudioDeviceCatalog, LocalMediaError};

const MAX_LABEL_CHARS: usize = 96;

/// A device's persistable identifier, or `None` when it disappeared mid-enumeration.
pub(super) fn device_id_for(device: &cpal::Device) -> Option<String> {
    device.id().ok().map(|id| id.to_string())
}

pub(super) fn safe_label(name: &str) -> String {
    let cleaned: String = name
        .chars()
        .filter(|character| !character.is_control())
        .take(MAX_LABEL_CHARS)
        .collect();
    let trimmed = cleaned.trim();
    if trimmed.is_empty() {
        "Unknown device".to_string()
    } else {
        trimmed.to_string()
    }
}

fn describe(device: &cpal::Device, default_id: Option<&str>) -> Option<AudioDevice> {
    let device_id = device_id_for(device)?;
    let label = device
        .description()
        .map(|description| safe_label(description.name()))
        .unwrap_or_else(|_| "Unknown device".to_string());
    Some(AudioDevice {
        is_default: default_id == Some(device_id.as_str()),
        label,
        device_id,
    })
}

pub(crate) struct CpalDeviceCatalog;

impl CpalDeviceCatalog {
    fn collect(
        devices: impl Iterator<Item = cpal::Device>,
        default_id: Option<String>,
    ) -> Vec<AudioDevice> {
        let mut collected = Vec::new();
        let mut seen = std::collections::BTreeSet::new();
        for device in devices {
            let Some(described) = describe(&device, default_id.as_deref()) else {
                continue;
            };
            if seen.insert(described.device_id.clone()) {
                collected.push(described);
            }
        }
        collected
    }
}

impl AudioDeviceCatalogPort for CpalDeviceCatalog {
    /// Enumerate without opening a stream.
    ///
    /// A host with no audio subsystem returns an empty catalog rather than an error: the Local
    /// media settings page has to render on a headless machine, and "no devices" is the honest
    /// answer there. A device that cannot be started still fails loudly at start time.
    fn catalog(&self) -> Result<AudioDeviceCatalog, LocalMediaError> {
        let host = cpal::default_host();
        let default_input = host.default_input_device().and_then(|d| device_id_for(&d));
        let default_output = host.default_output_device().and_then(|d| device_id_for(&d));
        let inputs = host
            .input_devices()
            .map(|devices| Self::collect(devices, default_input))
            .unwrap_or_default();
        let outputs = host
            .output_devices()
            .map(|devices| Self::collect(devices, default_output))
            .unwrap_or_default();
        Ok(AudioDeviceCatalog { inputs, outputs })
    }
}

fn device_by_id(device_id: &str) -> Option<cpal::Device> {
    let parsed = cpal::DeviceId::from_str(device_id).ok()?;
    cpal::default_host().device_by_id(&parsed)
}

/// Resolve a configured input device, falling back to the system default when none is configured.
///
/// A *configured* device that is no longer present returns `None` rather than falling back: a user
/// who selected a headset and then unplugged it should be told, not recorded through the laptop
/// microphone they were deliberately avoiding.
pub(super) fn resolve_input_device(device_id: Option<&str>) -> Option<cpal::Device> {
    let Some(device_id) = device_id.filter(|value| !value.trim().is_empty()) else {
        return cpal::default_host().default_input_device();
    };
    device_by_id(device_id)
}

/// Resolve a configured output device.
///
/// Unlike capture this does fall back to the default. Playing a preview through a different
/// speaker is a minor surprise; refusing to speak at all because a monitor was unplugged is worse.
pub(crate) fn resolve_output_device(device_id: Option<&str>) -> Option<cpal::Device> {
    let host = cpal::default_host();
    let Some(device_id) = device_id.filter(|value| !value.trim().is_empty()) else {
        return host.default_output_device();
    };
    device_by_id(device_id).or_else(|| host.default_output_device())
}

#[cfg(test)]
#[path = "devices_tests.rs"]
mod tests;
