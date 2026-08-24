use super::definition::{
    CliParameterCompatibility, CliParameterDefinition, CliParameterOption, CliParameterPlatform,
};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

/// The narrow read-only view of CLI lifecycle detection this subdomain consumes. It never
/// triggers detection; the owning subdomain refreshes it.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CliInstallationSnapshot {
    pub(crate) installed: bool,
    pub(crate) runnable: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) active_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) version: Option<String>,
    pub(crate) conflict: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "kebab-case")]
pub(crate) enum CliParameterSupport {
    Supported,
    NotInstalled,
    #[serde(rename_all = "camelCase")]
    UnknownVersion {
        #[serde(skip_serializing_if = "Option::is_none")]
        required_range: Option<String>,
    },
    #[serde(rename_all = "camelCase")]
    UnsupportedVersion {
        installed_version: String,
        required_range: String,
    },
    #[serde(rename_all = "camelCase")]
    UnsupportedPlatform {
        platform: String,
    },
}

impl CliParameterSupport {
    pub(crate) fn is_supported(&self) -> bool {
        matches!(self, Self::Supported)
    }

    /// A launch omits a selection only when the active installation proves it cannot work.
    /// An unknown version is not proof, so the value is still rendered.
    pub(crate) fn blocks_launch(&self) -> bool {
        matches!(
            self,
            Self::UnsupportedVersion { .. } | Self::UnsupportedPlatform { .. }
        )
    }

    /// New values may not be selected while compatibility cannot be confirmed.
    pub(crate) fn blocks_new_value(&self) -> bool {
        !self.is_supported()
    }
}

/// Version ordering is owned by the CLI lifecycle subdomain; this port lets the domain reuse it
/// without importing another subdomain's module.
pub(crate) trait CliVersionComparator: Send + Sync {
    fn compare(&self, left: &str, right: &str) -> Option<Ordering>;
}

fn evaluate_range(
    compatibility: &CliParameterCompatibility,
    snapshot: &CliInstallationSnapshot,
    platform: CliParameterPlatform,
    comparator: &dyn CliVersionComparator,
) -> CliParameterSupport {
    if !compatibility.platforms.contains(&platform) {
        return CliParameterSupport::UnsupportedPlatform {
            platform: platform.as_str().to_string(),
        };
    }
    if !compatibility.is_version_gated() {
        return CliParameterSupport::Supported;
    }
    if !snapshot.installed {
        return CliParameterSupport::NotInstalled;
    }
    let Some(version) = snapshot.version.as_deref() else {
        return CliParameterSupport::UnknownVersion {
            required_range: compatibility.required_range(),
        };
    };
    let required_range = compatibility.required_range().unwrap_or_default();
    if let Some(min_version) = &compatibility.min_version {
        match comparator.compare(version, min_version) {
            Some(Ordering::Less) => {
                return CliParameterSupport::UnsupportedVersion {
                    installed_version: version.to_string(),
                    required_range,
                }
            }
            None => {
                return CliParameterSupport::UnknownVersion {
                    required_range: compatibility.required_range(),
                }
            }
            _ => {}
        }
    }
    if let Some(max_version) = &compatibility.max_version {
        match comparator.compare(version, max_version) {
            Some(Ordering::Greater) => {
                return CliParameterSupport::UnsupportedVersion {
                    installed_version: version.to_string(),
                    required_range,
                }
            }
            None => {
                return CliParameterSupport::UnknownVersion {
                    required_range: compatibility.required_range(),
                }
            }
            _ => {}
        }
    }
    CliParameterSupport::Supported
}

pub(crate) fn evaluate_definition(
    definition: &CliParameterDefinition,
    snapshot: &CliInstallationSnapshot,
    platform: CliParameterPlatform,
    comparator: &dyn CliVersionComparator,
) -> CliParameterSupport {
    evaluate_range(&definition.compatibility, snapshot, platform, comparator)
}

pub(crate) fn evaluate_option(
    definition: &CliParameterDefinition,
    option: &CliParameterOption,
    snapshot: &CliInstallationSnapshot,
    platform: CliParameterPlatform,
    comparator: &dyn CliVersionComparator,
) -> CliParameterSupport {
    let definition_support = evaluate_definition(definition, snapshot, platform, comparator);
    if !definition_support.is_supported() {
        return definition_support;
    }
    match &option.compatibility {
        Some(compatibility) => evaluate_range(compatibility, snapshot, platform, comparator),
        None => CliParameterSupport::Supported,
    }
}

#[cfg(test)]
pub(crate) struct DottedVersionComparator;

#[cfg(test)]
impl CliVersionComparator for DottedVersionComparator {
    fn compare(&self, left: &str, right: &str) -> Option<Ordering> {
        let parse = |value: &str| -> Option<Vec<u64>> {
            value
                .split('-')
                .next()?
                .split('.')
                .map(|part| part.parse::<u64>().ok())
                .collect()
        };
        let (left, right) = (parse(left)?, parse(right)?);
        let width = left.len().max(right.len());
        for index in 0..width {
            let ordering = left
                .get(index)
                .copied()
                .unwrap_or(0)
                .cmp(&right.get(index).copied().unwrap_or(0));
            if ordering != Ordering::Equal {
                return Some(ordering);
            }
        }
        Some(Ordering::Equal)
    }
}

#[cfg(test)]
mod tests {
    use super::super::testing::{boolean_definition, enum_definition};
    use super::*;

    fn installed(version: Option<&str>) -> CliInstallationSnapshot {
        CliInstallationSnapshot {
            installed: true,
            runnable: true,
            active_path: Some("/usr/bin/claude".to_string()),
            version: version.map(str::to_string),
            conflict: false,
        }
    }

    fn gated(min: &str) -> CliParameterDefinition {
        let mut definition = boolean_definition();
        definition.compatibility.min_version = Some(min.to_string());
        definition
    }

    #[test]
    fn an_ungated_parameter_is_supported_even_without_a_known_version() {
        let definition = boolean_definition();
        for snapshot in [
            installed(None),
            installed(Some("nightly")),
            CliInstallationSnapshot::default(),
        ] {
            assert_eq!(
                evaluate_definition(
                    &definition,
                    &snapshot,
                    CliParameterPlatform::Linux,
                    &DottedVersionComparator
                ),
                CliParameterSupport::Supported
            );
        }
    }

    #[test]
    fn a_gated_parameter_reports_each_version_state() {
        let definition = gated("2.1.181");
        let comparator = &DottedVersionComparator;
        let platform = CliParameterPlatform::Linux;
        assert_eq!(
            evaluate_definition(
                &definition,
                &installed(Some("2.1.181")),
                platform,
                comparator
            ),
            CliParameterSupport::Supported
        );
        assert_eq!(
            evaluate_definition(&definition, &installed(Some("2.2.0")), platform, comparator),
            CliParameterSupport::Supported
        );
        assert_eq!(
            evaluate_definition(
                &definition,
                &installed(Some("2.1.180")),
                platform,
                comparator
            ),
            CliParameterSupport::UnsupportedVersion {
                installed_version: "2.1.180".to_string(),
                required_range: ">= 2.1.181".to_string(),
            }
        );
        assert_eq!(
            evaluate_definition(&definition, &installed(None), platform, comparator),
            CliParameterSupport::UnknownVersion {
                required_range: Some(">= 2.1.181".to_string()),
            }
        );
        assert_eq!(
            evaluate_definition(&definition, &installed(Some("weird")), platform, comparator),
            CliParameterSupport::UnknownVersion {
                required_range: Some(">= 2.1.181".to_string()),
            }
        );
        assert_eq!(
            evaluate_definition(
                &definition,
                &CliInstallationSnapshot::default(),
                platform,
                comparator
            ),
            CliParameterSupport::NotInstalled
        );
    }

    #[test]
    fn a_prerelease_below_the_gate_is_unsupported() {
        let definition = gated("2.1.181");
        assert_eq!(
            evaluate_definition(
                &definition,
                &installed(Some("2.1.180-beta.1")),
                CliParameterPlatform::Linux,
                &DottedVersionComparator
            ),
            CliParameterSupport::UnsupportedVersion {
                installed_version: "2.1.180-beta.1".to_string(),
                required_range: ">= 2.1.181".to_string(),
            }
        );
    }

    #[test]
    fn a_platform_gate_is_evaluated_before_any_version_gate() {
        let mut definition = gated("2.1.181");
        definition.compatibility.platforms = vec![CliParameterPlatform::Windows];
        assert_eq!(
            evaluate_definition(
                &definition,
                &installed(Some("9.9.9")),
                CliParameterPlatform::Linux,
                &DottedVersionComparator
            ),
            CliParameterSupport::UnsupportedPlatform {
                platform: "linux".to_string(),
            }
        );
    }

    #[test]
    fn an_option_gate_narrows_a_supported_definition() {
        let mut definition = enum_definition();
        definition.options[2].compatibility = Some(CliParameterCompatibility {
            min_version: Some("2.1.203".to_string()),
            ..CliParameterCompatibility::default()
        });
        let snapshot = installed(Some("2.1.190"));
        let platform = CliParameterPlatform::Linux;
        assert_eq!(
            evaluate_option(
                &definition,
                &definition.options[0],
                &snapshot,
                platform,
                &DottedVersionComparator
            ),
            CliParameterSupport::Supported
        );
        assert!(matches!(
            evaluate_option(
                &definition,
                &definition.options[2],
                &snapshot,
                platform,
                &DottedVersionComparator
            ),
            CliParameterSupport::UnsupportedVersion { .. }
        ));
    }

    #[test]
    fn unknown_version_does_not_block_a_launch_but_blocks_a_new_value() {
        let unknown = CliParameterSupport::UnknownVersion {
            required_range: None,
        };
        assert!(!unknown.blocks_launch());
        assert!(unknown.blocks_new_value());
        let unsupported = CliParameterSupport::UnsupportedVersion {
            installed_version: "1.0.0".to_string(),
            required_range: ">= 2.0.0".to_string(),
        };
        assert!(unsupported.blocks_launch());
        assert!(unsupported.blocks_new_value());
        assert!(!CliParameterSupport::Supported.blocks_launch());
    }
}
