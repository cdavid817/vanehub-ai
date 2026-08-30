//! The one SHA-256 digest implementation. Every skill-evolution context previously carried its
//! own copy of this loop with slightly different names, which is how encoding drift starts; the
//! contexts keep their own thin wrappers (formats differ deliberately: bare hex for identity
//! material, `sha256:<hex>` for witnesses) but all delegate here.

use sha2::{Digest, Sha256};

/// Lowercase hex rendering of raw bytes (for incrementally built digests).
pub(crate) fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

/// Lowercase hex digest of the input bytes.
pub(crate) fn sha256_hex(bytes: impl AsRef<[u8]>) -> String {
    hex(&Sha256::digest(bytes.as_ref()))
}

/// The repository-standard `sha256:<hex>` witness form.
pub(crate) fn sha256_tagged(bytes: impl AsRef<[u8]>) -> String {
    format!("sha256:{}", sha256_hex(bytes))
}
