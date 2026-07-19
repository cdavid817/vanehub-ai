#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum ExtensionCapabilityId {
    Ocr,
    Asr,
    Tts,
}

impl ExtensionCapabilityId {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Ocr => "ocr",
            Self::Asr => "asr",
            Self::Tts => "tts",
        }
    }

    pub(crate) fn parse(value: &str) -> Option<Self> {
        match value {
            "ocr" => Some(Self::Ocr),
            "asr" => Some(Self::Asr),
            "tts" => Some(Self::Tts),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum ExtensionFrameworkId {
    Paddleocr,
    FasterWhisper,
    SherpaOnnx,
}

impl ExtensionFrameworkId {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Paddleocr => "paddleocr",
            Self::FasterWhisper => "faster-whisper",
            Self::SherpaOnnx => "sherpa-onnx",
        }
    }

    pub(crate) fn parse(value: &str) -> Option<Self> {
        match value {
            "paddleocr" => Some(Self::Paddleocr),
            "faster-whisper" => Some(Self::FasterWhisper),
            "sherpa-onnx" => Some(Self::SherpaOnnx),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ExtensionModelRequirement {
    pub(crate) id: &'static str,
    pub(crate) size_mb: u64,
    pub(crate) description_key: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ExtensionRequirement {
    pub(crate) runtime: &'static str,
    pub(crate) packages: &'static [&'static str],
    pub(crate) import_module: &'static str,
    pub(crate) version_package: &'static str,
    pub(crate) estimated_download_mb: u64,
    pub(crate) estimated_disk_mb: u64,
    pub(crate) models: &'static [ExtensionModelRequirement],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ExtensionFrameworkDefinition {
    pub(crate) id: ExtensionFrameworkId,
    pub(crate) capability_id: ExtensionCapabilityId,
    pub(crate) name_key: &'static str,
    pub(crate) description_key: &'static str,
    pub(crate) default_port: u16,
    pub(crate) requirement: ExtensionRequirement,
}

const PADDLEOCR_MODELS: [ExtensionModelRequirement; 1] = [ExtensionModelRequirement {
    id: "PP-OCRv5-mobile",
    size_mb: 120,
    description_key: "extensions.model.paddleocr",
}];

const FASTER_WHISPER_MODELS: [ExtensionModelRequirement; 1] = [ExtensionModelRequirement {
    id: "base",
    size_mb: 150,
    description_key: "extensions.model.fasterWhisper",
}];

const SHERPA_ONNX_MODELS: [ExtensionModelRequirement; 1] = [ExtensionModelRequirement {
    id: "vits-zh-aishell3",
    size_mb: 170,
    description_key: "extensions.model.sherpaOnnx",
}];

const EXTENSION_DEFINITIONS: [ExtensionFrameworkDefinition; 3] = [
    ExtensionFrameworkDefinition {
        id: ExtensionFrameworkId::Paddleocr,
        capability_id: ExtensionCapabilityId::Ocr,
        name_key: "extensions.framework.paddleocr.name",
        description_key: "extensions.framework.paddleocr.description",
        default_port: 9875,
        requirement: ExtensionRequirement {
            runtime: "Python 3.10+",
            packages: &["paddleocr>=3,<4", "paddlepaddle>=3,<4"],
            import_module: "paddleocr",
            version_package: "paddleocr",
            estimated_download_mb: 650,
            estimated_disk_mb: 1800,
            models: &PADDLEOCR_MODELS,
        },
    },
    ExtensionFrameworkDefinition {
        id: ExtensionFrameworkId::FasterWhisper,
        capability_id: ExtensionCapabilityId::Asr,
        name_key: "extensions.framework.fasterWhisper.name",
        description_key: "extensions.framework.fasterWhisper.description",
        default_port: 9876,
        requirement: ExtensionRequirement {
            runtime: "Python 3.10+",
            packages: &["faster-whisper>=1,<2"],
            import_module: "faster_whisper",
            version_package: "faster-whisper",
            estimated_download_mb: 250,
            estimated_disk_mb: 900,
            models: &FASTER_WHISPER_MODELS,
        },
    },
    ExtensionFrameworkDefinition {
        id: ExtensionFrameworkId::SherpaOnnx,
        capability_id: ExtensionCapabilityId::Tts,
        name_key: "extensions.framework.sherpaOnnx.name",
        description_key: "extensions.framework.sherpaOnnx.description",
        default_port: 9879,
        requirement: ExtensionRequirement {
            runtime: "Python 3.10+",
            packages: &["sherpa-onnx>=1,<2"],
            import_module: "sherpa_onnx",
            version_package: "sherpa-onnx",
            estimated_download_mb: 180,
            estimated_disk_mb: 650,
            models: &SHERPA_ONNX_MODELS,
        },
    },
];

pub(crate) fn definitions() -> &'static [ExtensionFrameworkDefinition] {
    &EXTENSION_DEFINITIONS
}

pub(crate) fn definition(id: ExtensionFrameworkId) -> ExtensionFrameworkDefinition {
    match id {
        ExtensionFrameworkId::Paddleocr => EXTENSION_DEFINITIONS[0],
        ExtensionFrameworkId::FasterWhisper => EXTENSION_DEFINITIONS[1],
        ExtensionFrameworkId::SherpaOnnx => EXTENSION_DEFINITIONS[2],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_preserves_allowlisted_contract_and_execution_metadata() {
        assert_eq!(
            definitions()
                .iter()
                .map(|item| (item.id.as_str(), item.capability_id.as_str()))
                .collect::<Vec<_>>(),
            vec![
                ("paddleocr", "ocr"),
                ("faster-whisper", "asr"),
                ("sherpa-onnx", "tts")
            ]
        );
        let paddleocr = definition(ExtensionFrameworkId::Paddleocr);
        assert_eq!(
            paddleocr.requirement.packages,
            &["paddleocr>=3,<4", "paddlepaddle>=3,<4"]
        );
        assert_eq!(paddleocr.requirement.import_module, "paddleocr");
        assert_eq!(paddleocr.requirement.version_package, "paddleocr");
        assert_eq!(paddleocr.default_port, 9875);
    }

    #[test]
    fn unknown_identifiers_are_rejected_instead_of_falling_back() {
        assert_eq!(
            ExtensionFrameworkId::parse("faster-whisper"),
            Some(ExtensionFrameworkId::FasterWhisper)
        );
        assert!(ExtensionFrameworkId::parse("unknown").is_none());
        assert!(ExtensionCapabilityId::parse("vision").is_none());
    }
}
