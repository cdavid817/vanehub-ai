use super::*;

fn png(width: u32, height: u32) -> Vec<u8> {
    let mut bytes = b"\x89PNG\r\n\x1a\n\x00\x00\x00\rIHDR".to_vec();
    bytes.extend_from_slice(&width.to_be_bytes());
    bytes.extend_from_slice(&height.to_be_bytes());
    bytes.extend_from_slice(&[8, 6, 0, 0, 0]);
    bytes
}

fn jpeg(width: u16, height: u16) -> Vec<u8> {
    let mut bytes = vec![0xff, 0xd8];
    // An APP0 segment first, so the walker has to skip a segment rather than find SOF0 immediately.
    bytes.extend_from_slice(&[0xff, 0xe0, 0x00, 0x10]);
    bytes.extend_from_slice(b"JFIF\0\x01\x02\x00\x00\x01\x00\x01\x00\x00");
    bytes.extend_from_slice(&[0xff, 0xc0, 0x00, 0x11, 0x08]);
    bytes.extend_from_slice(&height.to_be_bytes());
    bytes.extend_from_slice(&width.to_be_bytes());
    bytes.extend_from_slice(&[3, 1, 0x22, 0, 2, 0x11, 1, 3, 0x11, 1]);
    bytes
}

fn bmp(width: i32, height: i32) -> Vec<u8> {
    let mut bytes = vec![0u8; 26];
    bytes[0] = b'B';
    bytes[1] = b'M';
    bytes[18..22].copy_from_slice(&width.to_le_bytes());
    bytes[22..26].copy_from_slice(&height.to_le_bytes());
    bytes
}

#[test]
fn png_dimensions_come_from_the_ihdr_header() {
    assert_eq!(
        image_dimensions(SniffedFormat::Png, &png(1920, 1080)),
        Some((1920, 1080))
    );
    assert_eq!(
        image_dimensions(SniffedFormat::Png, &png(1, 1)),
        Some((1, 1))
    );
}

#[test]
fn jpeg_dimensions_come_from_the_first_start_of_frame() {
    assert_eq!(
        image_dimensions(SniffedFormat::Jpeg, &jpeg(800, 600)),
        Some((800, 600))
    );
}

#[test]
fn bmp_dimensions_treat_a_negative_height_as_top_down() {
    // A top-down BMP stores a negative height; its magnitude is still the pixel count.
    assert_eq!(
        image_dimensions(SniffedFormat::Bmp, &bmp(640, 480)),
        Some((640, 480))
    );
    assert_eq!(
        image_dimensions(SniffedFormat::Bmp, &bmp(640, -480)),
        Some((640, 480))
    );
}

#[test]
fn a_truncated_header_yields_no_dimensions() {
    assert_eq!(
        image_dimensions(SniffedFormat::Png, b"\x89PNG\r\n\x1a\n"),
        None
    );
    assert_eq!(image_dimensions(SniffedFormat::Jpeg, b"\xff\xd8\xff"), None);
    assert_eq!(image_dimensions(SniffedFormat::Bmp, b"BM\x00\x00"), None);
}

#[test]
fn a_pdf_reports_no_dimensions_because_pages_bound_it_instead() {
    assert_eq!(image_dimensions(SniffedFormat::Pdf, b"%PDF-1.7"), None);
}

#[test]
fn a_jpeg_with_no_start_of_frame_yields_no_dimensions() {
    // Rather than guessing, an unreadable header means the pixel bound cannot be enforced and the
    // file is refused upstream.
    assert_eq!(
        image_dimensions(SniffedFormat::Jpeg, b"\xff\xd8\xff\xd9"),
        None
    );
}

#[test]
fn segment_walking_terminates_on_a_malformed_length() {
    // A zero or one-byte segment length would make a naive walker loop forever on a hostile file.
    let hostile = vec![0xff, 0xd8, 0xff, 0xe0, 0x00, 0x00, 0xff, 0xe1, 0x00, 0x01];
    assert_eq!(image_dimensions(SniffedFormat::Jpeg, &hostile), None);
}

#[test]
fn a_zero_dimension_is_rejected() {
    assert_eq!(image_dimensions(SniffedFormat::Png, &png(0, 100)), None);
    assert_eq!(image_dimensions(SniffedFormat::Bmp, &bmp(100, 0)), None);
}

#[test]
fn the_pixel_bound_multiplies_without_overflowing() {
    assert!(exceeds_pixel_limit((70_000, 70_000), 50_000_000));
    assert!(!exceeds_pixel_limit((5_000, 5_000), 50_000_000));
    assert!(exceeds_pixel_limit((u32::MAX, u32::MAX), 50_000_000));
    assert!(!exceeds_pixel_limit((1, 1), 1));
}
