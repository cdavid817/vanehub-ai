use super::{EndpointProfileSnapshot, ProfileRuntimeKind};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TaskClass {
    Summarization,
    Embeddings,
    Classification,
    CodeReview,
    Planning,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DataPolicy {
    CloudAllowed,
    LocalPreferred,
    LocalOnly,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct HybridRoutingRule {
    pub(crate) id: String,
    pub(crate) enabled: bool,
    pub(crate) order: u32,
    pub(crate) task_class: TaskClass,
    pub(crate) preferred_profile_id: String,
    pub(crate) fallback_profile_id: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct RequiredCapabilities {
    pub(crate) tools: bool,
    pub(crate) image: bool,
    pub(crate) structured_output: bool,
    pub(crate) reasoning: bool,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct RouteCandidate<'a> {
    pub(crate) profile: &'a EndpointProfileSnapshot,
    pub(crate) ready: bool,
}

pub(crate) struct HybridRouteRequest<'a> {
    pub(crate) routing_enabled: bool,
    pub(crate) task_class: TaskClass,
    pub(crate) data_policy: DataPolicy,
    pub(crate) active_profile_id: Option<&'a str>,
    pub(crate) required_capabilities: RequiredCapabilities,
    pub(crate) rules: &'a [HybridRoutingRule],
    pub(crate) candidates: &'a [RouteCandidate<'a>],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum HybridRouteReason {
    RulePreferred,
    RuleFallbackUnavailable,
    RuleFallbackIncapable,
    ActiveRoutingDisabled,
    ActiveNoMatch,
    WaitingLocalOnly,
    NoUsableProfile,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum HybridRouteDecision {
    Selected {
        profile_id: String,
        rule_id: Option<String>,
        reason: HybridRouteReason,
    },
    WaitingForUserChoice {
        rule_id: Option<String>,
        reason: HybridRouteReason,
    },
    Rejected {
        reason: HybridRouteReason,
    },
}

pub(crate) fn route_profile(request: HybridRouteRequest<'_>) -> HybridRouteDecision {
    if !request.routing_enabled {
        return select_active(&request, HybridRouteReason::ActiveRoutingDisabled);
    }
    let rule = request
        .rules
        .iter()
        .filter(|rule| rule.enabled && rule.task_class == request.task_class)
        .min_by_key(|rule| (rule.order, rule.id.as_str()));
    let Some(rule) = rule else {
        return select_active(&request, HybridRouteReason::ActiveNoMatch);
    };

    if let Some(preferred) = candidate(&request, &rule.preferred_profile_id) {
        if admitted(&request, preferred) {
            return HybridRouteDecision::Selected {
                profile_id: preferred.profile.id.clone(),
                rule_id: Some(rule.id.clone()),
                reason: HybridRouteReason::RulePreferred,
            };
        }
    }

    if request.data_policy == DataPolicy::LocalOnly {
        return HybridRouteDecision::WaitingForUserChoice {
            rule_id: Some(rule.id.clone()),
            reason: HybridRouteReason::WaitingLocalOnly,
        };
    }
    if let Some(fallback_id) = rule.fallback_profile_id.as_deref() {
        if let Some(fallback) = candidate(&request, fallback_id) {
            if admitted(&request, fallback) {
                return HybridRouteDecision::Selected {
                    profile_id: fallback.profile.id.clone(),
                    rule_id: Some(rule.id.clone()),
                    reason: HybridRouteReason::RuleFallbackUnavailable,
                };
            }
            return HybridRouteDecision::Rejected {
                reason: HybridRouteReason::RuleFallbackIncapable,
            };
        }
    }
    HybridRouteDecision::Rejected {
        reason: HybridRouteReason::NoUsableProfile,
    }
}

fn select_active(
    request: &HybridRouteRequest<'_>,
    reason: HybridRouteReason,
) -> HybridRouteDecision {
    let selected = request
        .active_profile_id
        .and_then(|id| candidate(request, id))
        .filter(|candidate| admitted(request, candidate));
    match selected {
        Some(candidate) => HybridRouteDecision::Selected {
            profile_id: candidate.profile.id.clone(),
            rule_id: None,
            reason,
        },
        None if request.data_policy == DataPolicy::LocalOnly => {
            HybridRouteDecision::WaitingForUserChoice {
                rule_id: None,
                reason: HybridRouteReason::WaitingLocalOnly,
            }
        }
        None => HybridRouteDecision::Rejected {
            reason: HybridRouteReason::NoUsableProfile,
        },
    }
}

fn candidate<'a>(request: &'a HybridRouteRequest<'a>, id: &str) -> Option<&'a RouteCandidate<'a>> {
    request
        .candidates
        .iter()
        .find(|candidate| candidate.profile.id == id)
}

fn admitted(request: &HybridRouteRequest<'_>, candidate: &RouteCandidate<'_>) -> bool {
    candidate.ready
        && privacy_allows(request.data_policy, candidate.profile.runtime_kind)
        && supports(request.required_capabilities, candidate.profile)
}

fn privacy_allows(policy: DataPolicy, kind: ProfileRuntimeKind) -> bool {
    match policy {
        DataPolicy::CloudAllowed => true,
        DataPolicy::LocalPreferred => true,
        DataPolicy::LocalOnly => kind == ProfileRuntimeKind::Local,
    }
}

fn supports(required: RequiredCapabilities, profile: &EndpointProfileSnapshot) -> bool {
    let capabilities = profile.capabilities;
    (!required.tools || capabilities.tool_calling.is_supported())
        && (!required.image || capabilities.image_input.is_supported())
        && (!required.structured_output || capabilities.structured_output.is_supported())
        && (!required.reasoning || capabilities.reasoning_field.is_supported())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contexts::agent_runtime::domain::{
        AuthenticationMode, CapabilityState, ContextCapacityProvenance, EndpointCapabilities,
        EndpointSource, ProfileContextCapacity, ProfilePrivacy,
    };

    fn profile(
        id: &str,
        kind: ProfileRuntimeKind,
        tools: CapabilityState,
    ) -> EndpointProfileSnapshot {
        let host = if kind == ProfileRuntimeKind::Local {
            "127.0.0.1:11434"
        } else {
            "api.example.test"
        };
        let privacy = match kind {
            ProfileRuntimeKind::Cloud => ProfilePrivacy::Cloud,
            ProfileRuntimeKind::Local => ProfilePrivacy::Local,
            ProfileRuntimeKind::Private => ProfilePrivacy::Private,
        };
        let mut capabilities = EndpointCapabilities::conservative_text();
        capabilities.tool_calling.state = tools;
        EndpointProfileSnapshot::new(EndpointProfileSnapshot {
            id: id.to_string(),
            agent_id: "onepiece".to_string(),
            runtime_kind: kind,
            endpoint_source: EndpointSource::Configured,
            base_url: format!("http://{host}/v1"),
            interface_format: "openai-compatible".to_string(),
            model_id: "shared-name".to_string(),
            authentication_mode: AuthenticationMode::None,
            credential_present: false,
            timeout_ms: 10_000,
            privacy,
            capabilities,
            context_capacity: ProfileContextCapacity {
                context_window_tokens: None,
                reserved_output_tokens: 0,
                provenance: ContextCapacityProvenance::Unknown,
            },
        })
        .expect("profile")
    }

    #[test]
    fn deterministic_rule_prefers_lowest_order_then_id() {
        let local = profile(
            "local",
            ProfileRuntimeKind::Local,
            CapabilityState::Supported,
        );
        let cloud = profile(
            "cloud",
            ProfileRuntimeKind::Cloud,
            CapabilityState::Supported,
        );
        let rules = vec![
            HybridRoutingRule {
                id: "b".to_string(),
                enabled: true,
                order: 2,
                task_class: TaskClass::Summarization,
                preferred_profile_id: "cloud".to_string(),
                fallback_profile_id: None,
            },
            HybridRoutingRule {
                id: "a".to_string(),
                enabled: true,
                order: 1,
                task_class: TaskClass::Summarization,
                preferred_profile_id: "local".to_string(),
                fallback_profile_id: Some("cloud".to_string()),
            },
        ];
        let candidates = vec![
            RouteCandidate {
                profile: &cloud,
                ready: true,
            },
            RouteCandidate {
                profile: &local,
                ready: true,
            },
        ];
        let result = route_profile(HybridRouteRequest {
            routing_enabled: true,
            task_class: TaskClass::Summarization,
            data_policy: DataPolicy::LocalPreferred,
            active_profile_id: Some("cloud"),
            required_capabilities: RequiredCapabilities::default(),
            rules: &rules,
            candidates: &candidates,
        });
        assert!(
            matches!(result, HybridRouteDecision::Selected { profile_id, rule_id: Some(rule), reason: HybridRouteReason::RulePreferred } if profile_id == "local" && rule == "a")
        );
    }

    #[test]
    fn local_only_never_selects_cloud_fallback() {
        let local = profile(
            "local",
            ProfileRuntimeKind::Local,
            CapabilityState::Supported,
        );
        let cloud = profile(
            "cloud",
            ProfileRuntimeKind::Cloud,
            CapabilityState::Supported,
        );
        let rules = vec![HybridRoutingRule {
            id: "local-first".to_string(),
            enabled: true,
            order: 0,
            task_class: TaskClass::Classification,
            preferred_profile_id: "local".to_string(),
            fallback_profile_id: Some("cloud".to_string()),
        }];
        let candidates = vec![
            RouteCandidate {
                profile: &local,
                ready: false,
            },
            RouteCandidate {
                profile: &cloud,
                ready: true,
            },
        ];
        let result = route_profile(HybridRouteRequest {
            routing_enabled: true,
            task_class: TaskClass::Classification,
            data_policy: DataPolicy::LocalOnly,
            active_profile_id: Some("cloud"),
            required_capabilities: RequiredCapabilities::default(),
            rules: &rules,
            candidates: &candidates,
        });
        assert!(matches!(
            result,
            HybridRouteDecision::WaitingForUserChoice {
                reason: HybridRouteReason::WaitingLocalOnly,
                ..
            }
        ));
    }

    #[test]
    fn unsupported_tools_are_rejected_before_selection() {
        let local = profile(
            "local",
            ProfileRuntimeKind::Local,
            CapabilityState::Unsupported,
        );
        let candidates = vec![RouteCandidate {
            profile: &local,
            ready: true,
        }];
        let result = route_profile(HybridRouteRequest {
            routing_enabled: false,
            task_class: TaskClass::Unknown,
            data_policy: DataPolicy::LocalOnly,
            active_profile_id: Some("local"),
            required_capabilities: RequiredCapabilities {
                tools: true,
                ..Default::default()
            },
            rules: &[],
            candidates: &candidates,
        });
        assert!(matches!(
            result,
            HybridRouteDecision::WaitingForUserChoice { .. }
        ));
    }

    #[test]
    fn ordered_rule_evaluation_has_a_bounded_single_pass_work_budget() {
        let local = profile(
            "local",
            ProfileRuntimeKind::Local,
            CapabilityState::Supported,
        );
        let candidates = vec![RouteCandidate {
            profile: &local,
            ready: true,
        }];
        let rules = (0..10_000)
            .map(|order| HybridRoutingRule {
                id: format!("rule-{order:05}"),
                enabled: true,
                order,
                task_class: TaskClass::Summarization,
                preferred_profile_id: "local".to_string(),
                fallback_profile_id: None,
            })
            .collect::<Vec<_>>();
        let result = route_profile(HybridRouteRequest {
            routing_enabled: true,
            task_class: TaskClass::Summarization,
            data_policy: DataPolicy::LocalOnly,
            active_profile_id: Some("local"),
            required_capabilities: RequiredCapabilities::default(),
            rules: &rules,
            candidates: &candidates,
        });
        assert!(
            matches!(result, HybridRouteDecision::Selected { rule_id: Some(id), .. } if id == "rule-00000")
        );
    }
}
