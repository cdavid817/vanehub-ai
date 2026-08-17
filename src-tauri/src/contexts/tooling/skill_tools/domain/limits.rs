use super::SkillToolDomainError;

/// Where a Skill's tool manifest and its integrity-bound implementation files must live.
///
/// Reserved base-package paths: Overlay mutation, learning blocks, and evolution auto-apply are
/// refused against these prefixes so executable content can only change by shipping a new package
/// revision (`design.md` decision 6).
pub(crate) const MANIFEST_PATH: &str = "scripts/tools.json";
pub(crate) const MODULE_DIRECTORY: &str = "scripts/modules/";
pub(crate) const RESERVED_EXECUTABLE_PREFIXES: &[&str] = &[MANIFEST_PATH, MODULE_DIRECTORY];

/// Only manifest version 1 exists. An unknown version marks its tools unavailable instead of
/// being best-effort parsed, because a newer manifest's safety fields would be silently dropped.
pub(crate) const SUPPORTED_MANIFEST_VERSION: u32 = 1;

/// Static contract bounds. These cap what a manifest may *declare*; runtime ceilings live in
/// `SkillToolLimits`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SkillToolManifestLimits {
    pub(crate) maximum_manifest_bytes: u64,
    pub(crate) maximum_tools: usize,
    pub(crate) maximum_description_characters: usize,
    pub(crate) maximum_capabilities: usize,
    pub(crate) maximum_template_fields: usize,
    pub(crate) maximum_module_bytes: u64,
    pub(crate) maximum_aggregate_module_bytes: u64,
    pub(crate) maximum_schema_depth: usize,
    pub(crate) maximum_schema_nodes: usize,
    pub(crate) maximum_schema_properties: usize,
    pub(crate) maximum_schema_enum_values: usize,
    pub(crate) maximum_schema_bytes: usize,
}

pub(crate) const DEFAULT_MANIFEST_LIMITS: SkillToolManifestLimits = SkillToolManifestLimits {
    maximum_manifest_bytes: 64 * 1024,
    maximum_tools: 16,
    maximum_description_characters: 512,
    maximum_capabilities: 8,
    maximum_template_fields: 32,
    maximum_module_bytes: 4 * 1024 * 1024,
    maximum_aggregate_module_bytes: 16 * 1024 * 1024,
    maximum_schema_depth: 6,
    maximum_schema_nodes: 128,
    maximum_schema_properties: 32,
    maximum_schema_enum_values: 32,
    maximum_schema_bytes: 16 * 1024,
};

/// Runtime ceilings for one tool invocation.
///
/// A manifest may only *tighten* these. Loosening is rejected rather than clamped so an operator
/// reading a manifest sees the value that will actually apply.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SkillToolLimits {
    pub(crate) wall_time_milliseconds: u64,
    pub(crate) fuel: u64,
    pub(crate) memory_bytes: u64,
    pub(crate) input_bytes: u64,
    pub(crate) output_bytes: u64,
    pub(crate) host_calls: u32,
    pub(crate) delegation_depth: u32,
    pub(crate) concurrency: u32,
    pub(crate) child_processes: u32,
    pub(crate) file_bytes: u64,
    pub(crate) network_bytes: u64,
}

/// `design.md` decision 2. Fuel is a placeholder budget until the engine adapter calibrates it in
/// section 4; every other value is the documented ceiling.
pub(crate) const DEFAULT_SKILL_TOOL_LIMITS: SkillToolLimits = SkillToolLimits {
    wall_time_milliseconds: 10_000,
    fuel: 200_000_000,
    memory_bytes: 64 * 1024 * 1024,
    input_bytes: 1024 * 1024,
    output_bytes: 1024 * 1024,
    host_calls: 8,
    delegation_depth: 4,
    concurrency: 2,
    child_processes: 4,
    file_bytes: 8 * 1024 * 1024,
    network_bytes: 16 * 1024 * 1024,
};

/// A manifest's optional overrides. Absent fields keep the ceiling.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct SkillToolLimitOverrides {
    pub(crate) wall_time_milliseconds: Option<u64>,
    pub(crate) fuel: Option<u64>,
    pub(crate) memory_bytes: Option<u64>,
    pub(crate) input_bytes: Option<u64>,
    pub(crate) output_bytes: Option<u64>,
    pub(crate) host_calls: Option<u32>,
    pub(crate) delegation_depth: Option<u32>,
    pub(crate) concurrency: Option<u32>,
    pub(crate) child_processes: Option<u32>,
    pub(crate) file_bytes: Option<u64>,
    pub(crate) network_bytes: Option<u64>,
}

impl SkillToolLimits {
    pub(crate) fn tighten(
        self,
        overrides: SkillToolLimitOverrides,
    ) -> Result<Self, SkillToolDomainError> {
        Ok(Self {
            wall_time_milliseconds: tighten_u64(
                "wallTimeMs",
                self.wall_time_milliseconds,
                overrides.wall_time_milliseconds,
            )?,
            fuel: tighten_u64("fuel", self.fuel, overrides.fuel)?,
            memory_bytes: tighten_u64("memoryBytes", self.memory_bytes, overrides.memory_bytes)?,
            input_bytes: tighten_u64("inputBytes", self.input_bytes, overrides.input_bytes)?,
            output_bytes: tighten_u64("outputBytes", self.output_bytes, overrides.output_bytes)?,
            host_calls: tighten_u32("hostCalls", self.host_calls, overrides.host_calls)?,
            delegation_depth: tighten_u32(
                "delegationDepth",
                self.delegation_depth,
                overrides.delegation_depth,
            )?,
            concurrency: tighten_u32("concurrency", self.concurrency, overrides.concurrency)?,
            child_processes: tighten_u32(
                "childProcesses",
                self.child_processes,
                overrides.child_processes,
            )?,
            file_bytes: tighten_u64("fileBytes", self.file_bytes, overrides.file_bytes)?,
            network_bytes: tighten_u64(
                "networkBytes",
                self.network_bytes,
                overrides.network_bytes,
            )?,
        })
    }
}

/// Zero is refused alongside anything above the ceiling: a zero budget is not a tighter limit,
/// it is a tool that can never run, and registering it would look like a runtime failure later.
fn tighten_u64(
    limit: &'static str,
    ceiling: u64,
    requested: Option<u64>,
) -> Result<u64, SkillToolDomainError> {
    match requested {
        None => Ok(ceiling),
        Some(value) if value > 0 && value <= ceiling => Ok(value),
        Some(_) => Err(SkillToolDomainError::LimitAboveCeiling { limit }),
    }
}

fn tighten_u32(
    limit: &'static str,
    ceiling: u32,
    requested: Option<u32>,
) -> Result<u32, SkillToolDomainError> {
    match requested {
        None => Ok(ceiling),
        Some(value) if value > 0 && value <= ceiling => Ok(value),
        Some(_) => Err(SkillToolDomainError::LimitAboveCeiling { limit }),
    }
}

/// True when `path` is a reserved executable path that only a new package revision may change.
pub(crate) fn is_reserved_executable_path(path: &str) -> bool {
    let normalized = path.trim_start_matches("./").replace('\\', "/");
    let normalized = normalized.trim_start_matches('/');
    RESERVED_EXECUTABLE_PREFIXES.iter().any(|reserved| {
        if let Some(directory) = reserved.strip_suffix('/') {
            normalized == directory || normalized.starts_with(reserved)
        } else {
            normalized == *reserved
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manifest_limits_may_only_tighten_the_central_ceilings() {
        let tightened = DEFAULT_SKILL_TOOL_LIMITS
            .tighten(SkillToolLimitOverrides {
                wall_time_milliseconds: Some(2_000),
                host_calls: Some(1),
                ..SkillToolLimitOverrides::default()
            })
            .expect("tightened limits");
        assert_eq!(tightened.wall_time_milliseconds, 2_000);
        assert_eq!(tightened.host_calls, 1);
        assert_eq!(
            tightened.memory_bytes,
            DEFAULT_SKILL_TOOL_LIMITS.memory_bytes
        );

        for overrides in [
            SkillToolLimitOverrides {
                wall_time_milliseconds: Some(10_001),
                ..SkillToolLimitOverrides::default()
            },
            SkillToolLimitOverrides {
                memory_bytes: Some(u64::MAX),
                ..SkillToolLimitOverrides::default()
            },
            SkillToolLimitOverrides {
                delegation_depth: Some(5),
                ..SkillToolLimitOverrides::default()
            },
            SkillToolLimitOverrides {
                host_calls: Some(0),
                ..SkillToolLimitOverrides::default()
            },
        ] {
            assert!(matches!(
                DEFAULT_SKILL_TOOL_LIMITS.tighten(overrides),
                Err(SkillToolDomainError::LimitAboveCeiling { .. })
            ));
        }
    }

    #[test]
    fn absent_overrides_keep_every_documented_ceiling() {
        let limits = DEFAULT_SKILL_TOOL_LIMITS
            .tighten(SkillToolLimitOverrides::default())
            .expect("defaults");
        assert_eq!(limits, DEFAULT_SKILL_TOOL_LIMITS);
        assert_eq!(limits.memory_bytes, 64 * 1024 * 1024);
        assert_eq!(limits.input_bytes, 1024 * 1024);
        assert_eq!(limits.output_bytes, 1024 * 1024);
        assert_eq!(limits.host_calls, 8);
        assert_eq!(limits.delegation_depth, 4);
    }

    #[test]
    fn reserved_executable_paths_cover_the_manifest_and_every_module_form() {
        for reserved in [
            "scripts/tools.json",
            "./scripts/tools.json",
            "scripts\\tools.json",
            "scripts/modules",
            "scripts/modules/score.wasm",
            "scripts/modules/nested/score.wasm",
        ] {
            assert!(
                is_reserved_executable_path(reserved),
                "{reserved} must be reserved"
            );
        }
        for allowed in [
            "references/guide.md",
            "scripts/notes.md",
            "scripts/tools.json.bak",
            "templates/report.md",
            "assets/diagram.png",
        ] {
            assert!(
                !is_reserved_executable_path(allowed),
                "{allowed} must stay editable"
            );
        }
    }

    /// Guards the one restated constant across the `skills` / `skill_tools` subdomain boundary.
    ///
    /// `skills::domain::overlay_path` cannot import this module — a domain module may not depend on
    /// another subdomain's domain model — so it carries its own copy of the reserved prefix. This
    /// asserts from the owning side that the copy still refuses everything this list reserves, so
    /// the two cannot drift apart silently.
    #[test]
    fn overlay_validation_refuses_every_path_this_subdomain_reserves() {
        use crate::contexts::tooling::skills::domain::{validate_overlay_path, OverlayPathError};

        for reserved in RESERVED_EXECUTABLE_PREFIXES {
            let directory = reserved.trim_end_matches('/');
            for path in [directory.to_string(), format!("{directory}/probe.bin")] {
                assert_eq!(
                    validate_overlay_path(&path),
                    Err(OverlayPathError::ReservedExecutablePath),
                    "{path} must be unreachable through Overlay"
                );
            }
        }
        assert!(is_reserved_executable_path(MANIFEST_PATH));
        assert!(is_reserved_executable_path(&format!(
            "{MODULE_DIRECTORY}probe.wasm"
        )));
    }
}
