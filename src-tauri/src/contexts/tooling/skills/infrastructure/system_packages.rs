use super::filesystem::parse_document;
use crate::contexts::tooling::skills::application::{
    validate_relative_resource_path, SkillApplicationError, SkillDocument, SkillLayerProvider,
    SkillPackageDescriptor, SkillPackageReader, SkillPackageResource, SkillResourceDocument,
    MAX_RESOURCE_BYTES, MAX_RESOURCE_CHARACTERS,
};
use crate::contexts::tooling::skills::domain::{
    SkillAvailability, SkillDelivery, SkillId, SkillLayer, SkillOrigin, SkillTrust, SkillType,
};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::path::Path;

const SYSTEM_MANIFEST: &str =
    include_str!("../../../../../resources/skills/system/v1/manifest.json");

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SystemManifest {
    schema_version: u32,
    package_set_version: String,
    packages: Vec<SystemManifestPackage>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SystemManifestPackage {
    id: String,
    version: String,
    document_sha256: String,
    aliases: Vec<String>,
    #[serde(rename = "type")]
    skill_type: String,
    delivery: String,
    resources: Vec<SystemManifestResource>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SystemManifestResource {
    path: String,
    sha256: String,
}

#[derive(Clone, Default)]
pub(crate) struct SystemSkillPackages;

impl SystemSkillPackages {
    fn manifest(&self) -> Result<SystemManifest, SkillApplicationError> {
        let manifest: SystemManifest = serde_json::from_str(SYSTEM_MANIFEST)
            .map_err(|error| invalid_manifest(error.to_string()))?;
        validate_manifest(&manifest)?;
        Ok(manifest)
    }
}

impl SkillLayerProvider for SystemSkillPackages {
    fn inventory(
        &self,
        _workspace_path: Option<&str>,
    ) -> Result<Vec<SkillPackageDescriptor>, SkillApplicationError> {
        let manifest = self.manifest()?;
        let package_set_version = manifest.package_set_version.clone();
        manifest
            .packages
            .into_iter()
            .map(|package| descriptor(&package_set_version, &package))
            .collect()
    }
}

impl SkillPackageReader for SystemSkillPackages {
    fn read_document(
        &self,
        package: &SkillPackageDescriptor,
    ) -> Result<SkillDocument, SkillApplicationError> {
        if package.layer != SkillLayer::System {
            return Err(SkillApplicationError::Validation(
                "System package reader only accepts System definitions".to_string(),
            ));
        }
        let manifest = self.manifest()?;
        let entry = manifest
            .packages
            .iter()
            .find(|entry| entry.id == package.metadata.id.as_str())
            .ok_or_else(|| {
                SkillApplicationError::NotFound(package.metadata.id.as_str().to_string())
            })?;
        if package.package_key != package_key(&manifest.package_set_version, &entry.id)
            || package.revision != entry.document_sha256
        {
            return Err(SkillApplicationError::ConcurrentModification(
                entry.id.clone(),
            ));
        }
        let content = embedded_document(&entry.id)
            .ok_or_else(|| invalid_manifest(format!("missing document for {}", entry.id)))?;
        verify_hash(content.as_bytes(), &entry.document_sha256)?;
        parse_document(content)
    }

    fn list_resources(
        &self,
        package: &SkillPackageDescriptor,
    ) -> Result<Vec<SkillPackageResource>, SkillApplicationError> {
        let manifest = self.manifest()?;
        let entry = manifest
            .packages
            .iter()
            .find(|entry| entry.id == package.metadata.id.as_str())
            .ok_or_else(|| SkillApplicationError::NotFound(package.metadata.id.as_str().into()))?;
        if package.package_key != package_key(&manifest.package_set_version, &entry.id)
            || package.revision != entry.document_sha256
        {
            return Err(SkillApplicationError::ConcurrentModification(
                entry.id.clone(),
            ));
        }
        Ok(entry
            .resources
            .iter()
            .map(|resource| SkillPackageResource {
                relative_path: resource.path.clone(),
                media_type: resource_media_type(&resource.path).to_string(),
                size_bytes: embedded_resource(&entry.id, &resource.path)
                    .map_or(0, |content| content.len() as u64),
                content_hash: resource.sha256.clone(),
            })
            .collect())
    }

    fn read_resource(
        &self,
        package: &SkillPackageDescriptor,
        relative_path: &str,
    ) -> Result<SkillResourceDocument, SkillApplicationError> {
        let manifest = self.manifest()?;
        let entry = manifest
            .packages
            .iter()
            .find(|entry| entry.id == package.metadata.id.as_str())
            .ok_or_else(|| SkillApplicationError::NotFound(package.metadata.id.as_str().into()))?;
        if package.package_key != package_key(&manifest.package_set_version, &entry.id)
            || package.revision != entry.document_sha256
        {
            return Err(SkillApplicationError::ConcurrentModification(
                entry.id.clone(),
            ));
        }
        let resource = entry
            .resources
            .iter()
            .find(|resource| resource.path == relative_path)
            .ok_or_else(|| SkillApplicationError::NotFound(relative_path.to_string()))?;
        let content = embedded_resource(&entry.id, relative_path)
            .ok_or_else(|| invalid_manifest("missing embedded resource"))?;
        verify_hash(content.as_bytes(), &resource.sha256)?;
        if content.len() as u64 > MAX_RESOURCE_BYTES
            || content.chars().count() > MAX_RESOURCE_CHARACTERS
        {
            return Err(SkillApplicationError::OversizedResource);
        }
        Ok(SkillResourceDocument {
            content: content.to_string(),
            size_bytes: content.len() as u64,
        })
    }

    fn read_resource_bytes(
        &self,
        package: &SkillPackageDescriptor,
        relative_path: &str,
    ) -> Result<Vec<u8>, SkillApplicationError> {
        self.read_resource(package, relative_path)
            .map(|resource| resource.content.into_bytes())
    }
}

fn resource_media_type(path: &str) -> &'static str {
    match Path::new(path)
        .extension()
        .and_then(|extension| extension.to_str())
        .map(str::to_ascii_lowercase)
        .as_deref()
    {
        Some("md" | "markdown") => "text/markdown",
        Some("json") => "application/json",
        Some("yaml" | "yml") => "application/yaml",
        Some("txt") => "text/plain",
        Some("png") => "image/png",
        Some("jpg" | "jpeg") => "image/jpeg",
        Some("gif") => "image/gif",
        Some("webp") => "image/webp",
        Some("svg") => "image/svg+xml",
        _ => "application/octet-stream",
    }
}

fn descriptor(
    package_set_version: &str,
    package: &SystemManifestPackage,
) -> Result<SkillPackageDescriptor, SkillApplicationError> {
    let content = embedded_document(&package.id)
        .ok_or_else(|| invalid_manifest(format!("missing document for {}", package.id)))?;
    verify_hash(content.as_bytes(), &package.document_sha256)?;
    let metadata = parse_document(content)?.metadata;
    let aliases = package
        .aliases
        .iter()
        .map(SkillId::parse)
        .collect::<Result<Vec<_>, _>>()?;
    if metadata.id.as_str() != package.id
        || metadata.version != package.version
        || metadata.aliases != aliases
        || metadata.skill_type != SkillType::parse(&package.skill_type)?
        || metadata.delivery != SkillDelivery::parse(&package.delivery)?
    {
        return Err(invalid_manifest(format!(
            "classification mismatch for {}",
            package.id
        )));
    }
    Ok(SkillPackageDescriptor {
        package_key: package_key(package_set_version, &package.id),
        workspace_path: None,
        metadata,
        layer: SkillLayer::System,
        origin: SkillOrigin::Shipped,
        trust: SkillTrust::Trusted,
        availability: SkillAvailability::Available,
        revision: package.document_sha256.clone(),
        source_path: None,
    })
}

fn validate_manifest(manifest: &SystemManifest) -> Result<(), SkillApplicationError> {
    if manifest.schema_version != 1 || manifest.package_set_version != "1" {
        return Err(invalid_manifest("unsupported manifest version"));
    }
    let mut previous_id = None;
    let mut ids = BTreeSet::new();
    for package in &manifest.packages {
        SkillId::parse(&package.id)?;
        if !ids.insert(package.id.as_str())
            || previous_id.is_some_and(|previous| previous >= package.id.as_str())
        {
            return Err(invalid_manifest("package ids must be unique and sorted"));
        }
        previous_id = Some(package.id.as_str());
        validate_sha256(&package.document_sha256)?;
        let mut previous_resource = None;
        for resource in &package.resources {
            validate_resource_path(&resource.path)?;
            validate_sha256(&resource.sha256)?;
            if previous_resource.is_some_and(|previous| previous >= resource.path.as_str()) {
                return Err(invalid_manifest("resource paths must be unique and sorted"));
            }
            previous_resource = Some(resource.path.as_str());
        }
    }
    Ok(())
}

fn validate_resource_path(path: &str) -> Result<(), SkillApplicationError> {
    let allowed = ["scripts/", "references/", "templates/", "assets/"];
    if path.is_empty()
        || path.starts_with('.')
        || path.contains("..")
        || path.contains('\\')
        || !allowed.iter().any(|prefix| path.starts_with(prefix))
    {
        return Err(invalid_manifest("invalid resource path"));
    }
    validate_relative_resource_path(path).map_err(|_| invalid_manifest("invalid resource path"))
}

fn validate_sha256(hash: &str) -> Result<(), SkillApplicationError> {
    if hash.len() == 64 && hash.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        Ok(())
    } else {
        Err(invalid_manifest("invalid SHA-256 hash"))
    }
}

fn verify_hash(content: &[u8], expected: &str) -> Result<(), SkillApplicationError> {
    let actual = Sha256::digest(content)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    if actual == expected {
        Ok(())
    } else {
        Err(invalid_manifest("System package hash mismatch"))
    }
}

fn package_key(package_set_version: &str, id: &str) -> String {
    format!("system:{package_set_version}:{id}")
}

fn invalid_manifest(message: impl Into<String>) -> SkillApplicationError {
    SkillApplicationError::Validation(format!("Invalid System Skill manifest: {}", message.into()))
}

fn embedded_document(id: &str) -> Option<&'static str> {
    match id {
        "api-doc-generation" => Some(include_str!(
            "../../../../../resources/skills/system/v1/api-doc-generation/SKILL.md"
        )),
        "code-review" => Some(include_str!(
            "../../../../../resources/skills/system/v1/code-review/SKILL.md"
        )),
        "code-security-scan" => Some(include_str!(
            "../../../../../resources/skills/system/v1/code-security-scan/SKILL.md"
        )),
        "readme-generation" => Some(include_str!(
            "../../../../../resources/skills/system/v1/readme-generation/SKILL.md"
        )),
        "tdd-discipline" => Some(include_str!(
            "../../../../../resources/skills/system/v1/tdd-discipline/SKILL.md"
        )),
        "unit-test-generation" => Some(include_str!(
            "../../../../../resources/skills/system/v1/unit-test-generation/SKILL.md"
        )),
        _ => None,
    }
}

fn embedded_resource(_id: &str, _path: &str) -> Option<&'static str> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contexts::tooling::skills::domain::{builtin_definition, SkillLayer};

    #[test]
    fn manifest_is_complete_hash_verified_and_deterministic() {
        let packages = SystemSkillPackages.inventory(None).expect("inventory");
        assert_eq!(packages.len(), 6);
        let ids = packages
            .iter()
            .map(|package| package.metadata.id.as_str())
            .collect::<Vec<_>>();
        assert_eq!(
            ids,
            vec![
                "api-doc-generation",
                "code-review",
                "code-security-scan",
                "readme-generation",
                "tdd-discipline",
                "unit-test-generation"
            ]
        );
        for package in packages {
            assert_eq!(package.layer, SkillLayer::System);
            assert_eq!(package.origin, SkillOrigin::Shipped);
            assert_eq!(package.trust, SkillTrust::Trusted);
            assert_eq!(package.availability, SkillAvailability::Available);
            assert!(package.source_path.is_none());
            let document = SystemSkillPackages
                .read_document(&package)
                .expect("document");
            let expected = builtin_definition(&package.metadata.id).expect("builtin");
            assert_eq!(document.body, expected.body);
        }
    }

    #[test]
    fn manifest_rejects_hash_classification_and_resource_order_changes() {
        let mut manifest: SystemManifest = serde_json::from_str(SYSTEM_MANIFEST).expect("manifest");
        manifest.packages[0].document_sha256 = "bad".to_string();
        assert!(validate_manifest(&manifest).is_err());

        let mut manifest: SystemManifest = serde_json::from_str(SYSTEM_MANIFEST).expect("manifest");
        manifest.packages[0].resources = vec![
            SystemManifestResource {
                path: "references/z.md".to_string(),
                sha256: "0".repeat(64),
            },
            SystemManifestResource {
                path: "references/a.md".to_string(),
                sha256: "1".repeat(64),
            },
        ];
        assert!(validate_manifest(&manifest).is_err());

        let mut manifest: SystemManifest = serde_json::from_str(SYSTEM_MANIFEST).expect("manifest");
        manifest.packages[0].skill_type = "utility".to_string();
        assert!(descriptor(&manifest.package_set_version, &manifest.packages[0]).is_err());
    }
}
