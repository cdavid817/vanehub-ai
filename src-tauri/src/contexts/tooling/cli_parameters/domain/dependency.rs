use super::definition::{
    CliConditionOperator, CliConditionValue, CliParameterCondition, CliParameterDefinition,
};
use super::diagnostic::{CliParameterDiagnostic, CliParameterDiagnosticCode};
use super::selection::{CliParameterSelection, CliParameterSelectionMap, CliParameterValue};
use std::collections::{BTreeMap, BTreeSet};

fn condition_satisfied(
    condition: &CliParameterCondition,
    selections: &CliParameterSelectionMap,
) -> bool {
    let selection = selections
        .get(&condition.parameter_id)
        .unwrap_or(&CliParameterSelection::Inherit);
    match condition.operator {
        CliConditionOperator::NotInherit => !selection.is_inherit(),
        CliConditionOperator::Equals => match (&condition.value, selection.as_value()) {
            (
                Some(CliConditionValue::Boolean(expected)),
                Some(CliParameterValue::Boolean(actual)),
            ) => expected == actual,
            (Some(CliConditionValue::Text(expected)), Some(CliParameterValue::Text(actual))) => {
                expected == actual
            }
            _ => false,
        },
        CliConditionOperator::Contains => match (&condition.value, selection.as_value()) {
            (
                Some(CliConditionValue::Text(expected)),
                Some(CliParameterValue::TextList(actual)),
            ) => actual.contains(expected),
            (Some(CliConditionValue::Text(expected)), Some(CliParameterValue::Text(actual))) => {
                actual == expected
            }
            _ => false,
        },
    }
}

/// Evaluates declared dependencies and conflicts once over the resolved selection map. The rules
/// only read other parameters' selections, so one pass is sufficient and always terminates.
pub(crate) fn evaluate(
    agent_id: &str,
    definitions: &[CliParameterDefinition],
    selections: &CliParameterSelectionMap,
) -> Vec<CliParameterDiagnostic> {
    let explicit = definitions
        .iter()
        .filter(|definition| {
            selections
                .get(&definition.id)
                .is_some_and(|selection| !selection.is_inherit())
        })
        .map(|definition| definition.id.as_str())
        .collect::<BTreeSet<_>>();

    let mut diagnostics = BTreeMap::new();
    for definition in definitions {
        if !explicit.contains(definition.id.as_str()) {
            continue;
        }
        for condition in &definition.dependencies.requires_all {
            if condition_satisfied(condition, selections) {
                continue;
            }
            let diagnostic = CliParameterDiagnostic::new(
                CliParameterDiagnosticCode::DependencyNotSatisfied,
                agent_id,
                Some(definition.id.clone()),
            )
            .with_detail("requiredParameterId", condition.parameter_id.clone());
            diagnostics.insert(diagnostic.dedup_key(), diagnostic);
        }
        for conflicting_id in &definition.dependencies.conflicts_with {
            if !explicit.contains(conflicting_id.as_str()) {
                continue;
            }
            for (owner, other) in [
                (definition.id.as_str(), conflicting_id.as_str()),
                (conflicting_id.as_str(), definition.id.as_str()),
            ] {
                let diagnostic = CliParameterDiagnostic::new(
                    CliParameterDiagnosticCode::ConflictingSelection,
                    agent_id,
                    Some(owner.to_string()),
                )
                .with_detail("conflictsWith", other.to_string());
                diagnostics.insert(diagnostic.dedup_key(), diagnostic);
            }
        }
    }
    diagnostics.into_values().collect()
}

/// Registry-time guard: a `requiresAll` graph must be acyclic so the settings page can present a
/// deterministic repair order.
pub(crate) fn find_requires_cycle(definitions: &[CliParameterDefinition]) -> Option<String> {
    let edges = definitions
        .iter()
        .map(|definition| {
            let targets = definition
                .dependencies
                .requires_all
                .iter()
                .map(|condition| condition.parameter_id.as_str())
                .collect::<Vec<_>>();
            (definition.id.as_str(), targets)
        })
        .collect::<BTreeMap<_, _>>();

    let mut visiting = BTreeSet::new();
    let mut visited = BTreeSet::new();
    let mut stack = Vec::new();
    for start in edges.keys() {
        if visit(start, &edges, &mut visiting, &mut visited, &mut stack) {
            return Some(stack.join(" -> "));
        }
    }
    None
}

fn visit<'a>(
    node: &'a str,
    edges: &BTreeMap<&'a str, Vec<&'a str>>,
    visiting: &mut BTreeSet<&'a str>,
    visited: &mut BTreeSet<&'a str>,
    stack: &mut Vec<&'a str>,
) -> bool {
    if visited.contains(node) {
        return false;
    }
    if !visiting.insert(node) {
        stack.push(node);
        return true;
    }
    stack.push(node);
    for next in edges.get(node).into_iter().flatten() {
        if visit(next, edges, visiting, visited, stack) {
            return true;
        }
    }
    stack.pop();
    visiting.remove(node);
    visited.insert(node);
    false
}

#[cfg(test)]
mod tests {
    use super::super::definition::CliParameterDependencies;
    use super::super::testing::{boolean_definition, custom_text_definition, enum_definition};
    use super::*;

    fn requires(parameter_id: &str, value: Option<CliConditionValue>) -> CliParameterCondition {
        CliParameterCondition {
            parameter_id: parameter_id.to_string(),
            operator: if value.is_some() {
                CliConditionOperator::Equals
            } else {
                CliConditionOperator::NotInherit
            },
            value,
        }
    }

    fn catalog() -> Vec<CliParameterDefinition> {
        let mut oss = boolean_definition();
        oss.id = "oss".to_string();
        let mut local_provider = enum_definition();
        local_provider.id = "localProvider".to_string();
        local_provider.dependencies = CliParameterDependencies {
            requires_all: vec![requires("oss", Some(CliConditionValue::Boolean(true)))],
            conflicts_with: Vec::new(),
        };
        vec![oss, local_provider]
    }

    #[test]
    fn an_unsatisfied_requirement_produces_a_blocking_field_diagnostic() {
        let selections = CliParameterSelectionMap::from([(
            "localProvider".to_string(),
            CliParameterSelection::text("ollama"),
        )]);
        let diagnostics = evaluate("codex-cli", &catalog(), &selections);
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(
            diagnostics[0].code,
            CliParameterDiagnosticCode::DependencyNotSatisfied
        );
        assert_eq!(
            diagnostics[0].parameter_id.as_deref(),
            Some("localProvider")
        );
        assert!(diagnostics[0].blocking);
        assert_eq!(
            diagnostics[0]
                .details
                .get("requiredParameterId")
                .map(String::as_str),
            Some("oss")
        );
    }

    #[test]
    fn a_satisfied_requirement_produces_nothing() {
        let selections = CliParameterSelectionMap::from([
            ("oss".to_string(), CliParameterSelection::boolean(true)),
            (
                "localProvider".to_string(),
                CliParameterSelection::text("ollama"),
            ),
        ]);
        assert!(evaluate("codex-cli", &catalog(), &selections).is_empty());
    }

    #[test]
    fn an_inherited_dependent_is_not_evaluated() {
        let selections = CliParameterSelectionMap::from([(
            "localProvider".to_string(),
            CliParameterSelection::Inherit,
        )]);
        assert!(evaluate("codex-cli", &catalog(), &selections).is_empty());
    }

    #[test]
    fn a_conflict_marks_both_affected_fields() {
        let mut first = custom_text_definition();
        first.id = "first".to_string();
        first.dependencies.conflicts_with = vec!["second".to_string()];
        let mut second = custom_text_definition();
        second.id = "second".to_string();
        let selections = CliParameterSelectionMap::from([
            ("first".to_string(), CliParameterSelection::text("sonnet")),
            ("second".to_string(), CliParameterSelection::text("opus")),
        ]);
        let diagnostics = evaluate("claude-code", &[first, second], &selections);
        let affected = diagnostics
            .iter()
            .filter_map(|entry| entry.parameter_id.as_deref())
            .collect::<Vec<_>>();
        assert_eq!(affected, ["first", "second"]);
        assert!(diagnostics.iter().all(|entry| entry.blocking));
    }

    #[test]
    fn the_contains_operator_reads_list_membership() {
        let condition = requires("extensions", Some(CliConditionValue::Text("a".to_string())));
        let condition = CliParameterCondition {
            operator: CliConditionOperator::Contains,
            ..condition
        };
        let selections = CliParameterSelectionMap::from([(
            "extensions".to_string(),
            CliParameterSelection::text_list(vec!["a".to_string(), "b".to_string()]),
        )]);
        assert!(condition_satisfied(&condition, &selections));
        let other = CliParameterSelectionMap::from([(
            "extensions".to_string(),
            CliParameterSelection::text_list(vec!["b".to_string()]),
        )]);
        assert!(!condition_satisfied(&condition, &other));
    }

    #[test]
    fn a_requires_cycle_is_detected_for_registry_validation() {
        let mut first = custom_text_definition();
        first.id = "first".to_string();
        first.dependencies.requires_all = vec![requires("second", None)];
        let mut second = custom_text_definition();
        second.id = "second".to_string();
        second.dependencies.requires_all = vec![requires("first", None)];
        assert!(find_requires_cycle(&[first, second]).is_some());
        assert!(find_requires_cycle(&catalog()).is_none());
    }
}
