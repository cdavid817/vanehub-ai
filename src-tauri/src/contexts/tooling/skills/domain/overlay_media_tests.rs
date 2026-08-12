use super::{
    validate_overlay_media, validate_overlay_path, OverlayContentKind, OverlayMediaError,
    DEFAULT_OVERLAY_LIMITS,
};

fn validate(
    path: &str,
    media_type: &str,
    content: &[u8],
) -> Result<super::ValidatedOverlayMedia, OverlayMediaError> {
    let path = validate_overlay_path(path).expect("test path must be valid");
    validate_overlay_media(&path, media_type, content)
}

#[test]
fn prohibited_extensions_and_executable_media_types_are_rejected_first() {
    for extension in [
        "py", "SH", "bat", "cmd", "ps1", "exe", "com", "dll", "msi", "wasm",
    ] {
        let path = format!("assets/tool.{extension}");
        assert_eq!(
            validate(&path, "text/plain", b"safe-looking text"),
            Err(OverlayMediaError::ProhibitedExtension)
        );
    }

    assert_eq!(
        validate(
            "references/guide.md",
            "Application/X-MsDownload; charset=binary",
            b"ordinary guidance"
        ),
        Err(OverlayMediaError::ProhibitedMediaType)
    );
}

#[test]
fn disguised_executable_signatures_are_rejected_before_format_checks() {
    for signature in [
        b"MZpayload".as_slice(),
        b"\x7fELFpayload".as_slice(),
        b"\0asmpayload".as_slice(),
        b"\xfe\xed\xfa\xcfpayload".as_slice(),
        b"#! /bin/sh\necho unsafe".as_slice(),
    ] {
        assert_eq!(
            validate("assets/disguised.png", "image/png", signature),
            Err(OverlayMediaError::ExecutableSignature)
        );
    }
}

#[test]
fn valid_utf8_documents_are_readable_but_invalid_utf8_is_rejected() {
    let content = "# 指南\n只保存确定性指导。".as_bytes();
    let media =
        validate("references/guide.md", "text/markdown", content).expect("valid UTF-8 Markdown");
    assert_eq!(media.content_kind(), OverlayContentKind::Utf8Text);
    assert_eq!(
        media.text_content(content).expect("text content"),
        "# 指南\n只保存确定性指导。"
    );

    assert_eq!(
        validate("templates/invalid.txt", "text/plain", &[0xff, 0xfe]),
        Err(OverlayMediaError::InvalidUtf8)
    );
}

#[test]
fn bounded_binary_assets_are_accepted_but_not_exposed_as_text() {
    let png = b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR";
    let media = validate("assets/diagram.png", "image/png", png)
        .expect("bounded PNG asset should be accepted");
    assert_eq!(media.content_kind(), OverlayContentKind::BinaryAsset);
    assert_eq!(
        media.text_content(png),
        Err(OverlayMediaError::BinaryTextReadDenied)
    );

    let oversized = vec![0_u8; DEFAULT_OVERLAY_LIMITS.maximum_supporting_file_bytes as usize + 1];
    assert!(matches!(
        validate("assets/large.png", "image/png", &oversized),
        Err(OverlayMediaError::TooLarge { .. })
    ));
}

#[test]
fn binary_assets_require_asset_scope_and_extension_media_signature_agreement() {
    let png = b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR";
    assert_eq!(
        validate("references/diagram.png", "image/png", png),
        Err(OverlayMediaError::BinaryOutsideAssets)
    );
    assert_eq!(
        validate("assets/diagram.png", "image/jpeg", png),
        Err(OverlayMediaError::MediaTypeMismatch)
    );
    assert_eq!(
        validate("assets/diagram.png", "image/png", b"not a PNG"),
        Err(OverlayMediaError::InvalidBinarySignature)
    );
}
