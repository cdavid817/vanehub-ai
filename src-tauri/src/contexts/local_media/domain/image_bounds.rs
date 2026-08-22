//! Header-only image dimensions.
//!
//! Deliberately not a decode. The point of the pixel ceiling is to refuse a decompression bomb
//! *before* anything allocates its pixels, so reading the declared dimensions out of a fixed-offset
//! header is not a shortcut -- it is the only order that works.
//!
//! PDFs are absent on purpose: page count, not pixel count, is their bound, and the only reliable
//! page count comes from a real parser. The worker enforces `maxPdfPages` and reports
//! `PDF_PAGE_LIMIT_EXCEEDED`; guessing here from `/Type /Page` occurrences would reject valid
//! documents whose content streams happen to contain that byte sequence.

use super::staged_input::SniffedFormat;

/// Declared pixel dimensions, or `None` when the header cannot be read.
///
/// `None` is a refusal upstream rather than a "skip the check": a file whose dimensions cannot be
/// established is a file whose pixel ceiling cannot be enforced.
pub(crate) fn image_dimensions(format: SniffedFormat, bytes: &[u8]) -> Option<(u32, u32)> {
    let dimensions = match format {
        SniffedFormat::Png => png_dimensions(bytes),
        SniffedFormat::Jpeg => jpeg_dimensions(bytes),
        SniffedFormat::Bmp => bmp_dimensions(bytes),
        SniffedFormat::Pdf => None,
    }?;
    if dimensions.0 == 0 || dimensions.1 == 0 {
        return None;
    }
    Some(dimensions)
}

/// Whether the declared dimensions exceed the ceiling, computed in 64 bits so a hostile
/// `u32::MAX x u32::MAX` header compares as too large instead of wrapping to something small.
pub(crate) fn exceeds_pixel_limit(dimensions: (u32, u32), limit: u64) -> bool {
    u64::from(dimensions.0).saturating_mul(u64::from(dimensions.1)) > limit
}

fn png_dimensions(bytes: &[u8]) -> Option<(u32, u32)> {
    // 8-byte signature, 4-byte length, "IHDR", then width and height.
    let width = bytes.get(16..20)?;
    let height = bytes.get(20..24)?;
    Some((read_be_u32(width)?, read_be_u32(height)?))
}

fn jpeg_dimensions(bytes: &[u8]) -> Option<(u32, u32)> {
    let mut index = 2usize;
    loop {
        // Markers may be preceded by any number of 0xFF fill bytes.
        while bytes.get(index) == Some(&0xff) && bytes.get(index + 1) == Some(&0xff) {
            index += 1;
        }
        if bytes.get(index)? != &0xff {
            return None;
        }
        let marker = *bytes.get(index + 1)?;
        // Every start-of-frame marker carries the dimensions in the same place. SOF4/SOF8/SOF12 are
        // not frame headers despite falling inside the range.
        let is_start_of_frame =
            (0xc0..=0xcf).contains(&marker) && !matches!(marker, 0xc4 | 0xc8 | 0xcc);
        let length = usize::from(read_be_u16(bytes.get(index + 2..index + 4)?)?);
        if length < 2 {
            // A malformed length would leave the cursor stationary; refusing beats looping.
            return None;
        }
        if is_start_of_frame {
            let height = read_be_u16(bytes.get(index + 5..index + 7)?)?;
            let width = read_be_u16(bytes.get(index + 7..index + 9)?)?;
            return Some((u32::from(width), u32::from(height)));
        }
        index = index.checked_add(2)?.checked_add(length)?;
        if index >= bytes.len() {
            return None;
        }
    }
}

fn bmp_dimensions(bytes: &[u8]) -> Option<(u32, u32)> {
    let width = read_le_i32(bytes.get(18..22)?)?;
    let height = read_le_i32(bytes.get(22..26)?)?;
    // A negative height marks a top-down bitmap; the row count is its magnitude.
    Some((width.unsigned_abs(), height.unsigned_abs()))
}

fn read_be_u32(slice: &[u8]) -> Option<u32> {
    Some(u32::from_be_bytes(slice.try_into().ok()?))
}

fn read_be_u16(slice: &[u8]) -> Option<u16> {
    Some(u16::from_be_bytes(slice.try_into().ok()?))
}

fn read_le_i32(slice: &[u8]) -> Option<i32> {
    Some(i32::from_le_bytes(slice.try_into().ok()?))
}

#[cfg(test)]
#[path = "image_bounds_tests.rs"]
mod tests;
