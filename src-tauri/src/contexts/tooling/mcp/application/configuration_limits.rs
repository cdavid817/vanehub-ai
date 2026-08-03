use super::{McpApplicationError, McpLimits};
use crate::contexts::tooling::mcp::domain::{ServerConfiguration, ServerConfigurationDraft};
use serde::Serialize;
use std::collections::BTreeMap;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct TransportConfiguration<'a> {
    transport_type: &'static str,
    command: Option<&'a str>,
    args: Option<&'a [String]>,
    env: Option<&'a BTreeMap<String, String>>,
    url: Option<&'a str>,
    headers: Option<&'a BTreeMap<String, String>>,
}

pub(super) fn validate_draft(draft: &ServerConfigurationDraft) -> Result<(), McpApplicationError> {
    validate(
        draft.transport_type.as_str(),
        draft.command.as_deref(),
        draft.args.as_deref(),
        draft.env.as_ref(),
        draft.url.as_deref(),
        draft.headers.as_ref(),
    )
}

pub(super) fn validate_server(server: &ServerConfiguration) -> Result<(), McpApplicationError> {
    validate(
        server.transport_type().as_str(),
        server.command(),
        server.args(),
        server.env(),
        server.url(),
        server.headers(),
    )
}

fn validate(
    transport_type: &'static str,
    command: Option<&str>,
    args: Option<&[String]>,
    env: Option<&BTreeMap<String, String>>,
    url: Option<&str>,
    headers: Option<&BTreeMap<String, String>>,
) -> Result<(), McpApplicationError> {
    let limits = McpLimits::DEFAULT;
    for count in [
        args.map_or(0, <[String]>::len),
        env.map_or(0, BTreeMap::len),
        headers.map_or(0, BTreeMap::len),
    ] {
        if count > limits.configuration_collection_entries {
            return Err(McpApplicationError::LimitExceeded);
        }
    }
    let configuration = TransportConfiguration {
        transport_type,
        command,
        args,
        env,
        url,
        headers,
    };
    let size = serde_json::to_vec(&configuration)
        .map_err(|error| McpApplicationError::Validation(error.to_string()))?
        .len();
    if size > limits.configuration_serialized_bytes {
        Err(McpApplicationError::LimitExceeded)
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contexts::tooling::mcp::domain::{Scope, TransportType};

    fn draft() -> ServerConfigurationDraft {
        ServerConfigurationDraft {
            name: "limit-fixture".to_string(),
            transport_type: TransportType::Stdio,
            command: Some("node".to_string()),
            args: None,
            env: None,
            url: None,
            headers: None,
            description: None,
            active: true,
            scope: Scope::User,
            project_path: None,
        }
    }

    #[test]
    fn collection_limits_accept_exact_boundary_and_reject_limit_plus_one() {
        let maximum = McpLimits::DEFAULT.configuration_collection_entries;
        for field in ["args", "env", "headers"] {
            let mut exact = draft();
            set_collection(&mut exact, field, maximum);
            assert!(validate_draft(&exact).is_ok(), "{field} exact boundary");

            let mut oversized = draft();
            set_collection(&mut oversized, field, maximum + 1);
            assert_eq!(
                validate_draft(&oversized),
                Err(McpApplicationError::LimitExceeded),
                "{field} limit plus one"
            );
        }
    }

    #[test]
    fn serialized_limit_accepts_exact_boundary_and_rejects_limit_plus_one() {
        let maximum = McpLimits::DEFAULT.configuration_serialized_bytes;
        let mut exact = draft();
        exact.env = Some(BTreeMap::from([("VALUE".to_string(), String::new())]));
        let base = serialized_size(&exact);
        exact.env.as_mut().expect("env").insert(
            "VALUE".to_string(),
            "x".repeat(maximum.saturating_sub(base)),
        );
        assert_eq!(serialized_size(&exact), maximum);
        assert!(validate_draft(&exact).is_ok());

        exact
            .env
            .as_mut()
            .expect("env")
            .get_mut("VALUE")
            .expect("value")
            .push('x');
        assert_eq!(serialized_size(&exact), maximum + 1);
        assert_eq!(
            validate_draft(&exact),
            Err(McpApplicationError::LimitExceeded)
        );
    }

    fn set_collection(draft: &mut ServerConfigurationDraft, field: &str, count: usize) {
        match field {
            "args" => draft.args = Some((0..count).map(|index| index.to_string()).collect()),
            "env" => {
                draft.env = Some(
                    (0..count)
                        .map(|index| (format!("KEY_{index}"), String::new()))
                        .collect(),
                )
            }
            "headers" => {
                draft.headers = Some(
                    (0..count)
                        .map(|index| (format!("x-key-{index}"), String::new()))
                        .collect(),
                )
            }
            _ => unreachable!(),
        }
    }

    fn serialized_size(draft: &ServerConfigurationDraft) -> usize {
        serde_json::to_vec(&TransportConfiguration {
            transport_type: draft.transport_type.as_str(),
            command: draft.command.as_deref(),
            args: draft.args.as_deref(),
            env: draft.env.as_ref(),
            url: draft.url.as_deref(),
            headers: draft.headers.as_ref(),
        })
        .expect("serialize")
        .len()
    }
}
