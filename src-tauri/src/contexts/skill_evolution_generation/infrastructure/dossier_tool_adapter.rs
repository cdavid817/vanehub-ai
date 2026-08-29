use crate::contexts::skill_evolution_generation::{
    application::{DossierSectionToolPort, GenerationToolError, GenerationToolSafeResultV1},
    domain::DossierSectionPageRequest,
};
use rusqlite::Connection;

use super::GenerationDossierQuery;

pub(crate) struct SqliteDossierSectionToolAdapter<'connection> {
    connection: &'connection Connection,
    input_witness_hash: String,
}

impl<'connection> SqliteDossierSectionToolAdapter<'connection> {
    pub(crate) fn new(connection: &'connection Connection, input_witness_hash: &str) -> Self {
        Self {
            connection,
            input_witness_hash: input_witness_hash.into(),
        }
    }
}

impl DossierSectionToolPort for SqliteDossierSectionToolAdapter<'_> {
    fn read_section(
        &self,
        dossier_id: &str,
        ordinal: u8,
        cursor: Option<&str>,
        limit: u16,
    ) -> Result<GenerationToolSafeResultV1, GenerationToolError> {
        let page = GenerationDossierQuery::new(self.connection)
            .section_page(&DossierSectionPageRequest {
                dossier_id,
                ordinal,
                cursor,
                limit,
            })
            .map_err(|_| GenerationToolError::InvalidArgument)?;
        let citation = format!(
            "dossier:{dossier_id}:section:{ordinal}:{}",
            page.section_hash
        );
        let safe_value = serde_json::to_value(page).map_err(|_| GenerationToolError::Failed)?;
        Ok(GenerationToolSafeResultV1 {
            safe_value,
            citations: vec![citation],
            source_witness_hash: self.input_witness_hash.clone(),
        })
    }
}
