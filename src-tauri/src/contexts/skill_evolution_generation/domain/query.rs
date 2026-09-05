use serde::{Deserialize, Serialize};

use super::{
    DossierRecordV1, DossierSectionKind, DossierSectionStatus, DossierSourceWitnessV1,
    DossierTruncationV1,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DossierSectionPageRequest<'a> {
    pub(crate) dossier_id: &'a str,
    pub(crate) ordinal: u8,
    pub(crate) cursor: Option<&'a str>,
    pub(crate) limit: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct DossierSectionPageV1 {
    pub(crate) dossier_id: String,
    pub(crate) dossier_revision: u64,
    pub(crate) ordinal: u8,
    pub(crate) kind: DossierSectionKind,
    pub(crate) status: DossierSectionStatus,
    pub(crate) source_witnesses: Vec<DossierSourceWitnessV1>,
    pub(crate) records: Vec<DossierRecordV1>,
    pub(crate) truncation: DossierTruncationV1,
    pub(crate) unavailable_reason_code: Option<String>,
    pub(crate) section_hash: String,
    pub(crate) next_cursor: Option<String>,
    pub(crate) page_complete: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct DossierSourceLinkV1 {
    pub(crate) link_kind: String,
    pub(crate) linked_id: String,
    pub(crate) linked_revision: String,
    pub(crate) witness_hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct DossierSourceLinkPageV1 {
    pub(crate) dossier_id: String,
    pub(crate) links: Vec<DossierSourceLinkV1>,
    pub(crate) next_cursor: Option<String>,
}
