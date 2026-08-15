//! Static inspection of a core WebAssembly module's import, export, and memory declarations.
//!
//! Deliberately a byte-level reader rather than a call into an engine: this must run with the
//! module runtime feature disabled, on a machine that will never execute the module, so that
//! "this Skill wants unrestricted WASI" is answerable during read-only discovery.
//!
//! It parses only the section framing it needs and skips every other section by length. It is not
//! a validator — the engine still validates before instantiation in section 4.

use super::SkillToolLimits;

const WASM_MAGIC: [u8; 4] = [0x00, 0x61, 0x73, 0x6d];
const WASM_VERSION: [u8; 4] = [0x01, 0x00, 0x00, 0x00];
const IMPORT_SECTION: u8 = 2;
const MEMORY_SECTION: u8 = 5;
const EXPORT_SECTION: u8 = 7;
const WASM_PAGE_BYTES: u64 = 64 * 1024;

/// The only import module a Skill tool may name. Everything a module can ask the host for goes
/// through one structured host call, so an import from `wasi_snapshot_preview1` or any other
/// module is refused by name rather than by trying to reason about which of its functions are safe.
pub(crate) const HOST_IMPORT_MODULE: &str = "vanehub";

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ModuleInspectionError {
    NotWebAssembly,
    UnsupportedVersion,
    Truncated,
    ForbiddenImport {
        module: String,
        name: String,
    },
    MissingExport(String),
    ExportIsNotAFunction(String),
    MemoryAboveCeiling {
        maximum_bytes: u64,
        requested_bytes: u64,
    },
    TooManyDeclarations,
}

impl ModuleInspectionError {
    pub(crate) fn code(&self) -> &'static str {
        match self {
            Self::NotWebAssembly => "module-not-webassembly",
            Self::UnsupportedVersion => "module-unsupported-version",
            Self::Truncated => "module-truncated",
            Self::ForbiddenImport { .. } => "module-forbidden-import",
            Self::MissingExport(_) => "module-missing-export",
            Self::ExportIsNotAFunction(_) => "module-export-not-a-function",
            Self::MemoryAboveCeiling { .. } => "module-memory-above-ceiling",
            Self::TooManyDeclarations => "module-too-many-declarations",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct ModuleInspection {
    pub(crate) imports: Vec<String>,
    pub(crate) exports: Vec<String>,
    pub(crate) declared_memory_bytes: u64,
}

/// Verifies a module declares no import outside the host allowlist, exports the entry point the
/// manifest names as a function, and does not request more linear memory than the limit allows.
pub(crate) fn inspect_module(
    bytes: &[u8],
    required_export: &str,
    limits: &SkillToolLimits,
) -> Result<ModuleInspection, ModuleInspectionError> {
    const MAX_DECLARATIONS: usize = 1024;

    if bytes.len() < 8 || bytes[..4] != WASM_MAGIC {
        return Err(ModuleInspectionError::NotWebAssembly);
    }
    if bytes[4..8] != WASM_VERSION {
        return Err(ModuleInspectionError::UnsupportedVersion);
    }

    let mut inspection = ModuleInspection::default();
    let mut cursor = Cursor::new(&bytes[8..]);
    while !cursor.is_empty() {
        let section_id = cursor.byte()?;
        let length =
            usize::try_from(cursor.leb128()?).map_err(|_| ModuleInspectionError::Truncated)?;
        let body = cursor.take(length)?;
        match section_id {
            IMPORT_SECTION => {
                let mut section = Cursor::new(body);
                let count = section.leb128()?;
                if count as usize > MAX_DECLARATIONS {
                    return Err(ModuleInspectionError::TooManyDeclarations);
                }
                for _ in 0..count {
                    let module = section.name()?;
                    let name = section.name()?;
                    // Descriptor kind plus its index/type payload; skipped because the allowlist
                    // decision is made on the name, not on what the import claims to be.
                    let _kind = section.byte()?;
                    section.leb128()?;
                    if module != HOST_IMPORT_MODULE {
                        return Err(ModuleInspectionError::ForbiddenImport { module, name });
                    }
                    inspection.imports.push(format!("{module}::{name}"));
                }
            }
            MEMORY_SECTION => {
                let mut section = Cursor::new(body);
                let count = section.leb128()?;
                for _ in 0..count {
                    let has_maximum = section.byte()? != 0;
                    let initial = section.leb128()?;
                    let declared = if has_maximum {
                        section.leb128()?
                    } else {
                        initial
                    };
                    let requested_bytes = declared.saturating_mul(WASM_PAGE_BYTES);
                    inspection.declared_memory_bytes =
                        inspection.declared_memory_bytes.max(requested_bytes);
                    if requested_bytes > limits.memory_bytes {
                        return Err(ModuleInspectionError::MemoryAboveCeiling {
                            maximum_bytes: limits.memory_bytes,
                            requested_bytes,
                        });
                    }
                }
            }
            EXPORT_SECTION => {
                let mut section = Cursor::new(body);
                let count = section.leb128()?;
                if count as usize > MAX_DECLARATIONS {
                    return Err(ModuleInspectionError::TooManyDeclarations);
                }
                for _ in 0..count {
                    let name = section.name()?;
                    let kind = section.byte()?;
                    section.leb128()?;
                    if name == required_export && kind != 0 {
                        return Err(ModuleInspectionError::ExportIsNotAFunction(name));
                    }
                    inspection.exports.push(name);
                }
            }
            _ => {}
        }
    }

    if !inspection
        .exports
        .iter()
        .any(|export| export == required_export)
    {
        return Err(ModuleInspectionError::MissingExport(
            required_export.to_string(),
        ));
    }
    Ok(inspection)
}

struct Cursor<'a> {
    bytes: &'a [u8],
}

impl<'a> Cursor<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes }
    }

    fn is_empty(&self) -> bool {
        self.bytes.is_empty()
    }

    fn byte(&mut self) -> Result<u8, ModuleInspectionError> {
        let (first, rest) = self
            .bytes
            .split_first()
            .ok_or(ModuleInspectionError::Truncated)?;
        self.bytes = rest;
        Ok(*first)
    }

    fn take(&mut self, length: usize) -> Result<&'a [u8], ModuleInspectionError> {
        if self.bytes.len() < length {
            return Err(ModuleInspectionError::Truncated);
        }
        let (body, rest) = self.bytes.split_at(length);
        self.bytes = rest;
        Ok(body)
    }

    /// Unsigned LEB128, capped at five groups so a padded encoding cannot spin.
    fn leb128(&mut self) -> Result<u64, ModuleInspectionError> {
        let mut value = 0_u64;
        for shift in 0..5 {
            let byte = self.byte()?;
            value |= u64::from(byte & 0x7f) << (shift * 7);
            if byte & 0x80 == 0 {
                return Ok(value);
            }
        }
        Err(ModuleInspectionError::Truncated)
    }

    fn name(&mut self) -> Result<String, ModuleInspectionError> {
        let length =
            usize::try_from(self.leb128()?).map_err(|_| ModuleInspectionError::Truncated)?;
        let bytes = self.take(length)?;
        String::from_utf8(bytes.to_vec()).map_err(|_| ModuleInspectionError::Truncated)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contexts::tooling::skill_tools::domain::DEFAULT_SKILL_TOOL_LIMITS;

    fn header() -> Vec<u8> {
        let mut bytes = WASM_MAGIC.to_vec();
        bytes.extend_from_slice(&WASM_VERSION);
        bytes
    }

    fn section(id: u8, body: Vec<u8>) -> Vec<u8> {
        let mut bytes = vec![id, body.len() as u8];
        bytes.extend(body);
        bytes
    }

    fn name(value: &str) -> Vec<u8> {
        let mut bytes = vec![value.len() as u8];
        bytes.extend_from_slice(value.as_bytes());
        bytes
    }

    fn export_section(entries: &[(&str, u8)]) -> Vec<u8> {
        let mut body = vec![entries.len() as u8];
        for (export, kind) in entries {
            body.extend(name(export));
            body.push(*kind);
            body.push(0);
        }
        section(EXPORT_SECTION, body)
    }

    fn import_section(entries: &[(&str, &str)]) -> Vec<u8> {
        let mut body = vec![entries.len() as u8];
        for (module, function) in entries {
            body.extend(name(module));
            body.extend(name(function));
            body.push(0);
            body.push(0);
        }
        section(IMPORT_SECTION, body)
    }

    fn memory_section(pages: u32) -> Vec<u8> {
        let mut body = vec![1, 1];
        body.extend(leb(u64::from(pages)));
        body.extend(leb(u64::from(pages)));
        section(MEMORY_SECTION, body)
    }

    fn leb(mut value: u64) -> Vec<u8> {
        let mut bytes = Vec::new();
        loop {
            let byte = (value & 0x7f) as u8;
            value >>= 7;
            if value == 0 {
                bytes.push(byte);
                return bytes;
            }
            bytes.push(byte | 0x80);
        }
    }

    #[test]
    fn a_pure_module_exporting_its_entry_point_passes_inspection() {
        let mut module = header();
        module.extend(memory_section(16));
        module.extend(export_section(&[("run", 0), ("memory", 2)]));
        let inspection =
            inspect_module(&module, "run", &DEFAULT_SKILL_TOOL_LIMITS).expect("inspection");
        assert_eq!(inspection.exports, vec!["run", "memory"]);
        assert!(inspection.imports.is_empty());
        assert_eq!(inspection.declared_memory_bytes, 16 * 64 * 1024);
    }

    #[test]
    fn only_the_single_host_import_module_is_allowed() {
        let mut allowed = header();
        allowed.extend(import_section(&[(HOST_IMPORT_MODULE, "host_call")]));
        allowed.extend(export_section(&[("run", 0)]));
        assert_eq!(
            inspect_module(&allowed, "run", &DEFAULT_SKILL_TOOL_LIMITS)
                .expect("inspection")
                .imports,
            vec!["vanehub::host_call"]
        );

        for module in ["wasi_snapshot_preview1", "env", "wasi_unstable"] {
            let mut forbidden = header();
            forbidden.extend(import_section(&[(module, "fd_write")]));
            forbidden.extend(export_section(&[("run", 0)]));
            assert_eq!(
                inspect_module(&forbidden, "run", &DEFAULT_SKILL_TOOL_LIMITS),
                Err(ModuleInspectionError::ForbiddenImport {
                    module: module.to_string(),
                    name: "fd_write".to_string()
                })
            );
        }
    }

    #[test]
    fn a_missing_or_non_function_entry_point_fails_before_execution() {
        let mut missing = header();
        missing.extend(export_section(&[("other", 0)]));
        assert_eq!(
            inspect_module(&missing, "run", &DEFAULT_SKILL_TOOL_LIMITS),
            Err(ModuleInspectionError::MissingExport("run".to_string()))
        );

        let mut wrong_kind = header();
        wrong_kind.extend(export_section(&[("run", 2)]));
        assert_eq!(
            inspect_module(&wrong_kind, "run", &DEFAULT_SKILL_TOOL_LIMITS),
            Err(ModuleInspectionError::ExportIsNotAFunction(
                "run".to_string()
            ))
        );
    }

    #[test]
    fn declared_memory_above_the_ceiling_is_refused() {
        let mut greedy = header();
        greedy.extend(memory_section(2048));
        greedy.extend(export_section(&[("run", 0)]));
        assert_eq!(
            inspect_module(&greedy, "run", &DEFAULT_SKILL_TOOL_LIMITS),
            Err(ModuleInspectionError::MemoryAboveCeiling {
                maximum_bytes: 64 * 1024 * 1024,
                requested_bytes: 2048 * 64 * 1024
            })
        );
    }

    #[test]
    fn non_modules_and_truncated_modules_fail_closed() {
        assert_eq!(
            inspect_module(
                b"MZ\x90\x00 native binary",
                "run",
                &DEFAULT_SKILL_TOOL_LIMITS
            ),
            Err(ModuleInspectionError::NotWebAssembly)
        );
        assert_eq!(
            inspect_module(b"", "run", &DEFAULT_SKILL_TOOL_LIMITS),
            Err(ModuleInspectionError::NotWebAssembly)
        );

        let mut wrong_version = WASM_MAGIC.to_vec();
        wrong_version.extend_from_slice(&[0x02, 0x00, 0x00, 0x00]);
        assert_eq!(
            inspect_module(&wrong_version, "run", &DEFAULT_SKILL_TOOL_LIMITS),
            Err(ModuleInspectionError::UnsupportedVersion)
        );

        let mut truncated = header();
        truncated.extend([EXPORT_SECTION, 40, 1]);
        assert_eq!(
            inspect_module(&truncated, "run", &DEFAULT_SKILL_TOOL_LIMITS),
            Err(ModuleInspectionError::Truncated)
        );
    }

    #[test]
    fn unknown_sections_are_skipped_by_length_without_being_interpreted() {
        let mut module = header();
        module.extend(section(0, b"custom producer data".to_vec()));
        module.extend(section(10, vec![0xff, 0xff, 0xff]));
        module.extend(export_section(&[("run", 0)]));
        assert!(inspect_module(&module, "run", &DEFAULT_SKILL_TOOL_LIMITS).is_ok());
    }
}
