// No production caller yet; see `identity.rs`.
#![cfg_attr(not(test), allow(dead_code))]

//! Exactly what a publisher's signature covers.
//!
//! Not the envelope bytes. Signing the file as written would make the signature depend on
//! whitespace and field order, so a byte-identical package could fail to verify after any
//! reformatting, and — worse — two different-looking envelopes could be made to carry the same
//! meaning while only one verifies. The signature covers the *values*, canonically encoded.
//!
//! Two properties this encoding has to have, and one it has to refuse:
//!
//! * **Domain separation.** The payload opens with a context string naming this format. An Ed25519
//!   key is just a key; without a context, a signature the publisher produced for some unrelated
//!   protocol could be replayed here if the byte strings ever coincided.
//! * **Every covered field, unambiguously.** Length-prefixed via the shared canonical encoder, so
//!   no two different sets of values encode identically.
//! * **The signature itself is not covered.** It cannot be — and saying so here keeps the next
//!   reader from "fixing" the omission.

use super::canonical::Canonical;
use super::SignatureEnvelope;

/// Names this format, and only this format.
const SIGNING_CONTEXT: &str = "vanehub.extension-platform.package-signature.v1";

/// The bytes a publisher signs and this application verifies.
pub(crate) fn signed_payload(envelope: &SignatureEnvelope) -> Vec<u8> {
    let mut canonical = Canonical::default();
    canonical.tag(SIGNING_CONTEXT);

    canonical.tag("envelope_version");
    canonical.text(&envelope.envelope_version.to_string());
    canonical.tag("algorithm");
    canonical.text(envelope.algorithm.as_str());
    canonical.tag("publisher");
    canonical.text(envelope.publisher.as_str());
    canonical.tag("extension");
    canonical.text(envelope.extension.as_str());
    canonical.tag("version");
    canonical.text(&envelope.version.to_string());
    canonical.tag("package_sha256");
    canonical.text(envelope.package_hash.as_str());
    canonical.tag("package_bytes");
    canonical.text(&envelope.package_bytes.to_string());
    canonical.tag("manifest_sha256");
    canonical.text(envelope.claimed_manifest_digest.as_str());
    canonical.tag("key_fingerprint");
    canonical.text(envelope.key_fingerprint.as_str());

    canonical.into_bytes()
}
