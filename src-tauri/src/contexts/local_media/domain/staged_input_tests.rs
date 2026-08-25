use super::*;

const PNG: &[u8] = b"\x89PNG\r\n\x1a\n\x00\x00\x00\rIHDR";
const JPEG: &[u8] = b"\xff\xd8\xff\xe0\x00\x10JFIF";
const PDF: &[u8] = b"%PDF-1.7\n1 0 obj";
const BMP: &[u8] =
    b"BM\x36\x10\x00\x00\x00\x00\x00\x00\x36\x00\x00\x00\x28\x00\x00\x00\x40\x00\x00\x00\x40\x00\x00\x00";
const GIF: &[u8] = b"GIF89a\x01\x00";

#[test]
fn supported_formats_are_recognized_from_their_magic_bytes() {
    assert_eq!(sniff_media(PNG), Some(SniffedFormat::Png));
    assert_eq!(sniff_media(JPEG), Some(SniffedFormat::Jpeg));
    assert_eq!(sniff_media(PDF), Some(SniffedFormat::Pdf));
    assert_eq!(sniff_media(BMP), Some(SniffedFormat::Bmp));
}

#[test]
fn formats_whose_dimensions_cannot_be_bounded_cheaply_are_not_admitted() {
    // TIFF and WEBP are perfectly readable by PaddleOCR, but admission has to enforce a decoded
    // pixel ceiling and neither exposes its dimensions at a fixed offset. Refusing them keeps the
    // ceiling real instead of nominally configured and never checked.
    assert_eq!(sniff_media(b"II\x2a\x00rest"), None);
    assert_eq!(sniff_media(b"MM\x00\x2arest"), None);
    assert_eq!(sniff_media(b"RIFF\x24\x00\x00\x00WEBPVP8 "), None);
    assert_eq!(sniff_media(b"RIFF\x24\x00\x00\x00WAVEfmt "), None);
}

#[test]
fn a_bmp_without_its_dib_header_is_not_admitted() {
    // The two-byte signature alone is not enough: dimensions live at offsets 18 and 22.
    assert_eq!(sniff_media(b"BM\x36\x00\x00\x00"), None);
}

#[test]
fn an_unsupported_format_is_rejected_rather_than_guessed() {
    assert_eq!(sniff_media(GIF), None);
    assert_eq!(sniff_media(b"<html><body>"), None);
    assert_eq!(sniff_media(b"MZ\x90\x00"), None);
    assert_eq!(sniff_media(b""), None);
    assert_eq!(sniff_media(b"%PD"), None);
}

#[test]
fn a_misleading_extension_cannot_override_the_sniffed_type() {
    // The caller may have named it `.png`; the bytes are a PDF and that is what governs.
    assert_eq!(
        sniff_media(PDF).map(SniffedFormat::media_type),
        Some(OcrMediaType::Pdf)
    );
    assert_eq!(
        sniff_media(PNG).map(SniffedFormat::media_type),
        Some(OcrMediaType::Image)
    );
}

#[test]
fn every_image_format_maps_to_the_image_media_type() {
    for format in [SniffedFormat::Png, SniffedFormat::Jpeg, SniffedFormat::Bmp] {
        assert_eq!(format.media_type(), OcrMediaType::Image);
    }
    assert_eq!(SniffedFormat::Pdf.media_type(), OcrMediaType::Pdf);
}

#[test]
fn display_names_keep_only_the_final_component_on_either_separator() {
    // `Path::file_name` on Linux does not treat a backslash as a separator, so both are stripped
    // explicitly. Otherwise a Windows path pasted on a Linux host would be shown in full.
    assert_eq!(sanitize_display_name("/home/user/scan.png"), "scan.png");
    assert_eq!(
        sanitize_display_name("C:\\Users\\someone\\scan.png"),
        "scan.png"
    );
    assert_eq!(sanitize_display_name("scan.png"), "scan.png");
    assert_eq!(sanitize_display_name("/a/b/"), "file");
}

#[test]
fn display_names_are_bounded_and_stripped_of_control_characters() {
    let long = format!("{}.png", "n".repeat(400));
    let sanitized = sanitize_display_name(&long);
    assert!(
        sanitized.chars().count() <= 120,
        "got {} chars",
        sanitized.chars().count()
    );
    assert_eq!(sanitize_display_name("na\nme.png"), "name.png");
    assert_eq!(sanitize_display_name(""), "file");
    assert_eq!(sanitize_display_name("   "), "file");
}

#[test]
fn display_names_truncate_on_character_boundaries() {
    let long = "文".repeat(400);
    let sanitized = sanitize_display_name(&long);
    assert!(sanitized.chars().count() <= 120);
    assert!(sanitized.chars().all(|character| character == '文'));
}

#[test]
fn a_freshly_staged_input_is_unclaimed_and_unexpired() {
    let record = StagedInputRecord::new(
        StagedInputId::new("lmi-0123456789abcdef0123456789abcdef"),
        StagedOcrSource {
            staged_input_id: StagedInputId::new("lmi-0123456789abcdef0123456789abcdef"),
            display_name: "scan.png".to_string(),
            media_type: OcrMediaType::Image,
            byte_length: 2048,
        },
        1_000,
    );
    assert!(!record.is_claimed());
    assert!(!record.is_expired(1_000));
    assert!(!record.is_expired(1_000 + STAGED_INPUT_TTL_MS - 1));
    assert!(record.is_expired(1_000 + STAGED_INPUT_TTL_MS));
}

#[test]
fn claiming_a_staged_input_is_a_one_time_transition() {
    let id = StagedInputId::new("lmi-0123456789abcdef0123456789abcdef");
    let mut record = StagedInputRecord::new(
        id.clone(),
        StagedOcrSource {
            staged_input_id: id,
            display_name: "scan.pdf".to_string(),
            media_type: OcrMediaType::Pdf,
            byte_length: 4096,
        },
        0,
    );
    assert!(record.claim(0).is_ok());
    assert!(record.is_claimed());
    // The second claim is the race this guards: two operations must not share one staged file.
    assert_eq!(record.claim(0), Err(LocalMediaErrorCode::InputNotFound));
}

#[test]
fn an_expired_staged_input_cannot_be_claimed() {
    let id = StagedInputId::new("lmi-0123456789abcdef0123456789abcdef");
    let mut record = StagedInputRecord::new(
        id.clone(),
        StagedOcrSource {
            staged_input_id: id,
            display_name: "scan.png".to_string(),
            media_type: OcrMediaType::Image,
            byte_length: 1,
        },
        0,
    );
    assert_eq!(
        record.claim(STAGED_INPUT_TTL_MS),
        Err(LocalMediaErrorCode::InputNotFound)
    );
    assert!(!record.is_claimed());
}

#[test]
fn the_staged_source_serializes_without_a_path() {
    let id = StagedInputId::new("lmi-0123456789abcdef0123456789abcdef");
    let source = StagedOcrSource {
        staged_input_id: id,
        display_name: "scan.png".to_string(),
        media_type: OcrMediaType::Image,
        byte_length: 2048,
    };
    let json = serde_json::to_value(&source).expect("serialize");
    assert_eq!(json["mediaType"], "image");
    assert_eq!(json["byteLength"], 2048);
    assert_eq!(json["displayName"], "scan.png");
    // The exact key set matters more than its order: anything beyond these four would be a field
    // the frontend was not meant to receive, and a path is the one most likely to be added.
    let keys: std::collections::BTreeSet<&str> = json
        .as_object()
        .expect("object")
        .keys()
        .map(String::as_str)
        .collect();
    assert_eq!(
        keys,
        ["byteLength", "displayName", "mediaType", "stagedInputId"]
            .into_iter()
            .collect()
    );
}
