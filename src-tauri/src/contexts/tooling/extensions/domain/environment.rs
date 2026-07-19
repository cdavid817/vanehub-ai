const PYTHON_MIN_MAJOR: u32 = 3;
const PYTHON_MIN_MINOR: u32 = 10;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PythonRuntime {
    pub(crate) path: String,
    pub(crate) version: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct HostEnvironment {
    pub(crate) os: String,
    pub(crate) arch: String,
    pub(crate) python: Option<PythonRuntime>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ExtensionEnvironmentReason {
    WindowsX64Only,
    PythonMissing,
    PythonVersion,
}

impl ExtensionEnvironmentReason {
    pub(crate) fn message_key(self) -> &'static str {
        match self {
            Self::WindowsX64Only => "extensions.environment.windowsX64Only",
            Self::PythonMissing => "extensions.environment.pythonMissing",
            Self::PythonVersion => "extensions.environment.pythonVersion",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ExtensionEnvironment {
    pub(crate) runtime: &'static str,
    pub(crate) os: String,
    pub(crate) arch: String,
    pub(crate) supported: bool,
    pub(crate) native_operations_available: bool,
    pub(crate) python: Option<PythonRuntime>,
    pub(crate) reason: Option<ExtensionEnvironmentReason>,
}

impl ExtensionEnvironment {
    pub(crate) fn evaluate(host: HostEnvironment) -> Self {
        let platform_supported = host.os == "windows" && host.arch == "x86_64";
        let version_supported = host
            .python
            .as_ref()
            .and_then(|python| parse_python_version(&python.version))
            .is_some_and(|(major, minor)| {
                major > PYTHON_MIN_MAJOR || (major == PYTHON_MIN_MAJOR && minor >= PYTHON_MIN_MINOR)
            });
        let reason = if !platform_supported {
            Some(ExtensionEnvironmentReason::WindowsX64Only)
        } else if host.python.is_none() {
            Some(ExtensionEnvironmentReason::PythonMissing)
        } else if !version_supported {
            Some(ExtensionEnvironmentReason::PythonVersion)
        } else {
            None
        };
        let supported = reason.is_none();
        Self {
            runtime: "tauri",
            os: host.os,
            arch: host.arch,
            supported,
            native_operations_available: supported,
            python: host.python,
            reason,
        }
    }

    pub(crate) fn reason_key(&self) -> Option<&'static str> {
        self.reason.map(ExtensionEnvironmentReason::message_key)
    }
}

fn parse_python_version(value: &str) -> Option<(u32, u32)> {
    let value = value.trim().strip_prefix("Python ").unwrap_or(value.trim());
    let mut parts = value.split('.');
    Some((parts.next()?.parse().ok()?, parts.next()?.parse().ok()?))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn host(os: &str, arch: &str, version: Option<&str>) -> HostEnvironment {
        HostEnvironment {
            os: os.to_string(),
            arch: arch.to_string(),
            python: version.map(|version| PythonRuntime {
                path: "python".to_string(),
                version: version.to_string(),
            }),
        }
    }

    #[test]
    fn support_requires_windows_x64_and_python_310_or_newer() {
        let supported = ExtensionEnvironment::evaluate(host("windows", "x86_64", Some("3.12.4")));
        assert!(supported.supported);
        assert!(supported.native_operations_available);
        assert!(supported.reason.is_none());

        let old_python =
            ExtensionEnvironment::evaluate(host("windows", "x86_64", Some("Python 3.9.18")));
        assert_eq!(
            old_python.reason,
            Some(ExtensionEnvironmentReason::PythonVersion)
        );
        assert_eq!(
            old_python.reason_key(),
            Some("extensions.environment.pythonVersion")
        );
    }

    #[test]
    fn platform_failure_takes_precedence_over_python_failures() {
        let unsupported = ExtensionEnvironment::evaluate(host("linux", "x86_64", None));
        assert_eq!(
            unsupported.reason,
            Some(ExtensionEnvironmentReason::WindowsX64Only)
        );

        let missing = ExtensionEnvironment::evaluate(host("windows", "x86_64", None));
        assert_eq!(
            missing.reason,
            Some(ExtensionEnvironmentReason::PythonMissing)
        );
    }
}
