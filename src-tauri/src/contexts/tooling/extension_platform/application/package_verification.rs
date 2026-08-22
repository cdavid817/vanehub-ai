// The install flow that calls this lands with task 2.6; see `identity.rs`.
#![cfg_attr(not(test), allow(dead_code))]

//! Turning package bytes and an optional envelope into one provenance answer.
//!
//! The service does the two things the domain cannot: it finds the key, and it decides what an
//! absent envelope means as a *state* rather than as an error. Everything between those — reading
//! the envelope, canonically encoding what was signed, checking the signature — is domain work,
//! called from here.
//!
//! Nothing in this file decides whether the package may be installed. That is task 2.5's policy,
//! and separating them is the point: a service that both verified and admitted would be one where
//! the admission rule could quietly start depending on something other than the verification.

use super::ports::PublisherKeyDirectory;
use crate::contexts::tooling::extension_platform::domain::{
    parse_signature_envelope, verify_package_signature, PackageFacts, SignatureRejection,
    SignatureState,
};
use std::sync::Arc;

/// A storage failure while looking up the key an envelope named.
///
/// Not a `SignatureState`: no verdict was reached. Reporting one would mean telling an operator
/// their package is untrusted when nothing was actually checked, so the caller has to handle this
/// separately from every answer the verifier can give.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PublisherLookupUnavailable(pub(crate) String);

pub(crate) struct PackageVerificationService {
    keys: Arc<dyn PublisherKeyDirectory>,
}

impl PackageVerificationService {
    pub(crate) fn new(keys: Arc<dyn PublisherKeyDirectory>) -> Self {
        Self { keys }
    }

    /// Answers what is known about a package's provenance.
    ///
    /// `envelope_bytes` is `None` when no signature accompanied the package. That is a state, not
    /// a failure — what to do about it is Developer Mode's business.
    pub(crate) fn verify(
        &self,
        envelope_bytes: Option<&[u8]>,
        package: &PackageFacts,
    ) -> Result<SignatureState, PublisherLookupUnavailable> {
        let Some(bytes) = envelope_bytes else {
            return Ok(SignatureState::Unsigned);
        };
        let envelope = match parse_signature_envelope(bytes) {
            Ok(envelope) => envelope,
            Err(error) => return Ok(SignatureState::Unreadable(error)),
        };
        let key = self
            .keys
            .find(&envelope.key_fingerprint)
            .map_err(PublisherLookupUnavailable)?;
        let Some(key) = key else {
            return Ok(SignatureState::Rejected(
                SignatureRejection::UnknownPublisherKey,
            ));
        };
        Ok(match verify_package_signature(&envelope, &key, package) {
            Ok(verified) => SignatureState::Verified(verified),
            Err(reason) => SignatureState::Rejected(reason),
        })
    }
}
