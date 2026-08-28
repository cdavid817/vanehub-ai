use super::*;

#[test]
fn a_label_is_bounded_and_free_of_control_characters() {
    assert_eq!(safe_label("Microphone"), "Microphone");
    assert_eq!(safe_label("Micro\nphone"), "Microphone");
    assert_eq!(safe_label(""), "Unknown device");
    assert_eq!(safe_label("   "), "Unknown device");
    let long = "d".repeat(200);
    assert!(safe_label(&long).chars().count() <= 96);
}

#[test]
fn labels_truncate_on_character_boundaries() {
    let long = "設".repeat(200);
    let label = safe_label(&long);
    assert!(label.chars().count() <= 96);
    assert!(label.chars().all(|character| character == '設'));
}

#[test]
fn enumerating_devices_never_panics_on_this_machine() {
    let _audio_host = crate::contexts::local_media::infrastructure::audio_host_guard();
    // CI runners frequently have no audio device at all. An empty catalog is the correct outcome;
    // a panic or an error would make the Local media settings page unopenable on a headless host.
    let catalog = CpalDeviceCatalog.catalog().expect("catalog must not error");
    for device in catalog.inputs.iter().chain(catalog.outputs.iter()) {
        assert!(!device.device_id.is_empty());
        assert!(!device.label.is_empty());
        // cpal renders an id as `host:device`. A bare device string would not round-trip through
        // `DeviceId::from_str`, and the stored profile would silently stop resolving.
        assert!(
            device.device_id.contains(':'),
            "unexpected id shape {}",
            device.device_id
        );
    }
    assert!(
        catalog
            .inputs
            .iter()
            .filter(|device| device.is_default)
            .count()
            <= 1
    );
    assert!(
        catalog
            .outputs
            .iter()
            .filter(|device| device.is_default)
            .count()
            <= 1
    );
}

#[test]
fn device_ids_are_unique_within_a_catalog() {
    let _audio_host = crate::contexts::local_media::infrastructure::audio_host_guard();
    let catalog = CpalDeviceCatalog.catalog().expect("catalog");
    let unique: std::collections::BTreeSet<&str> = catalog
        .inputs
        .iter()
        .map(|device| device.device_id.as_str())
        .collect();
    assert_eq!(unique.len(), catalog.inputs.len());
}

#[test]
fn an_unresolvable_configured_input_does_not_silently_fall_back() {
    let _audio_host = crate::contexts::local_media::infrastructure::audio_host_guard();
    // Recording through a different microphone than the one the user chose is worse than failing.
    assert!(resolve_input_device(Some("nosuchhost:nosuchdevice")).is_none());
}

#[test]
fn a_malformed_device_id_resolves_to_nothing_rather_than_the_default() {
    let _audio_host = crate::contexts::local_media::infrastructure::audio_host_guard();
    assert!(resolve_input_device(Some("not-an-id")).is_none());
}

#[test]
fn an_empty_configured_id_means_the_system_default() {
    let _audio_host = crate::contexts::local_media::infrastructure::audio_host_guard();
    // Whether a default exists depends on the machine; what matters is that a blank stored value is
    // treated as "unset" rather than as an id that will never resolve.
    let explicit = resolve_input_device(Some("   ")).is_some();
    let implicit = resolve_input_device(None).is_some();
    assert_eq!(explicit, implicit);
}

#[test]
fn an_unresolvable_configured_output_falls_back_to_the_default() {
    let _audio_host = crate::contexts::local_media::infrastructure::audio_host_guard();
    let fallback = resolve_output_device(Some("nosuchhost:nosuchdevice")).is_some();
    let default = resolve_output_device(None).is_some();
    assert_eq!(fallback, default);
}
