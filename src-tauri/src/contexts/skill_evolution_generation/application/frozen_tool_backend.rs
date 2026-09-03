use serde_json::json;

use super::{
    parse_generation_response, GenerationModelStage, GenerationToolArgumentsV1,
    GenerationToolBackendPort, GenerationToolError, GenerationToolName, GenerationToolSafeResultV1,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FrozenToolExcerptV1 {
    pub(crate) excerpt_id: String,
    pub(crate) logical_location: String,
    pub(crate) safe_text: String,
    pub(crate) source_witness_hash: String,
}

pub(crate) trait DossierSectionToolPort {
    fn read_section(
        &self,
        dossier_id: &str,
        ordinal: u8,
        cursor: Option<&str>,
        limit: u16,
    ) -> Result<GenerationToolSafeResultV1, GenerationToolError>;
}

pub(crate) trait PreviewSimulationToolPort {
    fn simulate(
        &self,
        structure_hash: &str,
    ) -> Result<GenerationToolSafeResultV1, GenerationToolError>;
}

pub(crate) struct FrozenGenerationToolBackend<'ports> {
    pub(crate) dossier: &'ports dyn DossierSectionToolPort,
    pub(crate) preview: &'ports dyn PreviewSimulationToolPort,
    pub(crate) excerpts: &'ports [FrozenToolExcerptV1],
    pub(crate) input_witness_hash: &'ports str,
}

impl GenerationToolBackendPort for FrozenGenerationToolBackend<'_> {
    fn execute(
        &self,
        name: GenerationToolName,
        arguments: &GenerationToolArgumentsV1,
    ) -> Result<GenerationToolSafeResultV1, GenerationToolError> {
        match (name, arguments) {
            (
                GenerationToolName::ReadDossierSection,
                GenerationToolArgumentsV1::ReadDossierSection {
                    dossier_id,
                    ordinal,
                    cursor,
                    limit,
                },
            ) => self
                .dossier
                .read_section(dossier_id, *ordinal, cursor.as_deref(), *limit),
            (
                GenerationToolName::ReadSkillExcerpt,
                GenerationToolArgumentsV1::ReadSkillExcerpt { excerpt_id },
            ) => self.read_excerpt(excerpt_id),
            (
                GenerationToolName::FindExactAnchor,
                GenerationToolArgumentsV1::FindExactAnchor { query },
            ) => self.find_anchor(query),
            (
                GenerationToolName::ValidateDraftStructure,
                GenerationToolArgumentsV1::ValidateDraftStructure { response_json },
            ) => {
                parse_generation_response(
                    GenerationModelStage::SynthesizeStructuredDraft,
                    response_json,
                )
                .map_err(|_| GenerationToolError::InvalidArgument)?;
                Ok(self.result(json!({"valid": true}), vec!["draft_structure".into()]))
            }
            (
                GenerationToolName::SimulateLocalPreview,
                GenerationToolArgumentsV1::SimulateLocalPreview { structure_hash },
            ) => self.preview.simulate(structure_hash),
            _ => Err(GenerationToolError::PolicyDenied),
        }
    }
}

impl FrozenGenerationToolBackend<'_> {
    fn read_excerpt(
        &self,
        excerpt_id: &str,
    ) -> Result<GenerationToolSafeResultV1, GenerationToolError> {
        let excerpt = self
            .excerpts
            .iter()
            .find(|value| value.excerpt_id == excerpt_id)
            .ok_or(GenerationToolError::InvalidArgument)?;
        if excerpt.source_witness_hash != self.input_witness_hash {
            return Err(GenerationToolError::StaleWitness);
        }
        Ok(self.result(
            json!({"excerptId": excerpt.excerpt_id, "logicalLocation": excerpt.logical_location,
            "safeText": excerpt.safe_text}),
            vec![excerpt.excerpt_id.clone()],
        ))
    }

    fn find_anchor(&self, query: &str) -> Result<GenerationToolSafeResultV1, GenerationToolError> {
        if query.is_empty() || query.len() > 512 {
            return Err(GenerationToolError::InvalidArgument);
        }
        let matches: Vec<_> = self
            .excerpts
            .iter()
            .flat_map(|excerpt| {
                excerpt
                    .safe_text
                    .match_indices(query)
                    .map(move |(offset, _)| (excerpt, offset))
            })
            .collect();
        let [(excerpt, offset)] = matches.as_slice() else {
            return Err(GenerationToolError::InvalidArgument);
        };
        Ok(self.result(
            json!({"excerptId": excerpt.excerpt_id, "logicalLocation": excerpt.logical_location,
                "byteOffset": offset, "exactMatch": query}),
            vec![excerpt.excerpt_id.clone()],
        ))
    }

    fn result(
        &self,
        safe_value: serde_json::Value,
        citations: Vec<String>,
    ) -> GenerationToolSafeResultV1 {
        GenerationToolSafeResultV1 {
            safe_value,
            citations,
            source_witness_hash: self.input_witness_hash.into(),
        }
    }
}
