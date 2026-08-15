use super::{
    DiscoveredSkillTool, EffectiveSkillToolPackage, RejectedSkillTool, SkillToolApplicationError,
    SkillToolDiscoveryOutcome, SkillToolPackageRef, SkillToolPackageSource,
};
use crate::contexts::tooling::skill_tools::domain::{
    content_hash_of, inspect_module, integrity_for, parse_manifest_bytes, revision_witness,
    ContentHash, SkillToolDeclaration, SkillToolDiagnostic, SkillToolDiagnosticSeverity,
    SkillToolImplementation, SkillToolKey, SkillToolManifest, SkillToolManifestLimits,
    MANIFEST_PATH, MODULE_DIRECTORY,
};

/// Reads Skill tool content out of the winning effective revision only.
///
/// Nothing here consults trust or enablement: discovery answers "what does this revision declare,
/// and does its content hold together", and governance answers whether any of it may run.
pub(crate) struct SkillToolDiscoveryService<'a> {
    source: &'a dyn SkillToolPackageSource,
    limits: SkillToolManifestLimits,
}

impl<'a> SkillToolDiscoveryService<'a> {
    pub(crate) fn new(
        source: &'a dyn SkillToolPackageSource,
        limits: SkillToolManifestLimits,
    ) -> Self {
        Self { source, limits }
    }

    /// Discovers the effective revision's tools.
    ///
    /// Shadowed revisions are recorded as ignored and never read: a lower-priority package that
    /// happens to ship a manifest must not contribute a tool, an integrity witness, or a hash the
    /// winning revision would then appear to have blessed.
    pub(crate) fn discover(
        &self,
        package: &EffectiveSkillToolPackage,
    ) -> Result<SkillToolDiscoveryOutcome, SkillToolApplicationError> {
        let mut outcome = SkillToolDiscoveryOutcome {
            ignored_shadowed_revisions: package
                .shadowed
                .iter()
                .map(|shadowed| shadowed.base_revision.clone())
                .collect(),
            ..SkillToolDiscoveryOutcome::default()
        };

        let effective = &package.effective;
        let Some(manifest_bytes) = self.source.read_manifest(effective)? else {
            return Ok(outcome);
        };
        outcome.manifest_present = true;

        let manifest_hash = content_hash_of(&manifest_bytes);
        let manifest = match parse_manifest_bytes(&manifest_bytes, &self.limits) {
            Ok(manifest) => manifest,
            Err(error) => {
                outcome.rejected.push(manifest_rejection(error.code()));
                return Ok(outcome);
            }
        };
        if manifest.owner != effective.owner {
            outcome.rejected.push(manifest_rejection(
                SkillToolApplicationError::OwnerMismatch {
                    declared: manifest.owner.as_str().to_string(),
                    package: effective.owner.as_str().to_string(),
                }
                .code(),
            ));
            return Ok(outcome);
        }

        outcome.undeclared_files = self.undeclared_files(effective, &manifest)?;
        for declaration in &manifest.tools {
            match self.resolve_tool(effective, declaration, &manifest_hash) {
                Ok(tool) => outcome.discovered.push(tool),
                Err(error) if error.is_content_failure() => {
                    outcome.rejected.push(RejectedSkillTool {
                        tool_id: Some(declaration.id.as_str().to_string()),
                        diagnostic: SkillToolDiagnostic::new(
                            SkillToolDiagnosticSeverity::Error,
                            error.code(),
                            declaration.id.as_str(),
                        ),
                    });
                }
                Err(error) => return Err(error),
            }
        }
        Ok(outcome)
    }

    /// Builds the immutable witness for one declared tool, verifying whatever bytes it binds.
    fn resolve_tool(
        &self,
        package: &SkillToolPackageRef,
        declaration: &SkillToolDeclaration,
        manifest_hash: &ContentHash,
    ) -> Result<DiscoveredSkillTool, SkillToolApplicationError> {
        let requires_module_runtime = matches!(
            declaration.implementation,
            SkillToolImplementation::Module(_)
        );
        if let SkillToolImplementation::Module(module) = &declaration.implementation {
            let bytes = self.source.read_implementation(package, &module.path)?;
            if bytes.len() as u64 > self.limits.maximum_module_bytes {
                return Err(SkillToolApplicationError::OversizedFile {
                    path: module.path.clone(),
                    maximum: self.limits.maximum_module_bytes,
                    actual: bytes.len() as u64,
                });
            }
            if content_hash_of(&bytes) != module.content_hash {
                return Err(SkillToolApplicationError::IntegrityMismatch {
                    path: module.path.clone(),
                });
            }
            inspect_module(&bytes, &module.export, &declaration.limits)?;
        }

        let integrity = integrity_for(declaration, &package.base_revision, manifest_hash);
        let revision =
            revision_witness(&package.owner, &package.source, &declaration.id, &integrity)?;
        let key = SkillToolKey::new(
            package.owner.clone(),
            package.source.clone(),
            declaration.id.clone(),
            revision,
        );
        Ok(DiscoveredSkillTool {
            canonical_name: key.canonical_name()?,
            key,
            declaration: declaration.clone(),
            integrity,
            requires_module_runtime,
        })
    }

    /// Files under the reserved tool directory that no manifest entry integrity-binds.
    ///
    /// Reported rather than loaded. Enforcing the aggregate size budget here, over everything
    /// present, is deliberate: a package that hides 200 MiB under `scripts/modules/` is a resource
    /// problem whether or not the manifest admits to the files.
    fn undeclared_files(
        &self,
        package: &SkillToolPackageRef,
        manifest: &SkillToolManifest,
    ) -> Result<Vec<String>, SkillToolApplicationError> {
        let declared = manifest.declared_implementation_paths();
        let mut aggregate = 0_u64;
        let mut undeclared = Vec::new();
        for entry in self.source.list_tool_files(package)? {
            aggregate = aggregate.saturating_add(entry.size_bytes);
            if aggregate > self.limits.maximum_aggregate_module_bytes {
                return Err(SkillToolApplicationError::OversizedPackage {
                    maximum: self.limits.maximum_aggregate_module_bytes,
                    actual: aggregate,
                });
            }
            let is_declared =
                entry.relative_path == MANIFEST_PATH || declared.contains(&entry.relative_path);
            if !is_declared && entry.relative_path.starts_with(MODULE_DIRECTORY) {
                undeclared.push(entry.relative_path);
            }
        }
        undeclared.sort();
        Ok(undeclared)
    }
}

/// A manifest-level failure rejects every tool at once, so it carries no tool id.
fn manifest_rejection(code: &str) -> RejectedSkillTool {
    RejectedSkillTool {
        tool_id: None,
        diagnostic: SkillToolDiagnostic::new(
            SkillToolDiagnosticSeverity::Error,
            code,
            MANIFEST_PATH,
        ),
    }
}
