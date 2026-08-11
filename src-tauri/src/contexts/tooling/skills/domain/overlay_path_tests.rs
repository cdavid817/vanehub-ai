use super::{
    validate_overlay_link_target, validate_overlay_path, OverlayPathError, DEFAULT_OVERLAY_LIMITS,
};

#[test]
fn unix_windows_unc_and_device_absolute_paths_are_rejected() {
    for path in [
        "/references/team.md",
        "C:/references/team.md",
        "C:\\references\\team.md",
        "\\\\server\\share\\team.md",
        "//server/share/team.md",
        "\\\\?\\C:\\team.md",
    ] {
        assert_eq!(
            validate_overlay_path(path),
            Err(OverlayPathError::AbsolutePath),
            "absolute path was accepted: {path}"
        );
    }
}

#[test]
fn parent_traversal_and_ambiguous_separators_are_rejected_cross_platform() {
    for path in [
        "references/../secret.md",
        "../references/team.md",
        "references\\..\\secret.md",
    ] {
        assert_eq!(
            validate_overlay_path(path),
            Err(OverlayPathError::ParentTraversal)
        );
    }
    assert_eq!(
        validate_overlay_path("references\\team.md"),
        Err(OverlayPathError::NonCanonicalSeparator)
    );
}

#[test]
fn hidden_components_reserved_devices_and_alternate_streams_are_rejected() {
    for path in ["references/.private/team.md", ".references/team.md"] {
        assert_eq!(
            validate_overlay_path(path),
            Err(OverlayPathError::HiddenComponent)
        );
    }
    for path in [
        "references/CON",
        "references/con.txt",
        "templates/AUX.md",
        "assets/COM1.png",
        "assets/lpt9.jpg",
    ] {
        assert_eq!(
            validate_overlay_path(path),
            Err(OverlayPathError::ReservedDevice)
        );
    }
    assert_eq!(
        validate_overlay_path("references/team.md:secret"),
        Err(OverlayPathError::AlternateDataStream)
    );
}

#[test]
fn only_supported_top_level_directories_and_bounded_paths_are_accepted() {
    assert_eq!(
        validate_overlay_path("scripts/tool.md"),
        Err(OverlayPathError::UnsupportedTopLevel)
    );
    assert_eq!(
        validate_overlay_path("references"),
        Err(OverlayPathError::MissingFileName)
    );
    let overlong = format!(
        "references/{}.md",
        "x".repeat(DEFAULT_OVERLAY_LIMITS.maximum_path_characters)
    );
    assert!(matches!(
        validate_overlay_path(&overlong),
        Err(OverlayPathError::TooLong { .. })
    ));
    let too_deep = format!(
        "references/{}/team.md",
        (0..DEFAULT_OVERLAY_LIMITS.maximum_path_depth)
            .map(|index| format!("level-{index}"))
            .collect::<Vec<_>>()
            .join("/")
    );
    assert!(matches!(
        validate_overlay_path(&too_deep),
        Err(OverlayPathError::TooDeep { .. })
    ));

    for path in [
        "references/team-guidance.md",
        "templates/report/summary.md",
        "assets/images/diagram.png",
    ] {
        assert_eq!(
            validate_overlay_path(path)
                .expect("valid Overlay path")
                .as_str(),
            path
        );
    }
}

#[test]
fn links_must_resolve_inside_an_allowed_overlay_resource_directory() {
    assert_eq!(
        validate_overlay_link_target("references/nested/link.md", "../../../outside.md"),
        Err(OverlayPathError::LinkEscape)
    );
    assert_eq!(
        validate_overlay_link_target("references/link.md", "C:/outside.md"),
        Err(OverlayPathError::AbsolutePath)
    );
    assert_eq!(
        validate_overlay_link_target("references/nested/link.md", "../team.md")
            .expect("in-bound link")
            .as_str(),
        "references/team.md"
    );
}
