// No production caller yet; see `identity.rs`.
#![cfg_attr(not(test), allow(dead_code))]

//! Decoding the `contributes` block.
//!
//! Split from `manifest_decoder.rs` because nine contribution kinds in one file buries the
//! top-level shape. Each kind is read the same way: the collection is keyed by id, the key becomes
//! the `ContributionLocalId`, and the value is a mapping read field by field with `finish`
//! refusing leftovers.

use super::decode_reader::{bound, MappingReader};
use super::manifest_decoder::MAX_CONTRIBUTIONS_PER_KIND;
use super::{
    identifier_at, path_at, AuthorizationRuleContribution, ConfigurationContribution,
    ConnectorContribution, ContributedRuleEffect, ContributionLocalId, ContributionManifest,
    DecodeReason, HookContribution, HookFailureMode, HookHandlerDeclaration, ManifestDecodeError,
    McpContribution, McpTransportDeclaration, ModePresetContribution, PortablePackagePath,
    ToolContribution, TransformContribution,
};
use vanehub_bounded_yaml::BoundedYamlValue;

pub(super) fn decode_contributions(
    root: &mut MappingReader<'_>,
) -> Result<ContributionManifest, ManifestDecodeError> {
    let path = root.child_path("contributes");
    let Some(value) = root.optional_value("contributes") else {
        return Ok(ContributionManifest::default());
    };
    let mut reader = MappingReader::open(path, value)?;

    let tools = decode_kind(&mut reader, "tools", decode_tool)?;
    let skills = decode_kind(&mut reader, "skills", decode_skill)?;
    let mcp_definitions = decode_kind(&mut reader, "mcp_definitions", decode_mcp)?;
    let modes = decode_kind(&mut reader, "modes", decode_mode)?;
    let hooks = decode_kind(&mut reader, "hooks", decode_hook)?;
    let authorization_rules = decode_kind(&mut reader, "authorization_rules", decode_rule)?;
    let connectors = decode_kind(&mut reader, "connectors", decode_connector)?;
    let configuration = decode_kind(&mut reader, "configuration", decode_configuration)?;
    let transforms = decode_kind(&mut reader, "transforms", decode_transform)?;

    reader.finish()?;
    Ok(ContributionManifest {
        tools,
        skills,
        mcp_definitions,
        modes,
        hooks,
        authorization_rules,
        connectors,
        configuration,
        transforms,
    })
}

/// Reads one id-keyed collection.
///
/// The id comes from the key, so it cannot be repeated: the parser already rejected duplicate
/// keys before this code ran. That is the whole reason the format is keyed rather than a list.
fn decode_kind<T>(
    reader: &mut MappingReader<'_>,
    field: &str,
    decode_entry: fn(ContributionLocalId, MappingReader<'_>) -> Result<T, ManifestDecodeError>,
) -> Result<Vec<T>, ManifestDecodeError> {
    let collection_path = reader.child_path(field);
    let entries = reader.keyed_collection(field)?;
    let mut decoded = Vec::with_capacity(entries.len());
    for (id_text, value) in entries {
        let entry_path = format!("{collection_path}.{id_text}");
        let id = ContributionLocalId::parse(id_text)
            .map_err(|error| identifier_at(&entry_path, &error))?;
        let entry_reader = MappingReader::open(entry_path, value)?;
        decoded.push(decode_entry(id, entry_reader)?);
    }
    bound(&collection_path, decoded, MAX_CONTRIBUTIONS_PER_KIND)
}

fn required_path(
    reader: &mut MappingReader<'_>,
    field: &str,
) -> Result<PortablePackagePath, ManifestDecodeError> {
    let path = reader.child_path(field);
    let text = reader.required_scalar(field)?;
    PortablePackagePath::parse(text).map_err(|error| path_at(path, &error))
}

fn optional_path(
    reader: &mut MappingReader<'_>,
    field: &str,
) -> Result<Option<PortablePackagePath>, ManifestDecodeError> {
    let path = reader.child_path(field);
    match reader.optional_scalar(field)? {
        Some(text) => PortablePackagePath::parse(text)
            .map(Some)
            .map_err(|error| path_at(path, &error)),
        None => Ok(None),
    }
}

/// A matcher is an open set of predicate names owned by the Hook or rule engine, so it is carried
/// through as name/values pairs rather than interpreted here. Each value list is bounded.
fn decode_matcher(
    reader: &mut MappingReader<'_>,
) -> Result<Vec<(String, Vec<String>)>, ManifestDecodeError> {
    let path = reader.child_path("matcher");
    let Some(value) = reader.optional_value("matcher") else {
        return Ok(Vec::new());
    };
    let Some(entries) = value.as_mapping() else {
        return Err(ManifestDecodeError::new(
            path,
            DecodeReason::ExpectedMapping,
        ));
    };
    let mut matcher = Vec::with_capacity(entries.len());
    for (key, child) in entries {
        let field = format!("{path}.{key}");
        let values = match child {
            BoundedYamlValue::Scalar(single) => vec![single.clone()],
            BoundedYamlValue::Sequence(items) => items.clone(),
            BoundedYamlValue::Mapping(_) => {
                return Err(ManifestDecodeError::new(
                    field,
                    DecodeReason::ExpectedScalarSequence,
                ))
            }
        };
        matcher.push((key.clone(), values));
    }
    Ok(matcher)
}

fn decode_tool(
    id: ContributionLocalId,
    mut reader: MappingReader<'_>,
) -> Result<ToolContribution, ManifestDecodeError> {
    let display_name = reader.required_scalar("display_name")?.to_string();
    let description = reader.optional_scalar("description")?.map(str::to_string);
    let input_schema = optional_path(&mut reader, "input_schema")?;
    let output_schema = optional_path(&mut reader, "output_schema")?;
    let handler = reader.required_scalar("handler")?.to_string();
    reader.finish()?;
    Ok(ToolContribution {
        id,
        display_name,
        description,
        input_schema,
        output_schema,
        handler,
    })
}

fn decode_skill(
    id: ContributionLocalId,
    mut reader: MappingReader<'_>,
) -> Result<SkillContributionOut, ManifestDecodeError> {
    let path = required_path(&mut reader, "path")?;
    reader.finish()?;
    Ok(super::SkillContribution { id, path })
}

type SkillContributionOut = super::SkillContribution;

fn decode_mcp(
    id: ContributionLocalId,
    mut reader: MappingReader<'_>,
) -> Result<McpContribution, ManifestDecodeError> {
    let display_name = reader.required_scalar("display_name")?.to_string();
    let transport_path = reader.child_path("transport");
    let transport_value = reader.required_value("transport")?;
    let mut transport_reader = MappingReader::open(transport_path.clone(), transport_value)?;

    let kind = transport_reader.required_scalar("kind")?;
    let transport = match kind {
        "stdio" => {
            let command = transport_reader.required_scalar("command")?.to_string();
            let args_path = transport_reader.child_path("args");
            let args = bound(
                &args_path,
                transport_reader.scalar_sequence("args")?,
                super::manifest_decoder::MAX_CAPABILITY_ENTRIES,
            )?;
            let env_path = transport_reader.child_path("env_keys");
            let env_keys = bound(
                &env_path,
                transport_reader.scalar_sequence("env_keys")?,
                super::manifest_decoder::MAX_CAPABILITY_ENTRIES,
            )?;
            McpTransportDeclaration::Stdio {
                command,
                args: args.into_iter().map(str::to_string).collect(),
                // Names only. A value here would be a secret shipped inside a package, which the
                // manifest has no representation for on purpose.
                env_keys: env_keys.into_iter().map(str::to_string).collect(),
            }
        }
        "http" => {
            let url = transport_reader.required_scalar("url")?.to_string();
            let headers_path = transport_reader.child_path("header_keys");
            let header_keys = bound(
                &headers_path,
                transport_reader.scalar_sequence("header_keys")?,
                super::manifest_decoder::MAX_CAPABILITY_ENTRIES,
            )?;
            McpTransportDeclaration::Http {
                url,
                header_keys: header_keys.into_iter().map(str::to_string).collect(),
            }
        }
        _ => {
            return Err(ManifestDecodeError::new(
                transport_reader.child_path("kind"),
                DecodeReason::UnknownValue {
                    expected: "stdio, http",
                },
            ))
        }
    };
    transport_reader.finish()?;
    reader.finish()?;
    Ok(McpContribution {
        id,
        display_name,
        transport,
    })
}

fn decode_mode(
    id: ContributionLocalId,
    mut reader: MappingReader<'_>,
) -> Result<ModePresetContribution, ManifestDecodeError> {
    let display_name = reader.required_scalar("display_name")?.to_string();
    // Referenced by name only; whether the strategy is registered is resolved against the
    // application registry, not here.
    let strategy = reader.required_scalar("strategy")?.to_string();
    let default_policy_template = reader
        .optional_scalar("default_policy_template")?
        .map(str::to_string);
    let groups_path = reader.child_path("required_tool_groups");
    let required_tool_groups = bound(
        &groups_path,
        reader.scalar_sequence("required_tool_groups")?,
        super::manifest_decoder::MAX_CAPABILITY_ENTRIES,
    )?;
    let skills_path = reader.child_path("required_skills");
    let required_skills = bound(
        &skills_path,
        reader.scalar_sequence("required_skills")?,
        super::manifest_decoder::MAX_CAPABILITY_ENTRIES,
    )?;
    let hooks_path = reader.child_path("required_hooks");
    let raw_hooks = bound(
        &hooks_path,
        reader.scalar_sequence("required_hooks")?,
        super::manifest_decoder::MAX_CAPABILITY_ENTRIES,
    )?;
    let required_hooks = raw_hooks
        .into_iter()
        .map(|text| {
            ContributionLocalId::parse(text).map_err(|error| identifier_at(&hooks_path, &error))
        })
        .collect::<Result<Vec<_>, _>>()?;
    reader.finish()?;
    Ok(ModePresetContribution {
        id,
        display_name,
        strategy,
        default_policy_template,
        required_tool_groups: required_tool_groups
            .into_iter()
            .map(str::to_string)
            .collect(),
        required_skills: required_skills.into_iter().map(str::to_string).collect(),
        required_hooks,
    })
}

fn decode_hook(
    id: ContributionLocalId,
    mut reader: MappingReader<'_>,
) -> Result<HookContribution, ManifestDecodeError> {
    let event = reader.required_scalar("event")?.to_string();
    let matcher = decode_matcher(&mut reader)?;

    let handler_path = reader.child_path("handler");
    let handler_value = reader.required_value("handler")?;
    let mut handler_reader = MappingReader::open(handler_path, handler_value)?;
    let handler_kind = handler_reader.required_scalar("kind")?;
    let handler = match handler_kind {
        "extension-runtime" => HookHandlerDeclaration::ExtensionRuntime {
            entry: handler_reader.required_scalar("entry")?.to_string(),
        },
        "mcp-tool" => HookHandlerDeclaration::McpTool {
            tool: handler_reader.required_scalar("tool")?.to_string(),
        },
        _ => {
            return Err(ManifestDecodeError::new(
                handler_reader.child_path("kind"),
                DecodeReason::UnknownValue {
                    // Command, HTTP, prompt, and Agent handlers are configured locally by an
                    // operator, not shipped inside a downloaded package.
                    expected: "extension-runtime, mcp-tool",
                },
            ));
        }
    };
    handler_reader.finish()?;

    let failure_path = reader.child_path("failure_mode");
    let failure_mode = match reader.optional_scalar("failure_mode")? {
        Some(text) => HookFailureMode::parse(text).ok_or_else(|| {
            ManifestDecodeError::new(
                failure_path,
                DecodeReason::UnknownValue {
                    expected: "fail_closed, fail_open",
                },
            )
        })?,
        // Silence means fail closed. A Hook that fails open by default would let a broken
        // handler quietly stop enforcing whatever it was added to enforce.
        None => HookFailureMode::FailClosed,
    };

    let priority = decode_priority(&mut reader)?;
    reader.finish()?;
    Ok(HookContribution {
        id,
        event,
        matcher,
        handler,
        failure_mode,
        priority,
    })
}

fn decode_priority(reader: &mut MappingReader<'_>) -> Result<i32, ManifestDecodeError> {
    let path = reader.child_path("priority");
    match reader.optional_scalar("priority")? {
        Some(text) => text
            .parse::<i32>()
            .map_err(|_| ManifestDecodeError::new(path, DecodeReason::ExpectedScalar)),
        None => Ok(0),
    }
}

fn decode_rule(
    id: ContributionLocalId,
    mut reader: MappingReader<'_>,
) -> Result<AuthorizationRuleContribution, ManifestDecodeError> {
    let operation = reader.required_scalar("operation")?.to_string();
    let matcher = decode_matcher(&mut reader)?;

    let effect_path = reader.child_path("effect");
    let effect_text = reader.required_scalar("effect")?;
    let effect = ContributedRuleEffect::parse(effect_text).ok_or_else(|| {
        ManifestDecodeError::new(
            effect_path,
            // `allow` is deliberately absent from the accepted set: a downloaded package may
            // tighten policy and never loosen it.
            DecodeReason::UnknownValue {
                expected: "ask, deny",
            },
        )
    })?;

    let risk = reader.required_scalar("risk")?.to_string();
    let scopes_path = reader.child_path("allowed_scopes");
    let allowed_scopes = bound(
        &scopes_path,
        reader.scalar_sequence("allowed_scopes")?,
        super::manifest_decoder::MAX_CAPABILITY_ENTRIES,
    )?;
    reader.finish()?;
    Ok(AuthorizationRuleContribution {
        id,
        operation,
        matcher,
        effect,
        risk,
        allowed_scopes: allowed_scopes.into_iter().map(str::to_string).collect(),
    })
}

fn decode_connector(
    id: ContributionLocalId,
    mut reader: MappingReader<'_>,
) -> Result<ConnectorContribution, ManifestDecodeError> {
    let display_name = reader.required_scalar("display_name")?.to_string();
    let connector_type = reader.required_scalar("type")?.to_string();
    let driver = reader.required_scalar("driver")?.to_string();
    let auth_strategy = reader.required_scalar("auth_strategy")?.to_string();
    let capabilities_path = reader.child_path("capabilities");
    let capabilities = bound(
        &capabilities_path,
        reader.scalar_sequence("capabilities")?,
        super::manifest_decoder::MAX_CAPABILITY_ENTRIES,
    )?;
    reader.finish()?;
    Ok(ConnectorContribution {
        id,
        display_name,
        connector_type,
        driver,
        auth_strategy,
        capabilities: capabilities.into_iter().map(str::to_string).collect(),
    })
}

fn decode_configuration(
    id: ContributionLocalId,
    mut reader: MappingReader<'_>,
) -> Result<ConfigurationContribution, ManifestDecodeError> {
    let schema = required_path(&mut reader, "schema")?;
    reader.finish()?;
    Ok(ConfigurationContribution { id, schema })
}

fn decode_transform(
    id: ContributionLocalId,
    mut reader: MappingReader<'_>,
) -> Result<TransformContribution, ManifestDecodeError> {
    let event = reader.required_scalar("event")?.to_string();
    let handler = reader.required_scalar("handler")?.to_string();
    reader.finish()?;
    Ok(TransformContribution { id, event, handler })
}
