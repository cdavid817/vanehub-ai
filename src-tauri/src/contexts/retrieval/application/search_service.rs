use crate::contexts::retrieval::domain::{
    cosine_similarity, escape_fts_query, fuse_with_rrf, Degradation, MatchedVia, RetrievalError,
    RetrievalQuery, RetrievalScope, ScoredHit, SourceKind,
};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use super::indexing_service::truncate_for_embedding;
use super::ports::{
    AuthorizedHitResolverPort, AuthorizedSourceSet, EmbeddingPort,
    RetrievalConfigurationRepository, RetrievalDocumentRepository,
};

/// `degraded` 为 `None` 时 `hits` 为空表示"搜了，确实没有"；`degraded` 为 `Some` 时 `hits`
/// 仍可能非空，表示"某一路搜不了，用另一路的结果兜底"——两者是不同的语义，不能互相替代。
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct SearchOutcome {
    pub(crate) hits: Vec<ScoredHit>,
    pub(crate) degraded: Option<Degradation>,
}

/// 双路（向量 + 关键词）混合检索，只服务 `SourceKind::AgentMemory`。
///
/// Governed by construction: every search takes the complete authorized source set its caller
/// resolved and a resolver bound to that caller's read context. Both paths are filtered through
/// the set *before* their top-k, so an unauthorized neighbour can never occupy a slot a valid
/// record should have had, and every delivered hit is re-read by the resolver at its pinned
/// version. There is deliberately no method that searches without a set.
pub(crate) struct SearchService {
    configuration: Arc<dyn RetrievalConfigurationRepository>,
    repository: Arc<dyn RetrievalDocumentRepository>,
    embeddings: Arc<dyn EmbeddingPort>,
    source_kind: SourceKind,
}

impl SearchService {
    pub(crate) fn new(
        configuration: Arc<dyn RetrievalConfigurationRepository>,
        repository: Arc<dyn RetrievalDocumentRepository>,
        embeddings: Arc<dyn EmbeddingPort>,
    ) -> Self {
        debug_assert!(RetrievalScope::GlobalMemory
            .validate_for(SourceKind::AgentMemory)
            .is_ok());
        Self {
            configuration,
            repository,
            embeddings,
            source_kind: SourceKind::AgentMemory,
        }
    }

    /// 授权域内双路召回 → RRF 融合 → 经调用方的受治理读取回源。
    ///
    /// 铁律：检索失败**永不**让生成失败（设计文档 §8.1）。除"未配置"外，任何一路的失败都只是
    /// 降级。An incomplete authority set is the one input that is refused outright: searching a
    /// partial relation would silently drop the eligible records past its cut, which is worse
    /// than telling the model the search is unavailable.
    pub(crate) fn search(
        &self,
        query: &RetrievalQuery,
        authority: &AuthorizedSourceSet,
        resolver: &dyn AuthorizedHitResolverPort,
    ) -> Result<SearchOutcome, RetrievalError> {
        let configuration = self.configuration.load()?;
        let Some((_profile, model)) = configuration.resolved_model() else {
            return Err(RetrievalError::NotConfigured);
        };
        if !authority.complete {
            return Err(RetrievalError::Unavailable);
        }
        // An admitted but empty pool is an ordinary empty success. Nothing is embedded for it:
        // there is no candidate a query vector could rank.
        if authority.source_ids.is_empty() {
            return Ok(SearchOutcome {
                hits: Vec::new(),
                degraded: None,
            });
        }
        let over_fetch = query.limit.saturating_mul(4).max(query.limit);

        // query 是模型自撰的，长度不受任何约束。用与索引侧相同的上限截断：超长 query 会让
        // embedding 调用直接失败（对用户表现为无声的 `keyword_only` 降级），同时把几千 token
        // 的短语塞进 FTS。
        let text = truncate_for_embedding(&query.text);

        // The query embedding is the one network call, and it happens before the query-local
        // relation is materialized: the repository holds nothing across it.
        let query_vector = self
            .embeddings
            .embed(model, std::slice::from_ref(&text))
            .ok()
            .and_then(|vectors| vectors.into_iter().next());
        let rows = self.repository.authorized_candidates(
            self.source_kind,
            authority,
            query_vector.as_ref().map(|_| model),
            Some(&escape_fts_query(&text)),
            over_fetch,
        )?;

        let vector_ranking = match (&query_vector, rows.vector) {
            (Some(query_vector), Ok(candidates)) => {
                Some(rank_by_similarity(query_vector, candidates, over_fetch))
            }
            _ => None,
        };
        let keyword_ranking = rows.keyword.ok();

        // 两路都失败必须与"两路都可用但都没命中"区分开。复用已有的 Err 路径而不是新增一种
        // 降级值：`execute_recall` 已有分支会把 Err 转成**成功的**工具结果"检索暂时不可用"。
        let degraded = match (&vector_ranking, &keyword_ranking) {
            (None, None) => return Err(RetrievalError::Unavailable),
            (None, Some(_)) => Some(Degradation::KeywordOnly),
            (Some(_), None) => Some(Degradation::VectorOnly),
            _ => None,
        };
        let vector_ids = vector_ranking.unwrap_or_default();
        let keyword_ids = keyword_ranking.unwrap_or_default();

        let fused = fuse_with_rrf(&[vector_ids.clone(), keyword_ids.clone()]);
        let in_vector: HashSet<&str> = vector_ids.iter().map(String::as_str).collect();
        let in_keyword: HashSet<&str> = keyword_ids.iter().map(String::as_str).collect();

        // 回源拿权威内容：经调用方的受治理读取，按候选 id 与其钉住的版本；版本已变、已删、
        // 已撤销的条目缺席。取的是**全部**融合候选而不是前 `limit` 条——`take(limit)` 在跳过
        // 陈旧条目*之后*才截断。候选数有界（两路各至多 `over_fetch` 条）。
        let wanted: Vec<String> = fused
            .iter()
            .map(|(source_id, _score)| source_id.clone())
            .collect();
        let resolved: HashMap<String, super::ports::ResolvedHit> = resolver
            .resolve(&wanted)?
            .into_iter()
            .map(|hit| (hit.source_id.clone(), hit))
            .collect();

        let hits = fused
            .into_iter()
            .filter_map(|(source_id, score)| {
                let record = resolved.get(&source_id)?;
                let matched_via = match (
                    in_vector.contains(source_id.as_str()),
                    in_keyword.contains(source_id.as_str()),
                ) {
                    (true, true) => MatchedVia::Both,
                    (true, false) => MatchedVia::Vector,
                    _ => MatchedVia::Keyword,
                };
                Some(ScoredHit {
                    source_id,
                    content: record.content.clone(),
                    created_at: record.created_at.clone(),
                    score,
                    matched_via,
                })
            })
            .take(query.limit)
            .collect();

        Ok(SearchOutcome { hits, degraded })
    }
}

fn rank_by_similarity(
    query_vector: &[f32],
    candidates: Vec<(String, Vec<f32>)>,
    limit: usize,
) -> Vec<String> {
    let mut scored: Vec<(String, f32)> = candidates
        .into_iter()
        .filter_map(|(source_id, vector)| {
            cosine_similarity(query_vector, &vector).map(|score| (source_id, score))
        })
        .collect();
    scored.sort_by(|left, right| {
        right
            .1
            .partial_cmp(&left.1)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| left.0.cmp(&right.0))
    });
    scored.into_iter().take(limit).map(|(id, _)| id).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contexts::retrieval::application::indexing_service::EMBEDDING_CONTENT_LIMIT;
    use crate::contexts::retrieval::application::ports::{
        AuthorizedCandidateRows, EmbeddingFailure, ResolvedHit, RetrievalConfiguration,
        RetrievalIndexStatus,
    };
    use crate::contexts::retrieval::domain::{FailureCategory, RetrievalDocument};
    use std::sync::Mutex;

    enum FakeEmbedder {
        Succeeds(Vec<f32>),
        Fails,
        Recording(Vec<f32>, Arc<Mutex<Vec<String>>>),
    }

    impl EmbeddingPort for FakeEmbedder {
        fn embed(
            &self,
            _model: &str,
            inputs: &[String],
        ) -> Result<Vec<Vec<f32>>, EmbeddingFailure> {
            match self {
                Self::Succeeds(vector) => Ok(vec![vector.clone()]),
                Self::Fails => Err(EmbeddingFailure {
                    category: FailureCategory::Network,
                    retry_after: None,
                    message: "fake embedding failure".to_string(),
                }),
                Self::Recording(vector, received) => {
                    received.lock().expect("lock").extend_from_slice(inputs);
                    Ok(vec![vector.clone()])
                }
            }
        }
    }

    struct FakeConfigurationRepository {
        configuration: RetrievalConfiguration,
    }

    impl FakeConfigurationRepository {
        fn configured(profile: &str, model: &str) -> Self {
            Self {
                configuration: RetrievalConfiguration {
                    source_profile_id: Some(profile.to_string()),
                    embedding_model: Some(model.to_string()),
                    automatic_code_index_mode: Default::default(),
                },
            }
        }

        fn unconfigured() -> Self {
            Self {
                configuration: RetrievalConfiguration::default(),
            }
        }
    }

    impl RetrievalConfigurationRepository for FakeConfigurationRepository {
        fn load(&self) -> Result<RetrievalConfiguration, RetrievalError> {
            Ok(self.configuration.clone())
        }

        fn save(&self, _profile_id: &str, _embedding_model: &str) -> Result<(), RetrievalError> {
            unimplemented!("not exercised by search_service tests")
        }

        fn save_automatic_code_index_mode(
            &self,
            _mode: crate::contexts::retrieval::domain::CodeIndexAutomaticMode,
        ) -> Result<(), RetrievalError> {
            unimplemented!("not exercised by search_service tests")
        }
    }

    /// The whole pool as the repository holds it. `authorized_candidates` applies the authority
    /// set itself, which is exactly the contract the real repository fulfils with a temp table:
    /// a row outside the set is never a candidate, however well it would have ranked.
    struct FakeRepository {
        vectors: Result<Vec<(String, Vec<f32>)>, RetrievalError>,
        keywords: Result<Vec<String>, RetrievalError>,
        received_keyword_query: Mutex<Option<String>>,
        received_authority: Mutex<Vec<Vec<String>>>,
    }

    impl FakeRepository {
        fn new(
            vectors: Result<Vec<(String, Vec<f32>)>, RetrievalError>,
            keywords: Result<Vec<String>, RetrievalError>,
        ) -> Self {
            Self {
                vectors,
                keywords,
                received_keyword_query: Mutex::new(None),
                received_authority: Mutex::new(Vec::new()),
            }
        }
    }

    impl RetrievalDocumentRepository for FakeRepository {
        fn upsert_pending(&self, _document: &RetrievalDocument) -> Result<(), RetrievalError> {
            unimplemented!("not exercised by search_service tests")
        }
        fn list_indexed_source_ids(
            &self,
            _source_kind: SourceKind,
        ) -> Result<Vec<(String, String)>, RetrievalError> {
            unimplemented!("not exercised by search_service tests")
        }
        fn delete_by_source(
            &self,
            _source_kind: SourceKind,
            _source_id: &str,
        ) -> Result<(), RetrievalError> {
            unimplemented!("not exercised by search_service tests")
        }
        fn claim_pending_batch(
            &self,
            _source_kind: SourceKind,
            _limit: usize,
        ) -> Result<Vec<RetrievalDocument>, RetrievalError> {
            unimplemented!("not exercised by search_service tests")
        }
        fn store_embedding(
            &self,
            _id: &str,
            _model: &str,
            _embedding: &[f32],
        ) -> Result<(), RetrievalError> {
            unimplemented!("not exercised by search_service tests")
        }
        fn record_failure(
            &self,
            _id: &str,
            _category: FailureCategory,
            _give_up: bool,
        ) -> Result<(), RetrievalError> {
            unimplemented!("not exercised by search_service tests")
        }
        fn vector_candidates(
            &self,
            _source_kind: SourceKind,
            _model: &str,
        ) -> Result<Vec<(String, Vec<f32>)>, RetrievalError> {
            unimplemented!("the governed search must not use the unscoped candidate query")
        }
        fn keyword_candidates(
            &self,
            _source_kind: SourceKind,
            _query: &str,
            _limit: usize,
        ) -> Result<Vec<String>, RetrievalError> {
            unimplemented!("the governed search must not use the unscoped candidate query")
        }
        fn index_status(&self) -> Result<RetrievalIndexStatus, RetrievalError> {
            unimplemented!("not exercised by search_service tests")
        }
        fn requeue_all(&self) -> Result<(), RetrievalError> {
            unimplemented!("not exercised by search_service tests")
        }
        fn requeue_stale_model(&self, _new_model: &str) -> Result<(), RetrievalError> {
            unimplemented!("not exercised by search_service tests")
        }
        fn authorized_candidates(
            &self,
            _source_kind: SourceKind,
            authority: &AuthorizedSourceSet,
            model: Option<&str>,
            keyword_query: Option<&str>,
            keyword_limit: usize,
        ) -> Result<AuthorizedCandidateRows, RetrievalError> {
            self.received_authority
                .lock()
                .expect("lock")
                .push(authority.source_ids.clone());
            *self.received_keyword_query.lock().expect("lock") = keyword_query.map(str::to_string);
            let admitted: HashSet<&str> = authority.source_ids.iter().map(String::as_str).collect();
            let vector = match (model, &self.vectors) {
                (None, _) => Err(RetrievalError::Unavailable),
                (Some(_), Ok(rows)) => Ok(rows
                    .iter()
                    .filter(|(id, _)| admitted.contains(id.as_str()))
                    .cloned()
                    .collect()),
                (Some(_), Err(error)) => Err(error.clone()),
            };
            let keyword = match &self.keywords {
                Ok(ids) => Ok(ids
                    .iter()
                    .filter(|id| admitted.contains(id.as_str()))
                    .take(keyword_limit)
                    .cloned()
                    .collect()),
                Err(error) => Err(error.clone()),
            };
            Ok(AuthorizedCandidateRows { vector, keyword })
        }
    }

    struct FakeResolver {
        records: Vec<ResolvedHit>,
        asked: Mutex<Vec<Vec<String>>>,
    }

    impl AuthorizedHitResolverPort for FakeResolver {
        fn resolve(&self, source_ids: &[String]) -> Result<Vec<ResolvedHit>, RetrievalError> {
            self.asked.lock().expect("lock").push(source_ids.to_vec());
            Ok(self
                .records
                .iter()
                .filter(|record| source_ids.contains(&record.source_id))
                .cloned()
                .collect())
        }
    }

    const PROFILE: &str = "profile-a";
    const MODEL: &str = "model-a";

    fn matching_vector() -> Vec<f32> {
        vec![1.0, 0.0]
    }

    fn sample_query(text: &str, limit: usize) -> RetrievalQuery {
        RetrievalQuery {
            text: text.to_string(),
            limit,
        }
    }

    fn record(source_id: &str, content: &str) -> ResolvedHit {
        ResolvedHit {
            source_id: source_id.to_string(),
            content: content.to_string(),
            created_at: "2026-08-05T00:00:00Z".to_string(),
        }
    }

    fn authority(ids: &[&str]) -> AuthorizedSourceSet {
        AuthorizedSourceSet {
            source_ids: ids.iter().map(|id| id.to_string()).collect(),
            complete: true,
        }
    }

    fn storage_failure() -> RetrievalError {
        RetrievalError::Storage("boom".to_string())
    }

    fn service(
        configuration: FakeConfigurationRepository,
        embedder: FakeEmbedder,
        vectors: Result<Vec<(String, Vec<f32>)>, RetrievalError>,
        keywords: Result<Vec<String>, RetrievalError>,
        records: Vec<ResolvedHit>,
    ) -> (SearchService, Arc<FakeRepository>, Arc<FakeResolver>) {
        let repository = Arc::new(FakeRepository::new(vectors, keywords));
        let resolver = Arc::new(FakeResolver {
            records,
            asked: Mutex::new(Vec::new()),
        });
        let search_service = SearchService::new(
            Arc::new(configuration),
            repository.clone(),
            Arc::new(embedder),
        );
        (search_service, repository, resolver)
    }

    fn healthy_service(
        vectors: Result<Vec<(String, Vec<f32>)>, RetrievalError>,
        keywords: Result<Vec<String>, RetrievalError>,
        records: Vec<ResolvedHit>,
    ) -> (SearchService, Arc<FakeRepository>, Arc<FakeResolver>) {
        service(
            FakeConfigurationRepository::configured(PROFILE, MODEL),
            FakeEmbedder::Succeeds(matching_vector()),
            vectors,
            keywords,
            records,
        )
    }

    #[test]
    fn both_paths_healthy_yields_no_degradation_and_marks_overlap_as_both() {
        let (service, _repository, resolver) = healthy_service(
            Ok(vec![("m1".to_string(), matching_vector())]),
            Ok(vec!["m1".to_string()]),
            vec![record("m1", "uses npm not pnpm")],
        );

        let outcome = service
            .search(
                &sample_query("npm", 5),
                &authority(&["m1"]),
                resolver.as_ref(),
            )
            .expect("search");

        assert_eq!(outcome.degraded, None);
        assert_eq!(outcome.hits.len(), 1);
        assert_eq!(outcome.hits[0].source_id, "m1");
        assert_eq!(outcome.hits[0].content, "uses npm not pnpm");
        assert_eq!(outcome.hits[0].matched_via, MatchedVia::Both);
    }

    #[test]
    fn unauthorized_rows_are_filtered_before_top_k_so_they_cannot_crowd_out_a_valid_hit() {
        // MR-12: five excluded rows outscore the one admitted record on both paths. Filtering
        // after ranking with limit 1 would return nothing; filtering before returns the admitted
        // record, and the resolver is never even asked about the excluded ids.
        let excluded: Vec<(String, Vec<f32>)> = (1..=5)
            .map(|index| (format!("x{index}"), matching_vector()))
            .collect();
        let mut vectors = excluded.clone();
        vectors.push(("ok".to_string(), vec![0.9, 0.1]));
        let mut keywords: Vec<String> = excluded.iter().map(|(id, _)| id.clone()).collect();
        keywords.push("ok".to_string());
        let (service, _repository, resolver) = healthy_service(
            Ok(vectors),
            Ok(keywords),
            vec![record("ok", "the valid one")],
        );

        let outcome = service
            .search(
                &sample_query("npm", 1),
                &authority(&["ok"]),
                resolver.as_ref(),
            )
            .expect("search");

        assert_eq!(outcome.hits.len(), 1);
        assert_eq!(outcome.hits[0].source_id, "ok");
        let asked = resolver.asked.lock().expect("lock");
        assert_eq!(asked.len(), 1);
        assert_eq!(asked[0], vec!["ok".to_string()]);
    }

    #[test]
    fn an_incomplete_authority_set_is_refused_rather_than_searched() {
        let (service, repository, resolver) = healthy_service(
            Ok(vec![("m1".to_string(), matching_vector())]),
            Ok(vec!["m1".to_string()]),
            vec![record("m1", "uses npm")],
        );
        let partial = AuthorizedSourceSet {
            source_ids: vec!["m1".to_string()],
            complete: false,
        };

        assert_eq!(
            service
                .search(&sample_query("npm", 5), &partial, resolver.as_ref())
                .unwrap_err(),
            RetrievalError::Unavailable
        );
        assert!(repository
            .received_authority
            .lock()
            .expect("lock")
            .is_empty());
        assert!(resolver.asked.lock().expect("lock").is_empty());
    }

    #[test]
    fn an_admitted_but_empty_pool_is_an_empty_success_without_an_embedding_call() {
        let received = Arc::new(Mutex::new(Vec::new()));
        let (service, repository, resolver) = service(
            FakeConfigurationRepository::configured(PROFILE, MODEL),
            FakeEmbedder::Recording(matching_vector(), received.clone()),
            Ok(Vec::new()),
            Ok(Vec::new()),
            Vec::new(),
        );

        let outcome = service
            .search(&sample_query("npm", 5), &authority(&[]), resolver.as_ref())
            .expect("search");

        assert_eq!(outcome.degraded, None);
        assert!(outcome.hits.is_empty());
        assert!(received.lock().expect("lock").is_empty());
        assert!(repository
            .received_authority
            .lock()
            .expect("lock")
            .is_empty());
    }

    #[test]
    fn query_embedding_failure_degrades_to_keyword_only_instead_of_erroring() {
        let (service, _repository, resolver) = service(
            FakeConfigurationRepository::configured(PROFILE, MODEL),
            FakeEmbedder::Fails,
            Ok(Vec::new()),
            Ok(vec!["m1".to_string()]),
            vec![record("m1", "uses npm")],
        );

        let outcome = service
            .search(
                &sample_query("npm", 5),
                &authority(&["m1"]),
                resolver.as_ref(),
            )
            .expect("search");

        assert_eq!(outcome.degraded, Some(Degradation::KeywordOnly));
        assert!(!outcome.hits.is_empty());
    }

    #[test]
    fn keyword_path_failure_degrades_to_vector_only_instead_of_erroring() {
        let (service, _repository, resolver) = healthy_service(
            Ok(vec![("m1".to_string(), matching_vector())]),
            Err(storage_failure()),
            vec![record("m1", "uses npm")],
        );

        let outcome = service
            .search(
                &sample_query("npm", 5),
                &authority(&["m1"]),
                resolver.as_ref(),
            )
            .expect("search");

        assert_eq!(outcome.degraded, Some(Degradation::VectorOnly));
        assert!(!outcome.hits.is_empty());
    }

    #[test]
    fn both_paths_available_but_empty_is_success_not_an_error() {
        let (service, _repository, resolver) =
            healthy_service(Ok(Vec::new()), Ok(Vec::new()), Vec::new());

        let outcome = service
            .search(
                &sample_query("npm", 5),
                &authority(&["m1"]),
                resolver.as_ref(),
            )
            .expect("search");

        assert_eq!(outcome.degraded, None);
        assert!(outcome.hits.is_empty());
    }

    #[test]
    fn both_paths_failing_reports_unavailable_rather_than_an_empty_result() {
        let (service, _repository, resolver) = service(
            FakeConfigurationRepository::configured(PROFILE, MODEL),
            FakeEmbedder::Fails,
            Ok(Vec::new()),
            Err(storage_failure()),
            Vec::new(),
        );

        assert_eq!(
            service
                .search(
                    &sample_query("npm", 5),
                    &authority(&["m1"]),
                    resolver.as_ref()
                )
                .unwrap_err(),
            RetrievalError::Unavailable
        );
    }

    #[test]
    fn results_are_truncated_to_the_requested_limit_after_stale_hits_are_dropped() {
        // m2 ranks second but its authoritative record is gone (revoked between ranking and
        // delivery). It must neither appear nor consume one of the two slots.
        let ids = ["m1", "m2", "m3", "m4", "m5"];
        let records = ["m1", "m3", "m4", "m5"]
            .iter()
            .map(|&id| record(id, "content"))
            .collect();
        let keyword_hits = ids.iter().map(|&id| id.to_string()).collect();
        let (service, _repository, resolver) =
            healthy_service(Ok(Vec::new()), Ok(keyword_hits), records);

        let outcome = service
            .search(&sample_query("npm", 2), &authority(&ids), resolver.as_ref())
            .expect("search");

        assert_eq!(
            outcome
                .hits
                .iter()
                .map(|hit| hit.source_id.as_str())
                .collect::<Vec<_>>(),
            vec!["m1", "m3"]
        );
    }

    #[test]
    fn an_over_long_query_is_truncated_before_it_reaches_either_path() {
        let received = Arc::new(Mutex::new(Vec::new()));
        let query_text = "x".repeat(EMBEDDING_CONTENT_LIMIT + 500);
        let (service, repository, resolver) = service(
            FakeConfigurationRepository::configured(PROFILE, MODEL),
            FakeEmbedder::Recording(matching_vector(), received.clone()),
            Ok(Vec::new()),
            Ok(Vec::new()),
            Vec::new(),
        );

        service
            .search(
                &sample_query(&query_text, 5),
                &authority(&["m1"]),
                resolver.as_ref(),
            )
            .expect("search");

        let embedded = received.lock().expect("lock");
        assert_eq!(embedded.len(), 1);
        assert_eq!(embedded[0].chars().count(), EMBEDDING_CONTENT_LIMIT);
        assert_eq!(
            repository
                .received_keyword_query
                .lock()
                .expect("lock")
                .as_deref()
                .map(|query| query.chars().count()),
            Some(EMBEDDING_CONTENT_LIMIT + 2)
        );
    }

    #[test]
    fn the_resolver_is_asked_exactly_once_for_the_fused_candidate_ids() {
        let (service, _repository, resolver) = healthy_service(
            Ok(vec![("m1".to_string(), matching_vector())]),
            Ok(vec!["m2".to_string()]),
            vec![record("m1", "uses npm"), record("m2", "uses cargo")],
        );

        service
            .search(
                &sample_query("npm", 5),
                &authority(&["m1", "m2"]),
                resolver.as_ref(),
            )
            .expect("search");

        let asked = resolver.asked.lock().expect("lock");
        assert_eq!(asked.len(), 1);
        let mut ids = asked[0].clone();
        ids.sort();
        assert_eq!(ids, vec!["m1".to_string(), "m2".to_string()]);
    }

    #[test]
    fn an_unconfigured_service_reports_not_configured() {
        let query = sample_query("npm", 5);
        let (absent, _repository, resolver) = service(
            FakeConfigurationRepository::unconfigured(),
            FakeEmbedder::Succeeds(matching_vector()),
            Ok(Vec::new()),
            Ok(Vec::new()),
            Vec::new(),
        );
        assert_eq!(
            absent
                .search(&query, &authority(&["m1"]), resolver.as_ref())
                .unwrap_err(),
            RetrievalError::NotConfigured
        );

        let (empty_profile, _repository, resolver) = service(
            FakeConfigurationRepository::configured("", MODEL),
            FakeEmbedder::Succeeds(matching_vector()),
            Ok(Vec::new()),
            Ok(Vec::new()),
            Vec::new(),
        );
        assert_eq!(
            empty_profile
                .search(&query, &authority(&["m1"]), resolver.as_ref())
                .unwrap_err(),
            RetrievalError::NotConfigured
        );
    }

    #[test]
    fn the_query_is_escaped_before_reaching_fts() {
        let (service, repository, resolver) =
            healthy_service(Ok(Vec::new()), Ok(Vec::new()), Vec::new());

        service
            .search(
                &sample_query("a OR b", 5),
                &authority(&["m1"]),
                resolver.as_ref(),
            )
            .expect("search");

        assert_eq!(
            *repository.received_keyword_query.lock().expect("lock"),
            Some("\"a OR b\"".to_string())
        );
    }
}
