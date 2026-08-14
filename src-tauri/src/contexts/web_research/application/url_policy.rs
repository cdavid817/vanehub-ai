use std::collections::BTreeSet;
use url::{Host, Url};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PublicUrlResolution {
    pub(crate) normalized_url: String,
    pub(crate) host: String,
    pub(crate) port: u16,
    pub(crate) addresses: BTreeSet<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum GuardedUrlPolicyError {
    InvalidUrl,
    DisallowedScheme,
    CredentialsDisallowed,
    HostRequired,
    PortDisallowed,
    ResolutionFailed,
    AddressDisallowed,
    DnsRebinding,
}

pub(crate) trait UrlResolverPort: Send + Sync {
    fn resolve(&self, host: &str, port: u16) -> Result<Vec<String>, GuardedUrlPolicyError>;
}

pub(crate) struct GuardedUrlPolicy;

impl GuardedUrlPolicy {
    pub(crate) fn resolve_public(
        raw_url: &str,
        resolver: &dyn UrlResolverPort,
    ) -> Result<PublicUrlResolution, GuardedUrlPolicyError> {
        let mut url = parse_url(raw_url)?;
        let host = url.host().ok_or(GuardedUrlPolicyError::HostRequired)?;
        let host_string = host.to_string();
        let port = url
            .port_or_known_default()
            .ok_or(GuardedUrlPolicyError::PortDisallowed)?;
        if !matches!(port, 80 | 443) {
            return Err(GuardedUrlPolicyError::PortDisallowed);
        }
        let addresses = match host {
            Host::Ipv4(address) => vec![address.to_string()],
            Host::Ipv6(address) => vec![address.to_string()],
            Host::Domain(domain) => resolver.resolve(domain, port)?,
        };
        let addresses = admit_addresses(addresses)?;
        url.set_fragment(None);
        Ok(PublicUrlResolution {
            normalized_url: url.to_string(),
            host: host_string,
            port,
            addresses,
        })
    }

    pub(crate) fn revalidate_resolution(
        witness: &PublicUrlResolution,
        resolver: &dyn UrlResolverPort,
    ) -> Result<(), GuardedUrlPolicyError> {
        let current = admit_addresses(resolver.resolve(&witness.host, witness.port)?)?;
        if current != witness.addresses {
            return Err(GuardedUrlPolicyError::DnsRebinding);
        }
        Ok(())
    }

    pub(crate) fn validate_redirect(
        current: &PublicUrlResolution,
        location: &str,
        resolver: &dyn UrlResolverPort,
    ) -> Result<PublicUrlResolution, GuardedUrlPolicyError> {
        let current_url =
            Url::parse(&current.normalized_url).map_err(|_| GuardedUrlPolicyError::InvalidUrl)?;
        let redirected = current_url
            .join(location)
            .map_err(|_| GuardedUrlPolicyError::InvalidUrl)?;
        Self::resolve_public(redirected.as_str(), resolver)
    }
}

fn parse_url(raw_url: &str) -> Result<Url, GuardedUrlPolicyError> {
    if raw_url.trim() != raw_url || raw_url.len() > 4096 {
        return Err(GuardedUrlPolicyError::InvalidUrl);
    }
    let url = Url::parse(raw_url).map_err(|_| GuardedUrlPolicyError::InvalidUrl)?;
    if !matches!(url.scheme(), "http" | "https") {
        return Err(GuardedUrlPolicyError::DisallowedScheme);
    }
    if !url.username().is_empty() || url.password().is_some() {
        return Err(GuardedUrlPolicyError::CredentialsDisallowed);
    }
    Ok(url)
}

fn admit_addresses(addresses: Vec<String>) -> Result<BTreeSet<String>, GuardedUrlPolicyError> {
    if addresses.is_empty() {
        return Err(GuardedUrlPolicyError::ResolutionFailed);
    }
    let addresses = addresses.into_iter().collect::<BTreeSet<_>>();
    if addresses.iter().any(|address| !is_public_address(address)) {
        return Err(GuardedUrlPolicyError::AddressDisallowed);
    }
    Ok(addresses)
}

fn is_public_address(address: &str) -> bool {
    match Host::parse(address) {
        Ok(Host::Ipv4(address)) => is_public_ipv4(address.octets()),
        Ok(Host::Ipv6(address)) => is_public_ipv6(
            address.segments(),
            address.to_ipv4_mapped().map(|value| value.octets()),
        ),
        _ => false,
    }
}

fn is_public_ipv4([a, b, c, _]: [u8; 4]) -> bool {
    !(a == 0
        || a == 10
        || a == 127
        || (a == 100 && (64..=127).contains(&b))
        || (a == 169 && b == 254)
        || (a == 172 && (16..=31).contains(&b))
        || (a == 192 && b == 0 && c == 0)
        || (a == 192 && b == 0 && c == 2)
        || (a == 192 && b == 168)
        || (a == 198 && matches!(b, 18 | 19))
        || (a == 198 && b == 51 && c == 100)
        || (a == 203 && b == 0 && c == 113)
        || a >= 224)
}

fn is_public_ipv6(segments: [u16; 8], mapped: Option<[u8; 4]>) -> bool {
    if let Some(address) = mapped {
        return is_public_ipv4(address);
    }
    !(segments.iter().all(|segment| *segment == 0)
        || segments == [0, 0, 0, 0, 0, 0, 0, 1]
        || (segments[0] & 0xfe00) == 0xfc00
        || (segments[0] & 0xffc0) == 0xfe80
        || (segments[0] & 0xff00) == 0xff00
        || (segments[0] == 0x2001 && segments[1] == 0x0db8))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    struct Resolver {
        addresses: Mutex<Vec<String>>,
    }

    impl UrlResolverPort for Resolver {
        fn resolve(&self, _host: &str, _port: u16) -> Result<Vec<String>, GuardedUrlPolicyError> {
            Ok(self.addresses.lock().expect("addresses").clone())
        }
    }

    fn resolver(addresses: &[&str]) -> Resolver {
        Resolver {
            addresses: Mutex::new(
                addresses
                    .iter()
                    .map(|address| (*address).to_owned())
                    .collect(),
            ),
        }
    }

    #[test]
    fn public_urls_are_normalized_and_credentials_schemes_and_ports_are_rejected() {
        let resolver = resolver(&["93.184.216.34"]);
        let public =
            GuardedUrlPolicy::resolve_public("https://Example.COM:443/docs?q=1#section", &resolver)
                .expect("public");
        assert_eq!(public.normalized_url, "https://example.com/docs?q=1");
        for (url, error) in [
            ("file:///secret", GuardedUrlPolicyError::DisallowedScheme),
            (
                "https://user:password@example.com/",
                GuardedUrlPolicyError::CredentialsDisallowed,
            ),
            (
                "https://example.com:8443/",
                GuardedUrlPolicyError::PortDisallowed,
            ),
        ] {
            assert_eq!(GuardedUrlPolicy::resolve_public(url, &resolver), Err(error));
        }
    }

    #[test]
    fn private_metadata_documentation_and_mixed_dns_answers_fail_closed() {
        for addresses in [
            vec!["127.0.0.1"],
            vec!["169.254.169.254"],
            vec!["10.0.0.1"],
            vec!["192.0.2.1"],
            vec!["::1"],
            vec!["fc00::1"],
            vec!["93.184.216.34", "10.0.0.1"],
        ] {
            assert_eq!(
                GuardedUrlPolicy::resolve_public("https://example.com/", &resolver(&addresses)),
                Err(GuardedUrlPolicyError::AddressDisallowed)
            );
        }
    }

    #[test]
    fn every_redirect_is_revalidated_and_dns_rebinding_is_rejected() {
        let resolver = resolver(&["93.184.216.34"]);
        let initial =
            GuardedUrlPolicy::resolve_public("https://example.com/a", &resolver).expect("initial");
        let redirected =
            GuardedUrlPolicy::validate_redirect(&initial, "/b", &resolver).expect("redirect");
        assert_eq!(redirected.normalized_url, "https://example.com/b");

        *resolver.addresses.lock().expect("addresses") = vec!["10.0.0.1".to_owned()];
        assert_eq!(
            GuardedUrlPolicy::revalidate_resolution(&initial, &resolver),
            Err(GuardedUrlPolicyError::AddressDisallowed)
        );
        *resolver.addresses.lock().expect("addresses") = vec!["93.184.216.35".to_owned()];
        assert_eq!(
            GuardedUrlPolicy::revalidate_resolution(&initial, &resolver),
            Err(GuardedUrlPolicyError::DnsRebinding)
        );
    }
}
