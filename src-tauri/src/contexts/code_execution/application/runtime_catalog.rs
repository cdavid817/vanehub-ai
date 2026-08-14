#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CodeRuntime {
    Python,
    JavaScript,
}

impl CodeRuntime {
    #[allow(dead_code)]
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::Python => "python",
            Self::JavaScript => "javascript",
        }
    }

    pub(crate) const fn source_extension(self) -> &'static str {
        match self {
            Self::Python => "py",
            Self::JavaScript => "mjs",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct RuntimeVersion {
    pub(crate) major: u16,
    pub(crate) minor: u16,
    pub(crate) patch: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ReviewedRuntime {
    pub(crate) runtime: CodeRuntime,
    pub(crate) executable_names: &'static [&'static str],
    pub(crate) version_argument: &'static str,
    pub(crate) minimum: RuntimeVersion,
    pub(crate) maximum_exclusive: RuntimeVersion,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RuntimeCatalogError {
    #[allow(dead_code)]
    UnsupportedRuntime,
    MalformedVersion,
    VersionNotReviewed,
}

pub(crate) struct RuntimeCatalog;

impl RuntimeCatalog {
    pub(crate) const fn reviewed(runtime: CodeRuntime) -> ReviewedRuntime {
        match runtime {
            CodeRuntime::Python => ReviewedRuntime {
                runtime,
                executable_names: &["python3", "python"],
                version_argument: "--version",
                minimum: RuntimeVersion::new(3, 11, 0),
                maximum_exclusive: RuntimeVersion::new(3, 15, 0),
            },
            CodeRuntime::JavaScript => ReviewedRuntime {
                runtime,
                executable_names: &["node"],
                version_argument: "--version",
                minimum: RuntimeVersion::new(20, 0, 0),
                maximum_exclusive: RuntimeVersion::new(25, 0, 0),
            },
        }
    }

    pub(crate) fn parse_version(
        runtime: CodeRuntime,
        output: &str,
    ) -> Result<RuntimeVersion, RuntimeCatalogError> {
        let prefix = match runtime {
            CodeRuntime::Python => "Python ",
            CodeRuntime::JavaScript => "v",
        };
        let version = output
            .trim()
            .strip_prefix(prefix)
            .ok_or(RuntimeCatalogError::MalformedVersion)?;
        let mut parts = version.split('.');
        let parsed = RuntimeVersion {
            major: parse_part(parts.next())?,
            minor: parse_part(parts.next())?,
            patch: parse_part(parts.next())?,
        };
        if parts.next().is_some() {
            return Err(RuntimeCatalogError::MalformedVersion);
        }
        let reviewed = Self::reviewed(runtime);
        if parsed < reviewed.minimum || parsed >= reviewed.maximum_exclusive {
            return Err(RuntimeCatalogError::VersionNotReviewed);
        }
        Ok(parsed)
    }
}

impl RuntimeVersion {
    pub(crate) const fn new(major: u16, minor: u16, patch: u16) -> Self {
        Self {
            major,
            minor,
            patch,
        }
    }
}

fn parse_part(value: Option<&str>) -> Result<u16, RuntimeCatalogError> {
    value
        .filter(|part| !part.is_empty() && part.bytes().all(|byte| byte.is_ascii_digit()))
        .ok_or(RuntimeCatalogError::MalformedVersion)?
        .parse()
        .map_err(|_| RuntimeCatalogError::MalformedVersion)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_contains_only_reviewed_python_and_javascript_commands() {
        let python = RuntimeCatalog::reviewed(CodeRuntime::Python);
        assert_eq!(python.executable_names, &["python3", "python"]);
        assert_eq!(python.version_argument, "--version");
        assert_eq!(CodeRuntime::Python.source_extension(), "py");
        let javascript = RuntimeCatalog::reviewed(CodeRuntime::JavaScript);
        assert_eq!(javascript.executable_names, &["node"]);
        assert_eq!(CodeRuntime::JavaScript.source_extension(), "mjs");
    }

    #[test]
    fn versions_are_strictly_parsed_and_checked_against_reviewed_ranges() {
        assert_eq!(
            RuntimeCatalog::parse_version(CodeRuntime::Python, "Python 3.12.4\n"),
            Ok(RuntimeVersion::new(3, 12, 4))
        );
        assert_eq!(
            RuntimeCatalog::parse_version(CodeRuntime::JavaScript, "v22.14.0"),
            Ok(RuntimeVersion::new(22, 14, 0))
        );
        assert_eq!(
            RuntimeCatalog::parse_version(CodeRuntime::Python, "Python 3.10.9"),
            Err(RuntimeCatalogError::VersionNotReviewed)
        );
        assert_eq!(
            RuntimeCatalog::parse_version(CodeRuntime::JavaScript, "node v22.1.0"),
            Err(RuntimeCatalogError::MalformedVersion)
        );
    }
}
