use super::invocation_budget::SkillToolInvocationBudget;
use crate::contexts::tooling::skill_tools::application::SkillToolApplicationError;
use crate::contexts::tooling::skill_tools::domain::{SkillNetworkPermissions, SkillToolLimits};
use crate::contexts::web_research::api::{
    GuardedUrlPolicy, GuardedUrlPolicyError, PublicUrlResolution, UrlResolverPort,
};
use crate::platform::network::{apply_proxy_routing, RoutableClientBuilder};
use reqwest::blocking::{Client, Response};
use reqwest::redirect::Policy;
use std::io::Read;
use std::net::{IpAddr, SocketAddr, ToSocketAddrs};
use std::time::Duration;
use url::Url;

const MAX_REDIRECTS: u8 = 5;

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct SkillToolNetworkResponse {
    pub(crate) final_url: String,
    pub(crate) status: u16,
    pub(crate) body: Vec<u8>,
    pub(crate) redirects: u8,
}

pub(crate) struct SkillToolNetworkGateway {
    origins: Vec<String>,
    limits: SkillToolLimits,
    budget: SkillToolInvocationBudget,
    resolver: SystemResolver,
}

impl SkillToolNetworkGateway {
    pub(crate) fn new(permissions: &SkillNetworkPermissions, limits: SkillToolLimits) -> Self {
        Self::with_budget(permissions, limits, SkillToolInvocationBudget::new(limits))
    }

    pub(crate) fn with_budget(
        permissions: &SkillNetworkPermissions,
        limits: SkillToolLimits,
        budget: SkillToolInvocationBudget,
    ) -> Self {
        Self {
            origins: permissions.origins.clone(),
            limits,
            budget,
            resolver: SystemResolver,
        }
    }

    pub(crate) fn get(
        &mut self,
        raw_url: &str,
    ) -> Result<SkillToolNetworkResponse, SkillToolApplicationError> {
        self.budget.reserve_host_call()?;
        let mut current = self.resolve_admitted(raw_url)?;
        let mut redirects = 0;
        loop {
            GuardedUrlPolicy::revalidate_resolution(&current, &self.resolver)
                .map_err(policy_error)?;
            let remaining = self.budget.remaining_time()?;
            let response = send(&current, remaining)?;
            let status = response.status().as_u16();
            if response.status().is_redirection() {
                if redirects >= MAX_REDIRECTS {
                    return Err(resource_limit("network.redirects"));
                }
                let location = response
                    .headers()
                    .get(reqwest::header::LOCATION)
                    .and_then(|value| value.to_str().ok())
                    .ok_or_else(|| denied("network.redirect"))?;
                current = GuardedUrlPolicy::validate_redirect(&current, location, &self.resolver)
                    .map_err(policy_error)?;
                self.admit_origin(&current.normalized_url)?;
                redirects += 1;
                continue;
            }
            let body = self.read_bounded(response)?;
            return Ok(SkillToolNetworkResponse {
                final_url: current.normalized_url,
                status,
                body,
                redirects,
            });
        }
    }

    fn resolve_admitted(
        &self,
        raw_url: &str,
    ) -> Result<PublicUrlResolution, SkillToolApplicationError> {
        self.admit_origin(raw_url)?;
        GuardedUrlPolicy::resolve_public(raw_url, &self.resolver).map_err(policy_error)
    }

    fn admit_origin(&self, raw_url: &str) -> Result<(), SkillToolApplicationError> {
        let url = Url::parse(raw_url).map_err(|_| denied("network.origin"))?;
        let origin = url.origin().ascii_serialization();
        self.origins
            .contains(&origin)
            .then_some(())
            .ok_or_else(|| denied("network.origin"))
    }

    fn read_bounded(
        &mut self,
        mut response: Response,
    ) -> Result<Vec<u8>, SkillToolApplicationError> {
        let remaining = self.limits.network_bytes;
        if response
            .content_length()
            .is_some_and(|length| length > remaining)
        {
            return Err(resource_limit("network.bytes"));
        }
        let mut body = Vec::new();
        response
            .by_ref()
            .take(remaining.saturating_add(1))
            .read_to_end(&mut body)
            .map_err(|_| transport_error())?;
        if body.len() as u64 > remaining {
            return Err(resource_limit("network.bytes"));
        }
        self.budget.consume_network(body.len() as u64)?;
        Ok(body)
    }
}

fn send(
    resolution: &PublicUrlResolution,
    timeout: Duration,
) -> Result<Response, SkillToolApplicationError> {
    let addresses = resolution
        .addresses
        .iter()
        .map(|address| {
            address
                .parse::<IpAddr>()
                .map(|address| SocketAddr::new(address, resolution.port))
                .map_err(|_| denied("network.address"))
        })
        .collect::<Result<Vec<_>, _>>()?;
    let builder = reqwest::blocking::Client::builder()
        .redirect(Policy::none())
        .referer(false)
        .timeout(timeout)
        .connect_timeout(timeout)
        .pool_max_idle_per_host(0)
        .resolve_to_addrs(&resolution.host, &addresses);
    let routed = apply_proxy_routing(builder).map_err(|_| transport_error())?;
    let client: Client = routed.finish().map_err(|_| transport_error())?;
    client
        .get(&resolution.normalized_url)
        .header(reqwest::header::ACCEPT_ENCODING, "identity")
        .send()
        .map_err(|_| transport_error())
}

#[derive(Debug, Clone, Copy)]
struct SystemResolver;

impl UrlResolverPort for SystemResolver {
    fn resolve(&self, host: &str, port: u16) -> Result<Vec<String>, GuardedUrlPolicyError> {
        (host, port)
            .to_socket_addrs()
            .map(|addresses| addresses.map(|address| address.ip().to_string()).collect())
            .map_err(|_| GuardedUrlPolicyError::ResolutionFailed)
    }
}

fn policy_error(_error: GuardedUrlPolicyError) -> SkillToolApplicationError {
    denied("network.policy")
}

fn denied(capability: &str) -> SkillToolApplicationError {
    SkillToolApplicationError::HostDenied(capability.to_string())
}

fn resource_limit(limit: &str) -> SkillToolApplicationError {
    SkillToolApplicationError::ResourceLimit(limit.to_string())
}

fn transport_error() -> SkillToolApplicationError {
    SkillToolApplicationError::Filesystem("network request failed".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contexts::tooling::skill_tools::domain::DEFAULT_SKILL_TOOL_LIMITS;

    #[test]
    fn undeclared_and_private_targets_fail_before_transport() {
        let permissions = SkillNetworkPermissions {
            origins: vec!["https://example.com".to_string()],
        };
        let mut gateway = SkillToolNetworkGateway::new(&permissions, DEFAULT_SKILL_TOOL_LIMITS);

        assert!(matches!(
            gateway.get("https://undeclared.example/path"),
            Err(SkillToolApplicationError::HostDenied(_))
        ));
        let private = SkillNetworkPermissions {
            origins: vec!["https://127.0.0.1".to_string()],
        };
        let mut private_gateway = SkillToolNetworkGateway::new(&private, DEFAULT_SKILL_TOOL_LIMITS);
        assert!(matches!(
            private_gateway.get("https://127.0.0.1/secret"),
            Err(SkillToolApplicationError::HostDenied(_))
        ));
    }

    #[test]
    fn origin_admission_is_exact_and_https_only() {
        let gateway = SkillToolNetworkGateway::new(
            &SkillNetworkPermissions {
                origins: vec!["https://example.com".to_string()],
            },
            DEFAULT_SKILL_TOOL_LIMITS,
        );
        assert!(gateway.admit_origin("https://example.com/path").is_ok());
        assert!(gateway.admit_origin("http://example.com/path").is_err());
        assert!(gateway
            .admit_origin("https://sub.example.com/path")
            .is_err());
    }
}
