use serde::{Deserialize, Serialize};

pub(crate) const MIN_SUPPORTED_PYTHON: (u16, u16) = (3, 9);
pub(crate) const MAX_SUPPORTED_PYTHON: (u16, u16) = (3, 13);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum PythonDiscoveryAvailability {
    Available,
    Unavailable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum PythonDiscoverySource {
    Configured,
    Path,
    WindowsLauncher,
}

impl PythonDiscoverySource {
    pub(crate) fn priority(self) -> u8 {
        match self {
            Self::Configured => 0,
            Self::WindowsLauncher => 1,
            Self::Path => 2,
        }
    }

    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Configured => "configured",
            Self::Path => "path",
            Self::WindowsLauncher => "windows_launcher",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum PythonCompatibility {
    Compatible,
    Unsupported,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum PythonDiscoveryReason {
    ManualConfigurationRequired,
    NativeUnavailable,
    UnsupportedVersion,
}

impl PythonDiscoveryReason {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::ManualConfigurationRequired => "manual_configuration_required",
            Self::NativeUnavailable => "native_unavailable",
            Self::UnsupportedVersion => "unsupported_version",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PythonVersion {
    pub(crate) major: u16,
    pub(crate) minor: u16,
    pub(crate) patch: u16,
}

impl PythonVersion {
    pub(crate) fn compatibility(self) -> PythonCompatibility {
        let line = (self.major, self.minor);
        if (MIN_SUPPORTED_PYTHON..=MAX_SUPPORTED_PYTHON).contains(&line) {
            PythonCompatibility::Compatible
        } else {
            PythonCompatibility::Unsupported
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PythonEnvironmentCandidate {
    pub(crate) executable_path: String,
    pub(crate) version: PythonVersion,
    pub(crate) compatibility: PythonCompatibility,
    pub(crate) reason_code: Option<PythonDiscoveryReason>,
    pub(crate) source: PythonDiscoverySource,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PythonEnvironmentDiscovery {
    pub(crate) availability: PythonDiscoveryAvailability,
    pub(crate) reason_code: Option<PythonDiscoveryReason>,
    pub(crate) candidates: Vec<PythonEnvironmentCandidate>,
}

impl PythonEnvironmentDiscovery {
    pub(crate) fn available(candidates: Vec<PythonEnvironmentCandidate>) -> Self {
        let has_compatible = candidates
            .iter()
            .any(|candidate| candidate.compatibility == PythonCompatibility::Compatible);
        Self {
            availability: PythonDiscoveryAvailability::Available,
            reason_code: (!has_compatible)
                .then_some(PythonDiscoveryReason::ManualConfigurationRequired),
            candidates,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compatibility_policy_is_inclusive_and_bounded() {
        assert_eq!(
            PythonVersion {
                major: 3,
                minor: 9,
                patch: 0
            }
            .compatibility(),
            PythonCompatibility::Compatible
        );
        assert_eq!(
            PythonVersion {
                major: 3,
                minor: 13,
                patch: 9
            }
            .compatibility(),
            PythonCompatibility::Compatible
        );
        assert_eq!(
            PythonVersion {
                major: 3,
                minor: 14,
                patch: 0
            }
            .compatibility(),
            PythonCompatibility::Unsupported
        );
    }

    #[test]
    fn empty_discovery_directs_the_user_to_manual_configuration() {
        let result = PythonEnvironmentDiscovery::available(Vec::new());
        assert_eq!(
            result.reason_code,
            Some(PythonDiscoveryReason::ManualConfigurationRequired)
        );
    }
}
