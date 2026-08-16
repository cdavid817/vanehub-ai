use super::dto;
use crate::contexts::agent_runtime::domain;

pub(crate) fn page_to_dto(
    page: domain::ContextEvidenceManifestPage,
) -> dto::ContextEvidenceManifestPage {
    dto::ContextEvidenceManifestPage {
        items: page.items.into_iter().map(manifest_to_dto).collect(),
        next_cursor: page.next_cursor,
    }
}

pub(crate) fn manifest_to_dto(
    manifest: domain::ContextEvidenceManifest,
) -> dto::ContextEvidenceManifest {
    dto::ContextEvidenceManifest {
        session_id: manifest.session_id,
        turn_id: manifest.turn_id,
        generation_id: manifest.generation_id,
        policy_version: manifest.policy_version,
        evidence_budget: manifest.evidence_budget,
        occupied_tokens: manifest.occupied_tokens,
        selected: manifest
            .selected
            .into_iter()
            .map(|item| dto::ContextEvidenceSummary {
                id: item.id,
                source_kind: item.source_kind.as_str().to_string(),
                source_ref: item.source_ref,
                start_line: item.range.map(|range| range.start_line),
                end_line: item.range.map(|range| range.end_line),
                symbol: item.symbol,
                token_estimate: item.token_estimate,
                reason_codes: item
                    .reasons
                    .into_iter()
                    .map(|reason| reason.as_str().to_string())
                    .collect(),
            })
            .collect(),
        rejected: manifest
            .rejected
            .into_iter()
            .map(|(id, reason)| dto::ContextEvidenceRejection {
                id,
                reason_code: reason.as_str().to_string(),
            })
            .collect(),
        source_outcomes: manifest
            .source_outcomes
            .into_iter()
            .map(|(kind, outcome)| {
                let outcome = outcome.as_str().replace('-', "_");
                (kind.as_str().to_string(), outcome)
            })
            .collect(),
        duplicate_tokens_saved: manifest.duplicate_tokens_saved,
        collection_latency_bucket: manifest.collection_latency_bucket,
        ranking_latency_bucket: manifest.ranking_latency_bucket,
        compaction_triggered: manifest.compaction_triggered,
        runtime: "desktop",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contexts::agent_runtime::domain::{
        ContextEvidenceSummary, ContextReasonCode, ContextSourceKind, ContextSourceOutcome,
    };
    use std::collections::BTreeMap;

    #[test]
    fn desktop_contract_is_camel_case_bounded_and_content_free() {
        let dto = manifest_to_dto(domain::ContextEvidenceManifest {
            session_id: "session".to_string(),
            turn_id: "turn".to_string(),
            generation_id: "generation".to_string(),
            recorded_at: "100".to_string(),
            policy_version: "context-engine-v1".to_string(),
            evidence_budget: 100,
            occupied_tokens: 10,
            selected: vec![ContextEvidenceSummary {
                id: "evidence".to_string(),
                source_kind: ContextSourceKind::Retrieval,
                source_ref: "src/lib.rs".to_string(),
                range: None,
                symbol: None,
                token_estimate: 10,
                safe_fingerprint: "safe".to_string(),
                reasons: vec![ContextReasonCode::SemanticMatch],
            }],
            rejected: Vec::new(),
            source_outcomes: BTreeMap::from([(
                ContextSourceKind::LspDefinition,
                ContextSourceOutcome::TimedOut,
            )]),
            duplicate_tokens_saved: 0,
            collection_latency_bucket: "sub-10ms".to_string(),
            ranking_latency_bucket: "sub-10ms".to_string(),
            compaction_triggered: false,
        });
        let value = serde_json::to_value(dto).expect("serialize");
        assert_eq!(value["runtime"], "desktop");
        assert_eq!(value["sourceOutcomes"]["lsp-definition"], "timed_out");
        assert_eq!(value["selected"][0]["reasonCodes"][0], "semantic-match");
        assert!(!value.to_string().contains("secret source body"));
    }
}
