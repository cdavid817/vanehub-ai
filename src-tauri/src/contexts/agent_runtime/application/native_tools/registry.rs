use super::{
    NativeToolDefinition, NativeToolExecutionMode, NativeToolHandler, NativeToolOperation,
    OnePieceToolFeatureGates, ToolEligibility, ToolEligibilityContext,
    NATIVE_TOOL_CONTRACT_VERSION,
};
use crate::contexts::agent_runtime::application::ToolDefinition;
use std::collections::BTreeMap;
use std::sync::Arc;

pub(crate) const ONEPIECE_AGENT_ID: &str = "onepiece";
pub(crate) const ONEPIECE_ONLY_TOOL_NAMES: [&str; 8] = [
    "browser",
    "web_search",
    "web_fetch",
    "code_execution",
    "ocr",
    "artifact",
    "delegate_cli",
    "apply_delegation_changes",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum NativeToolRegistryError {
    EmptyName,
    DuplicateName(String),
    UnsupportedContractVersion { name: String, version: u16 },
}

#[derive(Clone)]
pub(crate) struct NativeToolRegistry {
    handlers: Arc<BTreeMap<String, Arc<dyn NativeToolHandler>>>,
    feature_gates: Arc<OnePieceToolFeatureGates>,
    readiness: Arc<BTreeMap<String, Option<super::NativeToolReadinessReasonCode>>>,
}

impl NativeToolRegistry {
    pub(crate) fn empty() -> Self {
        Self {
            handlers: Arc::new(BTreeMap::new()),
            feature_gates: Arc::new(OnePieceToolFeatureGates::all_enabled()),
            readiness: Arc::new(BTreeMap::new()),
        }
    }

    #[allow(dead_code)]
    pub(crate) fn try_new(
        handlers: Vec<Arc<dyn NativeToolHandler>>,
    ) -> Result<Self, NativeToolRegistryError> {
        Self::try_new_with_feature_gates(handlers, OnePieceToolFeatureGates::all_enabled())
    }

    pub(crate) fn try_new_with_feature_gates(
        handlers: Vec<Arc<dyn NativeToolHandler>>,
        feature_gates: OnePieceToolFeatureGates,
    ) -> Result<Self, NativeToolRegistryError> {
        Self::try_new_with_feature_gates_and_readiness(handlers, feature_gates, BTreeMap::new())
    }

    pub(crate) fn try_new_with_feature_gates_and_readiness(
        handlers: Vec<Arc<dyn NativeToolHandler>>,
        feature_gates: OnePieceToolFeatureGates,
        readiness_overrides: BTreeMap<String, super::NativeToolReadinessReasonCode>,
    ) -> Result<Self, NativeToolRegistryError> {
        let mut indexed = BTreeMap::new();
        for handler in handlers {
            let definition = handler.definition();
            let name = definition.name.trim().to_string();
            if name.is_empty() {
                return Err(NativeToolRegistryError::EmptyName);
            }
            if definition.contract_version != NATIVE_TOOL_CONTRACT_VERSION {
                return Err(NativeToolRegistryError::UnsupportedContractVersion {
                    name,
                    version: definition.contract_version,
                });
            }
            if indexed.insert(name.clone(), handler).is_some() {
                return Err(NativeToolRegistryError::DuplicateName(name));
            }
        }
        let readiness = indexed
            .keys()
            .map(|name| (name.clone(), readiness_overrides.get(name).copied()))
            .collect();
        Ok(Self {
            handlers: Arc::new(indexed),
            feature_gates: Arc::new(feature_gates),
            readiness: Arc::new(readiness),
        })
    }

    pub(crate) fn handler(&self, name: &str) -> Option<Arc<dyn NativeToolHandler>> {
        self.handlers.get(name).cloned()
    }

    pub(crate) fn eligible_definitions(
        &self,
        context: &ToolEligibilityContext,
    ) -> Vec<NativeToolDefinition> {
        self.handlers
            .values()
            .filter(|handler| {
                self.eligibility(handler.definition().name.as_str(), context)
                    == ToolEligibility::Eligible
            })
            .map(|handler| project_definition(handler.definition(), context, &self.feature_gates))
            .collect()
    }

    pub(crate) fn eligibility(
        &self,
        name: &str,
        context: &ToolEligibilityContext,
    ) -> ToolEligibility {
        if is_onepiece_only(name) && context.agent_id != ONEPIECE_AGENT_ID {
            return ToolEligibility::Ineligible {
                reason: super::NativeToolReadinessReasonCode::PolicyUnavailable,
            };
        }
        if !self.feature_gates.tool_enabled(name) {
            return ToolEligibility::Ineligible {
                reason: super::NativeToolReadinessReasonCode::Disabled,
            };
        }
        if let Some(reason) = self.readiness_reason(name) {
            return ToolEligibility::Ineligible { reason };
        }
        if context.execution_mode == NativeToolExecutionMode::Plan
            && name == "artifact"
            && !self.feature_gates.enabled("artifact", "read")
        {
            return ToolEligibility::Ineligible {
                reason: super::NativeToolReadinessReasonCode::Disabled,
            };
        }
        if context.execution_mode == NativeToolExecutionMode::Plan
            && self
                .handlers
                .get(name)
                .is_some_and(|handler| !handler.definition().plan_mode_compatible)
        {
            return ToolEligibility::Ineligible {
                reason: super::NativeToolReadinessReasonCode::PolicyUnavailable,
            };
        }
        self.handlers.get(name).map_or(
            ToolEligibility::Ineligible {
                reason: super::NativeToolReadinessReasonCode::BackendUnavailable,
            },
            |handler| handler.eligibility(context),
        )
    }

    pub(crate) fn eligible_tool_definitions(
        &self,
        context: &ToolEligibilityContext,
    ) -> Vec<ToolDefinition> {
        self.eligible_definitions(context)
            .into_iter()
            .map(|definition| ToolDefinition {
                name: definition.name,
                description: definition.description,
                input_schema: definition.input_schema,
            })
            .collect()
    }

    pub(crate) fn readiness_snapshot(
        &self,
    ) -> BTreeMap<String, Option<super::NativeToolReadinessReasonCode>> {
        self.readiness.as_ref().clone()
    }

    pub(crate) fn readiness_reason(
        &self,
        name: &str,
    ) -> Option<super::NativeToolReadinessReasonCode> {
        self.readiness.get(name).copied().flatten()
    }

    #[allow(dead_code)]
    pub(crate) fn len(&self) -> usize {
        self.handlers.len()
    }

    #[allow(dead_code)]
    pub(crate) fn is_empty(&self) -> bool {
        self.handlers.is_empty()
    }

    pub(crate) fn is_registered(&self, name: &str) -> bool {
        self.handlers.contains_key(name)
    }

    pub(crate) fn is_feature_enabled(&self, capability: &str, mode: &str) -> bool {
        self.feature_gates.enabled(capability, mode)
    }

    pub(crate) fn is_operation_enabled(&self, operation: NativeToolOperation) -> bool {
        self.feature_gates.operation_enabled(operation)
    }
}

fn project_definition(
    definition: &NativeToolDefinition,
    context: &ToolEligibilityContext,
    feature_gates: &OnePieceToolFeatureGates,
) -> NativeToolDefinition {
    let mut projected = definition.clone();
    projected.operations.retain(|operation| match operation {
        NativeToolOperation::ArtifactRead => feature_gates.enabled("artifact", "read"),
        NativeToolOperation::ArtifactPublish => feature_gates.enabled("artifact", "publish"),
        _ => true,
    });
    if definition.name == "artifact" {
        let mut operations = Vec::new();
        if feature_gates.enabled("artifact", "read") {
            operations.extend(["list", "metadata", "read_text"]);
        }
        if feature_gates.enabled("artifact", "publish") {
            operations.push("publish");
        }
        if let Some(values) = projected
            .input_schema
            .pointer_mut("/properties/operation/enum")
        {
            *values = serde_json::json!(operations);
        }
    } else if definition.name == "ocr" && !feature_gates.enabled("artifact", "publish") {
        if let Some(properties) = projected
            .input_schema
            .get_mut("properties")
            .and_then(serde_json::Value::as_object_mut)
        {
            properties.remove("publish");
        }
    }
    if context.execution_mode != NativeToolExecutionMode::Plan {
        return projected;
    }
    projected.operations.retain(|operation| {
        matches!(
            operation,
            NativeToolOperation::ArtifactRead | NativeToolOperation::OcrRead
        )
    });
    if definition.name == "artifact" {
        if let Some(values) = projected
            .input_schema
            .pointer_mut("/properties/operation/enum")
        {
            *values = if feature_gates.enabled("artifact", "read") {
                serde_json::json!(["list", "metadata", "read_text"])
            } else {
                serde_json::json!([])
            };
        }
    } else if definition.name == "ocr" {
        if let Some(properties) = projected
            .input_schema
            .get_mut("properties")
            .and_then(serde_json::Value::as_object_mut)
        {
            properties.remove("publish");
        }
    }
    projected
}

pub(crate) fn is_onepiece_only(name: &str) -> bool {
    ONEPIECE_ONLY_TOOL_NAMES.contains(&name)
}

#[cfg(test)]
#[path = "registry_tests.rs"]
mod tests;
