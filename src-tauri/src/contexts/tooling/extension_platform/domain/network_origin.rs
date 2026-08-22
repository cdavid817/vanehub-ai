// No production caller yet; see `identity.rs`.
#![cfg_attr(not(test), allow(dead_code))]

//! Network origins a package may request.
//!
//! An origin is scheme, host, and port — nothing else. A request carrying a path, a query, or
//! credentials is not an origin, and accepting one would mean the reviewed text and the enforced
//! rule disagree: the user reads "https://api.github.com/repos/acme" and approves what looks like
//! one repository, while the broker can only ever match on the origin and has granted the whole
//! host.
//!
//! Unlike a package path, an origin *is* canonicalized. Host and scheme are case-insensitive by
//! definition and a default port is not a distinction, so `HTTPS://API.GitHub.com:443` and
//! `https://api.github.com` name the same thing; storing them as two entries would let a package
//! appear to request less than it does. What is refused is anything that is not merely a different
//! spelling of the same origin.

use super::ExtensionOriginError;
use url::{Host, Url};

/// Long enough for any real hostname, short enough to bound a diagnostic.
pub(crate) const MAX_ORIGIN_CHARACTERS: usize = 253;

/// A validated `scheme://host[:port]`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct NetworkOrigin(String);

impl NetworkOrigin {
    pub(crate) fn parse(value: &str) -> Result<Self, ExtensionOriginError> {
        let refuse = |reason: OriginRejection| {
            Err(ExtensionOriginError {
                origin: value.chars().take(MAX_ORIGIN_CHARACTERS).collect(),
                reason,
            })
        };

        if value.is_empty() {
            return refuse(OriginRejection::Empty);
        }
        if value.chars().count() > MAX_ORIGIN_CHARACTERS {
            return refuse(OriginRejection::TooLong);
        }
        // Checked on the text, before the parser gets a chance to interpret `*` as an ordinary
        // host character. A wildcard is a request for every subdomain, which is not something a
        // reviewer can evaluate and not something the broker can enforce.
        if value.contains('*') {
            return refuse(OriginRejection::Wildcard);
        }

        let Ok(url) = Url::parse(value) else {
            return refuse(OriginRejection::Unparseable);
        };

        if !matches!(url.scheme(), "http" | "https") {
            return refuse(OriginRejection::UnsupportedScheme);
        }
        if !url.username().is_empty() || url.password().is_some() {
            return refuse(OriginRejection::Userinfo);
        }
        // `Url` normalizes an empty path to "/", so both spellings mean "no path".
        if !matches!(url.path(), "" | "/") {
            return refuse(OriginRejection::HasPath);
        }
        if url.query().is_some() {
            return refuse(OriginRejection::HasQuery);
        }
        if url.fragment().is_some() {
            return refuse(OriginRejection::HasFragment);
        }

        let Some(host) = url.host() else {
            return refuse(OriginRejection::MissingHost);
        };
        if !is_loopback(&host) && url.scheme() == "http" {
            // Plaintext to a remote host hands every request and every bearer token to the
            // network. Loopback is exempt because a local service has no other option.
            return refuse(OriginRejection::InsecureRemoteScheme);
        }

        // `origin()` returns the canonical `scheme://host[:port]`, dropping a default port. Both
        // http and https are special schemes, so this is never opaque here.
        let canonical = url.origin().ascii_serialization();
        if canonical == "null" {
            return refuse(OriginRejection::Unparseable);
        }
        Ok(Self(canonical))
    }

    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

fn is_loopback(host: &Host<&str>) -> bool {
    match host {
        Host::Domain(domain) => domain.eq_ignore_ascii_case("localhost"),
        Host::Ipv4(address) => address.is_loopback(),
        Host::Ipv6(address) => address.is_loopback(),
    }
}

/// Why a requested origin is not one. One variant per rule, so a diagnostic says which.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum OriginRejection {
    Empty,
    TooLong,
    /// `https://*.github.com`. Neither reviewable by a person nor enforceable by the broker.
    Wildcard,
    Unparseable,
    /// Anything but http or https: no `file:`, no `ftp:`, no custom scheme.
    UnsupportedScheme,
    /// Plaintext http to something other than loopback.
    InsecureRemoteScheme,
    /// `https://user:token@host` — a credential written into a manifest.
    Userinfo,
    MissingHost,
    HasPath,
    HasQuery,
    HasFragment,
}

impl OriginRejection {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::Empty => "empty",
            Self::TooLong => "too_long",
            Self::Wildcard => "wildcard",
            Self::Unparseable => "unparseable",
            Self::UnsupportedScheme => "unsupported_scheme",
            Self::InsecureRemoteScheme => "insecure_remote_scheme",
            Self::Userinfo => "userinfo",
            Self::MissingHost => "missing_host",
            Self::HasPath => "has_path",
            Self::HasQuery => "has_query",
            Self::HasFragment => "has_fragment",
        }
    }
}
