use super::invocation_budget::SkillToolInvocationBudget;
use crate::contexts::tooling::skill_tools::application::SkillToolApplicationError;
use crate::contexts::tooling::skill_tools::domain::DEFAULT_SKILL_TOOL_LIMITS;
use crate::contexts::tooling::skills::api::SkillSecretReadPort;
use std::collections::BTreeSet;

#[derive(Clone, PartialEq, Eq)]
pub(crate) struct SkillSecretBinding {
    pub(crate) capability: String,
    pub(crate) record_id: String,
    pub(crate) property_key: String,
}

pub(crate) trait RedactGrantedSecret {
    fn redact_granted_secret(&mut self, secret: &str);
}

impl RedactGrantedSecret for String {
    fn redact_granted_secret(&mut self, secret: &str) {
        if !secret.is_empty() {
            *self = self.replace(secret, "[REDACTED]");
        }
    }
}

pub(crate) struct SkillToolSecretGateway<'a> {
    declared: &'a [String],
    approved: BTreeSet<String>,
    bindings: &'a [SkillSecretBinding],
    secrets: &'a dyn SkillSecretReadPort,
    budget: SkillToolInvocationBudget,
}

impl<'a> SkillToolSecretGateway<'a> {
    pub(crate) fn new(
        declared: &'a [String],
        approved: impl IntoIterator<Item = String>,
        bindings: &'a [SkillSecretBinding],
        secrets: &'a dyn SkillSecretReadPort,
    ) -> Self {
        Self::with_budget(
            declared,
            approved,
            bindings,
            secrets,
            SkillToolInvocationBudget::new(DEFAULT_SKILL_TOOL_LIMITS),
        )
    }

    pub(crate) fn with_budget(
        declared: &'a [String],
        approved: impl IntoIterator<Item = String>,
        bindings: &'a [SkillSecretBinding],
        secrets: &'a dyn SkillSecretReadPort,
        budget: SkillToolInvocationBudget,
    ) -> Self {
        Self {
            declared,
            approved: approved.into_iter().collect(),
            bindings,
            secrets,
            budget,
        }
    }

    /// The value exists only for the duration of the backend closure and is zeroized afterwards.
    /// The gateway itself exposes no serializable or printable secret-bearing model.
    pub(crate) fn with_secret<T: RedactGrantedSecret>(
        &self,
        capability: &str,
        operation: impl FnOnce(&str) -> T,
    ) -> Result<T, SkillToolApplicationError> {
        if !self.declared.iter().any(|item| item == capability)
            || !self.approved.contains(capability)
        {
            return Err(denied());
        }
        self.budget.reserve_host_call()?;
        let binding = self
            .bindings
            .iter()
            .find(|binding| binding.capability == capability)
            .ok_or_else(denied)?;
        let value = self
            .secrets
            .read_secret(&binding.record_id, &binding.property_key)
            .map_err(|_| unavailable())?
            .ok_or_else(unavailable)?;
        let mut output = operation(value.as_str());
        output.redact_granted_secret(value.as_str());
        Ok(output)
    }
}

fn denied() -> SkillToolApplicationError {
    SkillToolApplicationError::HostDenied("secret.capability".to_string())
}

fn unavailable() -> SkillToolApplicationError {
    SkillToolApplicationError::Filesystem("secret unavailable".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contexts::tooling::skills::api::SkillError;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use zeroize::Zeroizing;

    struct Secrets {
        reads: AtomicUsize,
    }

    impl SkillSecretReadPort for Secrets {
        fn read_secret(
            &self,
            _record_id: &str,
            _property_key: &str,
        ) -> Result<Option<Zeroizing<String>>, SkillError> {
            self.reads.fetch_add(1, Ordering::SeqCst);
            Ok(Some(Zeroizing::new("fixture-secret".to_string())))
        }
    }

    #[test]
    fn exact_declared_and_approved_capability_is_required_before_resolution() {
        let secrets = Secrets {
            reads: AtomicUsize::new(0),
        };
        let declared = vec!["service.token".to_string()];
        let bindings = vec![SkillSecretBinding {
            capability: "service.token".to_string(),
            record_id: "configured-skill:user".to_string(),
            property_key: "api_key".to_string(),
        }];
        let denied_gateway =
            SkillToolSecretGateway::new(&declared, Vec::new(), &bindings, &secrets);
        assert!(denied_gateway
            .with_secret("service.token", |value| value.to_string())
            .is_err());
        assert_eq!(secrets.reads.load(Ordering::SeqCst), 0);

        let approved_gateway = SkillToolSecretGateway::new(
            &declared,
            vec!["service.token".to_string()],
            &bindings,
            &secrets,
        );
        assert_eq!(
            approved_gateway
                .with_secret("service.token", |_| "safe-result".to_string())
                .expect("approved"),
            "safe-result"
        );
        assert_eq!(secrets.reads.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn binding_and_gateway_debug_surfaces_cannot_expose_values() {
        assert!(!std::any::type_name::<SkillSecretBinding>().contains("fixture-secret"));
        assert!(!std::any::type_name::<SkillToolSecretGateway<'_>>().contains("fixture-secret"));
    }

    #[test]
    fn sibling_gateways_share_the_same_host_call_budget() {
        let secrets = Secrets {
            reads: AtomicUsize::new(0),
        };
        let declared = vec!["service.token".to_string()];
        let bindings = vec![SkillSecretBinding {
            capability: "service.token".to_string(),
            record_id: "configured-skill:user".to_string(),
            property_key: "api_key".to_string(),
        }];
        let mut limits = DEFAULT_SKILL_TOOL_LIMITS;
        limits.host_calls = 1;
        let budget = SkillToolInvocationBudget::new(limits);
        let first = SkillToolSecretGateway::with_budget(
            &declared,
            vec!["service.token".to_string()],
            &bindings,
            &secrets,
            budget.clone(),
        );
        let nested = SkillToolSecretGateway::with_budget(
            &declared,
            vec!["service.token".to_string()],
            &bindings,
            &secrets,
            budget,
        );

        first
            .with_secret("service.token", |_| "safe".to_string())
            .expect("first call");
        assert!(matches!(
            nested.with_secret("service.token", |_| "safe".to_string()),
            Err(SkillToolApplicationError::ResourceLimit(_))
        ));
        assert_eq!(secrets.reads.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn granted_secret_is_removed_from_operation_output_before_return() {
        let secrets = Secrets {
            reads: AtomicUsize::new(0),
        };
        let declared = vec!["service.token".to_string()];
        let bindings = vec![SkillSecretBinding {
            capability: "service.token".to_string(),
            record_id: "configured-skill:user".to_string(),
            property_key: "api_key".to_string(),
        }];
        let gateway = SkillToolSecretGateway::new(
            &declared,
            vec!["service.token".to_string()],
            &bindings,
            &secrets,
        );

        let output = gateway
            .with_secret("service.token", |value| format!("echo:{value}"))
            .expect("operation");
        assert_eq!(output, "echo:[REDACTED]");
        assert!(!output.contains("fixture-secret"));
    }
}
