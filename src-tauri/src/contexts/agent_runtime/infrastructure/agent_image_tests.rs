use super::*;
use image::{DynamicImage, RgbImage, RgbaImage};

fn png_bytes(width: u32, height: u32) -> Vec<u8> {
    let mut data = Vec::new();
    DynamicImage::ImageRgba8(RgbaImage::new(width, height))
        .write_to(&mut std::io::Cursor::new(&mut data), ImageFormat::Png)
        .expect("encode png fixture");
    data
}

fn jpeg_bytes(width: u32, height: u32) -> Vec<u8> {
    let mut data = Vec::new();
    DynamicImage::ImageRgb8(RgbImage::new(width, height))
        .write_to(&mut std::io::Cursor::new(&mut data), ImageFormat::Jpeg)
        .expect("encode jpeg fixture");
    data
}

#[test]
fn a_small_png_passes_through_byte_identical() {
    let bytes = png_bytes(64, 32);
    let prepared = prepare_image(&bytes, None).expect("prepare");

    assert_eq!(prepared.media_type(), ImageMediaType::Png);
    assert_eq!((prepared.width(), prepared.height()), (64, 32));
    assert!(!prepared.was_downscaled());
    assert_eq!(prepared.byte_len(), bytes.len());
    assert_eq!(prepared.downscale_note(), None);
}

#[test]
fn a_small_jpeg_is_recognized_from_its_content() {
    let prepared = prepare_image(&jpeg_bytes(48, 48), None).expect("prepare");
    assert_eq!(prepared.media_type(), ImageMediaType::Jpeg);
    assert!(!prepared.was_downscaled());
}

#[test]
fn an_oversized_image_is_downscaled_and_says_so() {
    let bytes = png_bytes(MAX_IMAGE_EDGE_PIXELS + 400, 200);
    let prepared = prepare_image(&bytes, None).expect("prepare");

    assert!(prepared.was_downscaled());
    assert_eq!(prepared.width(), MAX_IMAGE_EDGE_PIXELS);
    assert!(prepared.width().max(prepared.height()) <= MAX_IMAGE_EDGE_PIXELS);
    // Aspect ratio survives the resize rather than the image being squashed to a square.
    assert!(prepared.height() < prepared.width());
    let note = prepared.downscale_note().expect("a downscale note");
    assert!(note.contains(&MAX_IMAGE_EDGE_PIXELS.to_string()), "{note}");
}

#[test]
fn an_image_exactly_at_the_edge_bound_is_not_downscaled() {
    let prepared = prepare_image(&png_bytes(MAX_IMAGE_EDGE_PIXELS, 10), None).expect("prepare");
    assert!(!prepared.was_downscaled());
    assert_eq!(prepared.width(), MAX_IMAGE_EDGE_PIXELS);
}

/// JPEG has no alpha channel, so a downscaled RGBA source has to be flattened before encoding --
/// otherwise the encoder rejects it and a legitimate image turns into a failure.
#[test]
fn a_downscaled_image_with_an_alpha_channel_still_encodes() {
    let mut data = Vec::new();
    DynamicImage::ImageRgba8(RgbaImage::new(MAX_IMAGE_EDGE_PIXELS + 100, 100))
        .write_to(&mut std::io::Cursor::new(&mut data), ImageFormat::Png)
        .expect("encode rgba png");
    let prepared = prepare_image(&data, None).expect("an RGBA source must survive downscaling");
    assert!(prepared.was_downscaled());
    assert_eq!(prepared.media_type(), ImageMediaType::Png);
}

#[test]
fn a_declared_type_that_disagrees_with_the_content_is_rejected() {
    let jpeg = jpeg_bytes(32, 32);
    assert_eq!(
        prepare_image(&jpeg, Some("image/png")),
        Err(ImageInputError::DeclaredTypeMismatch)
    );
    // The agreeing case still passes, including the common `image/jpg` spelling.
    assert!(prepare_image(&jpeg, Some("image/jpeg")).is_ok());
    assert!(prepare_image(&jpeg, Some("image/jpg")).is_ok());
    assert!(prepare_image(&jpeg, Some("IMAGE/JPEG")).is_ok());
}

#[test]
fn an_unreviewed_declared_type_is_rejected_without_conversion() {
    assert_eq!(
        prepare_image(&png_bytes(8, 8), Some("image/webp")),
        Err(ImageInputError::DeclaredTypeMismatch)
    );
}

#[test]
fn non_image_and_unreviewed_image_bytes_are_rejected() {
    for bytes in [b"not an image at all".to_vec(), Vec::new(), vec![0_u8; 64]] {
        assert_eq!(
            prepare_image(&bytes, None),
            Err(ImageInputError::UnsupportedFormat),
            "expected rejection for {} bytes",
            bytes.len()
        );
    }
}

/// GIF is a real image format the `image` crate can identify but this product does not review, so
/// it is refused rather than converted into something reviewed.
#[test]
fn a_recognizable_but_unreviewed_format_is_refused() {
    let mut gif = b"GIF89a".to_vec();
    gif.extend_from_slice(&[0x01, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00]);
    assert_eq!(
        prepare_image(&gif, None),
        Err(ImageInputError::UnsupportedFormat)
    );
}

#[test]
fn media_types_round_trip_through_their_wire_strings() {
    assert_eq!(ImageMediaType::Png.as_str(), "image/png");
    assert_eq!(ImageMediaType::Jpeg.as_str(), "image/jpeg");
    assert_eq!(
        ImageMediaType::parse("image/png"),
        Some(ImageMediaType::Png)
    );
    assert_eq!(ImageMediaType::parse("image/gif"), None);
    assert_eq!(ImageMediaType::parse(""), None);
}

#[test]
fn extensions_map_only_to_reviewed_types() {
    assert_eq!(
        ImageMediaType::from_extension("PNG"),
        Some(ImageMediaType::Png)
    );
    for jpeg in ["jpg", "jpeg", "JPEG"] {
        assert_eq!(
            ImageMediaType::from_extension(jpeg),
            Some(ImageMediaType::Jpeg)
        );
    }
    for other in ["webp", "gif", "bmp", "svg", "txt", ""] {
        assert_eq!(ImageMediaType::from_extension(other), None, "{other}");
    }
}

#[test]
fn the_content_hash_identifies_the_bytes_that_were_sent() {
    let first = prepare_image(&png_bytes(16, 16), None).expect("prepare");
    let same = prepare_image(&png_bytes(16, 16), None).expect("prepare");
    let different = prepare_image(&png_bytes(16, 32), None).expect("prepare");

    assert_eq!(first.content_hash(), same.content_hash());
    assert_ne!(first.content_hash(), different.content_hash());
    assert_eq!(first.content_hash().len(), 64);
}

#[test]
fn base64_encodes_the_prepared_bytes() {
    let prepared = prepare_image(&png_bytes(8, 8), None).expect("prepare");
    let encoded = prepared.base64();
    let decoded = base64::engine::general_purpose::STANDARD
        .decode(&encoded)
        .expect("decode");
    assert_eq!(decoded.len(), prepared.byte_len());
    assert!(!encoded.contains('\n'), "wire payloads must not be wrapped");
}

#[test]
fn every_error_carries_an_actionable_message() {
    for error in [
        ImageInputError::UnsupportedFormat,
        ImageInputError::DeclaredTypeMismatch,
        ImageInputError::Undecodable,
        ImageInputError::TooLarge,
        ImageInputError::EncodeFailed,
    ] {
        assert!(!error.message().is_empty());
    }
}
