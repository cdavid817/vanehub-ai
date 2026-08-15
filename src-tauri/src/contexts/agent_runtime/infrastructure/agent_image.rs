//! Decode, validate, bound, and encode images the Agent is allowed to look at
//! (`add-agent-image-input`).
//!
//! One internal representation feeds both wire formats. The alternative -- an image path per
//! provider module -- would duplicate every bound check and every redaction rule, and the second
//! copy is the one that would drift.
//!
//! Format support is deliberately narrow: PNG covers screenshots and rendered PDF pages, JPEG
//! covers photos and scans. Every other codec `image`'s default features would enable is attack
//! surface for a format nothing in this product produces.

use base64::Engine;
use image::ImageFormat;
use sha2::{Digest, Sha256};

/// The long-edge ceiling. Matches what the providers themselves downscale to, so doing it here
/// makes the token cost predictable and the upload smaller without a second, lossier resize on
/// their side.
pub(crate) const MAX_IMAGE_EDGE_PIXELS: u32 = 1568;

/// Encoded-size ceiling, checked *after* downscaling. An image still over this is not one the
/// model was meant to read.
pub(crate) const MAX_IMAGE_BYTES: usize = 5 * 1024 * 1024;

/// Images one request may carry. Exceeding it is an error rather than a silent drop: answering a
/// question about the image that got dropped would be confident nonsense.
pub(crate) const MAX_IMAGES_PER_REQUEST: usize = 8;

/// Quality used when re-encoding a downscaled JPEG. High enough that text in a screenshot stays
/// legible, low enough that the re-encode does not undo the size win.
const JPEG_REENCODE_QUALITY: u8 = 85;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ImageMediaType {
    Png,
    Jpeg,
}

impl ImageMediaType {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::Png => "image/png",
            Self::Jpeg => "image/jpeg",
        }
    }

    fn from_format(format: ImageFormat) -> Option<Self> {
        match format {
            ImageFormat::Png => Some(Self::Png),
            ImageFormat::Jpeg => Some(Self::Jpeg),
            _ => None,
        }
    }

    /// Recognizes the media type a caller *declared*, including the common `image/jpg` spelling.
    pub(crate) fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "image/png" => Some(Self::Png),
            "image/jpeg" | "image/jpg" => Some(Self::Jpeg),
            _ => None,
        }
    }

    /// Whether a filesystem extension names a reviewed image type. Used by the file tool to decide
    /// an image read before it touches the bytes.
    pub(crate) fn from_extension(extension: &str) -> Option<Self> {
        match extension.trim().to_ascii_lowercase().as_str() {
            "png" => Some(Self::Png),
            "jpg" | "jpeg" => Some(Self::Jpeg),
            _ => None,
        }
    }

    const fn format(self) -> ImageFormat {
        match self {
            Self::Png => ImageFormat::Png,
            Self::Jpeg => ImageFormat::Jpeg,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ImageInputError {
    /// The bytes are not a reviewed image type, or are not a decodable image at all.
    UnsupportedFormat,
    /// The declared media type disagrees with what the bytes actually are.
    DeclaredTypeMismatch,
    /// Decoding failed even though the container looked like a reviewed type.
    Undecodable,
    /// Still over the byte ceiling after downscaling.
    TooLarge,
    /// Re-encoding a downscaled image failed.
    EncodeFailed,
}

impl ImageInputError {
    pub(crate) fn message(self) -> &'static str {
        match self {
            Self::UnsupportedFormat => {
                "Only PNG and JPEG images can be read. This file is neither."
            }
            Self::DeclaredTypeMismatch => {
                "The declared image type does not match the file's actual content."
            }
            Self::Undecodable => "This image could not be decoded.",
            Self::TooLarge => {
                "This image is still too large to send after downscaling. Crop it or reduce its resolution."
            }
            Self::EncodeFailed => "This image could not be prepared for sending.",
        }
    }
}

/// An image that has passed every check and is ready for either wire format.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AgentImage {
    media_type: ImageMediaType,
    data: Vec<u8>,
    width: u32,
    height: u32,
    downscaled: bool,
}

impl AgentImage {
    pub(crate) const fn media_type(&self) -> ImageMediaType {
        self.media_type
    }

    pub(crate) fn width(&self) -> u32 {
        self.width
    }

    pub(crate) fn height(&self) -> u32 {
        self.height
    }

    pub(crate) fn byte_len(&self) -> usize {
        self.data.len()
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) const fn was_downscaled(&self) -> bool {
        self.downscaled
    }

    pub(crate) fn base64(&self) -> String {
        base64::engine::general_purpose::STANDARD.encode(&self.data)
    }

    /// Content hash for durable logs. Logs carry this, the media type, the dimensions, and the
    /// byte count -- never the bytes, which for a single screenshot exceed the whole log line
    /// budget on their own.
    pub(crate) fn content_hash(&self) -> String {
        let mut hasher = Sha256::new();
        hasher.update(&self.data);
        // `sha2` 0.11's output type has no `LowerHex`, so hex is spelled out the same way the
        // delegation probes already spell it.
        const HEX: &[u8; 16] = b"0123456789abcdef";
        let digest = hasher.finalize();
        let mut encoded = String::with_capacity(digest.len() * 2);
        for byte in digest {
            encoded.push(char::from(HEX[usize::from(byte >> 4)]));
            encoded.push(char::from(HEX[usize::from(byte & 0x0f)]));
        }
        encoded
    }

    /// A caller-facing note when the image the model sees is not the original resolution. A model
    /// reasoning about pixel positions needs to know that.
    pub(crate) fn downscale_note(&self) -> Option<String> {
        self.downscaled.then(|| {
            format!(
                "[image downscaled to {}x{} before sending]",
                self.width, self.height
            )
        })
    }
}

/// Validates, decodes, bounds, and (when needed) downscales `bytes`.
///
/// `declared` is checked against the *content*, never trusted in place of it: a caller that says
/// "image/png" over JPEG bytes gets rejected rather than having a provider reject the whole
/// generation later.
pub(crate) fn prepare_image(
    bytes: &[u8],
    declared: Option<&str>,
) -> Result<AgentImage, ImageInputError> {
    let format = image::guess_format(bytes).map_err(|_| ImageInputError::UnsupportedFormat)?;
    let media_type =
        ImageMediaType::from_format(format).ok_or(ImageInputError::UnsupportedFormat)?;
    if let Some(declared) = declared {
        let declared =
            ImageMediaType::parse(declared).ok_or(ImageInputError::DeclaredTypeMismatch)?;
        if declared != media_type {
            return Err(ImageInputError::DeclaredTypeMismatch);
        }
    }

    let decoded = image::load_from_memory_with_format(bytes, format)
        .map_err(|_| ImageInputError::Undecodable)?;
    let (width, height) = (decoded.width(), decoded.height());
    let needs_downscale = width.max(height) > MAX_IMAGE_EDGE_PIXELS;

    if !needs_downscale && bytes.len() <= MAX_IMAGE_BYTES {
        return Ok(AgentImage {
            media_type,
            data: bytes.to_vec(),
            width,
            height,
            downscaled: false,
        });
    }

    // Over one bound or the other: downscaling is the only lever, so apply it even when only the
    // byte ceiling was exceeded -- re-encoding at the same dimensions would rarely be enough.
    let target = MAX_IMAGE_EDGE_PIXELS.min(width.max(height));
    let resized = decoded.resize(target, target, image::imageops::FilterType::Triangle);
    let data = encode(&resized, media_type)?;
    if data.len() > MAX_IMAGE_BYTES {
        return Err(ImageInputError::TooLarge);
    }
    Ok(AgentImage {
        media_type,
        width: resized.width(),
        height: resized.height(),
        data,
        downscaled: true,
    })
}

fn encode(
    image: &image::DynamicImage,
    media_type: ImageMediaType,
) -> Result<Vec<u8>, ImageInputError> {
    let mut data = Vec::new();
    let mut cursor = std::io::Cursor::new(&mut data);
    match media_type {
        // JPEG has no alpha channel, so an RGBA source has to be flattened before encoding or the
        // encoder rejects it outright.
        ImageMediaType::Jpeg => {
            let encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(
                &mut cursor,
                JPEG_REENCODE_QUALITY,
            );
            image
                .to_rgb8()
                .write_with_encoder(encoder)
                .map_err(|_| ImageInputError::EncodeFailed)?;
        }
        ImageMediaType::Png => {
            image
                .write_to(&mut cursor, media_type.format())
                .map_err(|_| ImageInputError::EncodeFailed)?;
        }
    }
    Ok(data)
}

#[cfg(test)]
#[path = "agent_image_tests.rs"]
mod tests;
