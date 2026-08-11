#![cfg_attr(not(test), allow(dead_code))]

use super::{ValidatedOverlayPath, DEFAULT_OVERLAY_LIMITS};

const PROHIBITED_EXTENSIONS: &[&str] = &[
    "bat", "cmd", "com", "dll", "exe", "msi", "ps1", "py", "sh", "wasm",
];
const PROHIBITED_MEDIA_TYPES: &[&str] = &[
    "application/javascript",
    "application/vnd.microsoft.portable-executable",
    "application/wasm",
    "application/x-executable",
    "application/x-msdownload",
    "application/x-powershell",
    "application/x-sh",
    "text/javascript",
    "text/x-python",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum OverlayContentKind {
    Utf8Text,
    BinaryAsset,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum OverlayMediaError {
    MissingExtension,
    ProhibitedExtension,
    ProhibitedMediaType,
    UnsupportedExtension,
    MediaTypeMismatch,
    ExecutableSignature,
    InvalidUtf8,
    BinaryOutsideAssets,
    InvalidBinarySignature,
    BinaryTextReadDenied,
    TooLarge { maximum: u64, actual: u64 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ValidatedOverlayMedia {
    content_kind: OverlayContentKind,
}

impl ValidatedOverlayMedia {
    pub(crate) fn content_kind(self) -> OverlayContentKind {
        self.content_kind
    }

    pub(crate) fn text_content(self, content: &[u8]) -> Result<&str, OverlayMediaError> {
        if self.content_kind == OverlayContentKind::BinaryAsset {
            return Err(OverlayMediaError::BinaryTextReadDenied);
        }
        std::str::from_utf8(content).map_err(|_| OverlayMediaError::InvalidUtf8)
    }
}

pub(crate) fn validate_overlay_media(
    path: &ValidatedOverlayPath,
    declared_media_type: &str,
    content: &[u8],
) -> Result<ValidatedOverlayMedia, OverlayMediaError> {
    let extension = extension(path.as_str())?;
    let media_type = normalize_media_type(declared_media_type);

    if PROHIBITED_EXTENSIONS.contains(&extension.as_str()) {
        return Err(OverlayMediaError::ProhibitedExtension);
    }
    if PROHIBITED_MEDIA_TYPES
        .iter()
        .any(|prohibited| media_type.eq_ignore_ascii_case(prohibited))
    {
        return Err(OverlayMediaError::ProhibitedMediaType);
    }
    let actual = content.len() as u64;
    if actual > DEFAULT_OVERLAY_LIMITS.maximum_supporting_file_bytes {
        return Err(OverlayMediaError::TooLarge {
            maximum: DEFAULT_OVERLAY_LIMITS.maximum_supporting_file_bytes,
            actual,
        });
    }
    if has_executable_signature(content) {
        return Err(OverlayMediaError::ExecutableSignature);
    }

    if let Some(expected_media_types) = text_media_types(&extension) {
        require_media_type(media_type, expected_media_types)?;
        std::str::from_utf8(content).map_err(|_| OverlayMediaError::InvalidUtf8)?;
        return Ok(ValidatedOverlayMedia {
            content_kind: OverlayContentKind::Utf8Text,
        });
    }

    let binary_format = binary_format(&extension)?;
    if !path.as_str().starts_with("assets/") {
        return Err(OverlayMediaError::BinaryOutsideAssets);
    }
    require_media_type(media_type, binary_format.media_types)?;
    if !(binary_format.signature_matches)(content) {
        return Err(OverlayMediaError::InvalidBinarySignature);
    }
    Ok(ValidatedOverlayMedia {
        content_kind: OverlayContentKind::BinaryAsset,
    })
}

fn extension(path: &str) -> Result<String, OverlayMediaError> {
    path.rsplit('/')
        .next()
        .and_then(|file_name| file_name.rsplit_once('.'))
        .map(|(_, extension)| extension.to_ascii_lowercase())
        .filter(|extension| !extension.is_empty())
        .ok_or(OverlayMediaError::MissingExtension)
}

fn normalize_media_type(media_type: &str) -> &str {
    media_type
        .split_once(';')
        .map_or(media_type, |(value, _)| value)
        .trim()
}

fn require_media_type(actual: &str, expected: &[&str]) -> Result<(), OverlayMediaError> {
    if expected
        .iter()
        .any(|candidate| actual.eq_ignore_ascii_case(candidate))
    {
        Ok(())
    } else {
        Err(OverlayMediaError::MediaTypeMismatch)
    }
}

fn text_media_types(extension: &str) -> Option<&'static [&'static str]> {
    match extension {
        "md" => Some(&["text/markdown"]),
        "txt" => Some(&["text/plain"]),
        "json" => Some(&["application/json"]),
        "yaml" | "yml" => Some(&["application/yaml", "text/yaml"]),
        "toml" => Some(&["application/toml"]),
        "csv" => Some(&["text/csv"]),
        _ => None,
    }
}

struct BinaryFormat {
    media_types: &'static [&'static str],
    signature_matches: fn(&[u8]) -> bool,
}

fn binary_format(extension: &str) -> Result<BinaryFormat, OverlayMediaError> {
    match extension {
        "png" => Ok(BinaryFormat {
            media_types: &["image/png"],
            signature_matches: |content| content.starts_with(b"\x89PNG\r\n\x1a\n"),
        }),
        "jpg" | "jpeg" => Ok(BinaryFormat {
            media_types: &["image/jpeg"],
            signature_matches: |content| content.starts_with(&[0xff, 0xd8, 0xff]),
        }),
        "gif" => Ok(BinaryFormat {
            media_types: &["image/gif"],
            signature_matches: |content| {
                content.starts_with(b"GIF87a") || content.starts_with(b"GIF89a")
            },
        }),
        "webp" => Ok(BinaryFormat {
            media_types: &["image/webp"],
            signature_matches: |content| {
                content.starts_with(b"RIFF")
                    && content.get(8..12).is_some_and(|magic| magic == b"WEBP")
            },
        }),
        _ => Err(OverlayMediaError::UnsupportedExtension),
    }
}

fn has_executable_signature(content: &[u8]) -> bool {
    const SIGNATURES: &[&[u8]] = &[
        b"MZ",
        b"\x7fELF",
        b"\0asm",
        b"\xfe\xed\xfa\xce",
        b"\xfe\xed\xfa\xcf",
        b"\xce\xfa\xed\xfe",
        b"\xcf\xfa\xed\xfe",
        b"\xca\xfe\xba\xbe",
        b"\xd0\xcf\x11\xe0\xa1\xb1\x1a\xe1",
        b"#!",
    ];
    SIGNATURES
        .iter()
        .any(|signature| content.starts_with(signature))
}
