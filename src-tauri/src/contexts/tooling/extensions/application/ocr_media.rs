use super::{OcrAdmissionError, OcrAdmissionLimits};

pub(super) fn validate_image_dimensions(
    media_type: &str,
    bytes: &[u8],
    limits: OcrAdmissionLimits,
) -> Result<(), OcrAdmissionError> {
    let dimensions = match media_type {
        "image/png" => png_dimensions(bytes),
        "image/jpeg" => jpeg_dimensions(bytes),
        _ => return Ok(()),
    }
    .ok_or(OcrAdmissionError::IntegrityFailure)?;
    let (width, height) = dimensions;
    let pixels = u64::from(width).saturating_mul(u64::from(height));
    if width == 0
        || height == 0
        || width > limits.image_width
        || height > limits.image_height
        || pixels > limits.image_pixels
    {
        Err(OcrAdmissionError::LimitExceeded)
    } else {
        Ok(())
    }
}

fn png_dimensions(bytes: &[u8]) -> Option<(u32, u32)> {
    if bytes.len() < 24 || !bytes.starts_with(b"\x89PNG\r\n\x1a\n") || &bytes[12..16] != b"IHDR" {
        return None;
    }
    Some((
        u32::from_be_bytes(bytes[16..20].try_into().ok()?),
        u32::from_be_bytes(bytes[20..24].try_into().ok()?),
    ))
}

fn jpeg_dimensions(bytes: &[u8]) -> Option<(u32, u32)> {
    if !bytes.starts_with(&[0xff, 0xd8, 0xff]) {
        return None;
    }
    let mut offset = 2_usize;
    while offset + 4 <= bytes.len() {
        while offset < bytes.len() && bytes[offset] == 0xff {
            offset += 1;
        }
        let marker = *bytes.get(offset)?;
        offset += 1;
        if matches!(marker, 0xd8 | 0xd9) {
            continue;
        }
        let length = usize::from(u16::from_be_bytes([
            *bytes.get(offset)?,
            *bytes.get(offset + 1)?,
        ]));
        if length < 2 || offset + length > bytes.len() {
            return None;
        }
        if matches!(
            marker,
            0xc0 | 0xc1
                | 0xc2
                | 0xc3
                | 0xc5
                | 0xc6
                | 0xc7
                | 0xc9
                | 0xca
                | 0xcb
                | 0xcd
                | 0xce
                | 0xcf
        ) {
            let height = u32::from(u16::from_be_bytes([
                *bytes.get(offset + 3)?,
                *bytes.get(offset + 4)?,
            ]));
            let width = u32::from(u16::from_be_bytes([
                *bytes.get(offset + 5)?,
                *bytes.get(offset + 6)?,
            ]));
            return Some((width, height));
        }
        offset += length;
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn png_dimensions_are_checked_before_staging() {
        let mut png = b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR".to_vec();
        png.extend_from_slice(&20_000_u32.to_be_bytes());
        png.extend_from_slice(&10_u32.to_be_bytes());
        assert_eq!(
            validate_image_dimensions("image/png", &png, OcrAdmissionLimits::HARD_CEILING),
            Err(OcrAdmissionError::LimitExceeded)
        );
    }
}
