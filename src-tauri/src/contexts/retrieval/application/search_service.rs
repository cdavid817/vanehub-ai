use crate::contexts::retrieval::domain::{
    cosine_similarity, escape_fts_query, fuse_with_rrf, Degradation, MatchedVia, RetrievalError,
    RetrievalQuery, ScoredHit, SourceKind,
};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use super::indexing_service::{truncate_for_embedding, IndexSourcePort, IndexSourceRecord};
use super::ports::{EmbeddingPort, RetrievalConfigurationRepository, RetrievalDocumentRepository};

/// `degraded` 为 `None` 时 `hits` 为空表示"搜了，确实没有"；`degraded` 为 `Some` 时 `hits`
/// 仍可能非空，表示"某一路搜不了，用另一路的结果兜底"——两者是不同的语义，不能互相替代。
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct SearchOutcome {
    pub(crate) hits: Vec<ScoredHit>,
    pub(crate) degraded: Option<Degradation>,
}

/// 双路（向量 + 关键词）混合检索，第 1 期只服务 `SourceKind::AgentMemory`。
pub(crate) struct SearchService {
    configuration: Arc<dyn RetrievalConfigurationRepository>,
    repository: Arc<dyn RetrievalDocumentRepository>,
    source: Arc<dyn IndexSourcePort>,
    embeddings: Arc<dyn EmbeddingPort>,
}

impl SearchService {
    pub(crate) fn new(
        configuration: Arc<dyn RetrievalConfigurationRepository>,
        repository: Arc<dyn RetrievalDocumentRepository>,
        source: Arc<dyn IndexSourcePort>,
        embeddings: Arc<dyn EmbeddingPort>,
    ) -> Self {
        Self {
            configuration,
            repository,
            source,
            embeddings,
        }
    }

    /// 双路召回 → RRF 融合 → 回查源表。
    ///
    /// 铁律：检索失败**永不**让生成失败（设计文档 §8.1）。除"未配置"外，任何一路的失败都只是
    /// 降级，因为把一个可选增强能力的故障冒泡成生成失败是不可接受的。
    pub(crate) fn search(&self, query: &RetrievalQuery) -> Result<SearchOutcome, RetrievalError> {
        let configuration = self.configuration.load()?;
        let Some((_profile, model)) = configuration.resolved_model() else {
            return Err(RetrievalError::NotConfigured);
        };
        let over_fetch = query.limit.saturating_mul(4).max(query.limit);

        // query 是模型自撰的，长度不受任何约束。用与索引侧相同的上限截断：超长 query 会让
        // embedding 调用直接失败（对用户表现为无声的 `keyword_only` 降级），同时把几千 token
        // 的短语塞进 FTS。
        let text = truncate_for_embedding(&query.text);

        let vector_ranking = self.vector_ranking(&text, query, model, over_fetch);
        let keyword_ranking = self
            .repository
            .keyword_candidates(
                &query.scope,
                SourceKind::AgentMemory,
                &escape_fts_query(&text),
                over_fetch,
            )
            .ok();

        // 两路都失败必须与"两路都可用但都没命中"区分开。复用已有的 Err 路径而不是新增一种
        // 降级值：Task 13 的 `execute_recall` 已有分支会把 Err 转成**成功的**工具结果
        // "检索暂时不可用"，所以 §8.1 的铁律仍然成立，且不必新增一套模型要理解的词汇。
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

        // 回查源表拿权威内容：索引行可能陈旧，源已删则跳过——这保证已删记忆永不外泄，
        // 也是显式撤销失败时的第一道兜底（§5.3）。
        //
        // 只回查融合后的候选 id，不取全量快照：本方法跑在生成的工具调用里，而源表只增不减。
        // 取的是**全部**候选而不是前 `limit` 条——下面的 `take(limit)` 在跳过"源已删"的条目
        // *之后*才截断，只回查前 `limit` 条会让一条已删记忆白占一个名额。候选数本身有界
        // （两路各至多 `over_fetch` 条 = 4 × limit）。
        let wanted: Vec<String> = fused
            .iter()
            .map(|(source_id, _score)| source_id.clone())
            .collect();
        let sources: HashMap<String, IndexSourceRecord> = self
            .source
            .fetch(&wanted)?
            .into_iter()
            .map(|record| (record.source_id.clone(), record))
            .collect();

        let hits = fused
            .into_iter()
            .filter_map(|(source_id, score)| {
                let record = sources.get(&source_id)?;
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

    /// `None` 表示这一路整体不可用（query embedding 失败或候选查询失败），交由调用方降级；
    /// 空 `Vec` 表示这一路可用但没有命中，是正常结果。
    fn vector_ranking(
        &self,
        text: &str,
        query: &RetrievalQuery,
        model: &str,
        limit: usize,
    ) -> Option<Vec<String>> {
        let embedded = self.embeddings.embed(model, &[text.to_string()]).ok()?;
        let query_vector = embedded.into_iter().next()?;
        let candidates = self
            .repository
            .vector_candidates(&query.scope, SourceKind::AgentMemory, model)
            .ok()?;
        let mut scored: Vec<(String, f32)> = candidates
            .into_iter()
            .filter_map(|(source_id, vector)| {
                cosine_similarity(&query_vector, &vector).map(|score| (source_id, score))
            })
            .collect();
        scored.sort_by(|left, right| {
            right
                .1
                .partial_cmp(&left.1)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| left.0.cmp(&right.0))
        });
        Some(scored.into_iter().take(limit).map(|(id, _)| id).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contexts::retrieval::application::indexing_service::EMBEDDING_CONTENT_LIMIT;
    use crate::contexts::retrieval::application::ports::{
        EmbeddingFailure, RetrievalConfiguration, RetrievalIndexStatus,
    };
    use crate::contexts::retrieval::domain::{FailureCategory, RetrievalDocument, RetrievalScope};
    use std::sync::Mutex;

    /// embed() 的可编排行为：多数测试只需要"这一路整体可用/不可用"。`Recording` 额外把收到的
    /// inputs 录进调用方持有的句柄，只有断言 query 截断的那条测试需要它。
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
    }

    /// `vector_candidates`/`keyword_candidates` 的返回值由测试摆放，且后者记录收到的 FTS
    /// 查询串；其余九个方法在本文件的测试里都不可达，走 unimplemented!()。
    struct FakeRepository {
        vector_candidates_result: Result<Vec<(String, Vec<f32>)>, RetrievalError>,
        keyword_candidates_result: Result<Vec<String>, RetrievalError>,
        received_keyword_query: Mutex<Option<String>>,
    }

    impl FakeRepository {
        fn new(
            vector_candidates_result: Result<Vec<(String, Vec<f32>)>, RetrievalError>,
            keyword_candidates_result: Result<Vec<String>, RetrievalError>,
        ) -> Self {
            Self {
                vector_candidates_result,
                keyword_candidates_result,
                received_keyword_query: Mutex::new(None),
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
            _scope: &RetrievalScope,
            _source_kind: SourceKind,
            _model: &str,
        ) -> Result<Vec<(String, Vec<f32>)>, RetrievalError> {
            self.vector_candidates_result.clone()
        }
        fn keyword_candidates(
            &self,
            _scope: &RetrievalScope,
            _source_kind: SourceKind,
            query: &str,
            _limit: usize,
        ) -> Result<Vec<String>, RetrievalError> {
            *self.received_keyword_query.lock().expect("lock") = Some(query.to_string());
            self.keyword_candidates_result.clone()
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
    }

    /// `snapshot()` 故意 `unimplemented!()`：检索路一旦退回全量快照，就会在生成的工具调用里
    /// 同步加载并克隆全部 Agent 的全部记忆（源表只增不减）。让它直接炸掉，而不是安静地退化。
    struct FakeSource {
        records: Vec<IndexSourceRecord>,
        fetched: Mutex<Vec<Vec<String>>>,
    }

    impl IndexSourcePort for FakeSource {
        fn snapshot(&self) -> Result<Vec<IndexSourceRecord>, RetrievalError> {
            unimplemented!("the search path must resolve records by id, never snapshot the source")
        }
        fn fetch(&self, source_ids: &[String]) -> Result<Vec<IndexSourceRecord>, RetrievalError> {
            self.fetched.lock().expect("lock").push(source_ids.to_vec());
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

    /// 查询向量与候选向量共用同一个值，保证需要"命中"的场景里余弦相似度恒为 1.0。
    fn matching_vector() -> Vec<f32> {
        vec![1.0, 0.0]
    }

    fn scope() -> RetrievalScope {
        RetrievalScope {
            agent_id: "a".to_string(),
            folder: String::new(),
        }
    }

    fn sample_query(text: &str, limit: usize) -> RetrievalQuery {
        RetrievalQuery {
            text: text.to_string(),
            scope: scope(),
            limit,
        }
    }

    fn record(source_id: &str, content: &str) -> IndexSourceRecord {
        IndexSourceRecord {
            source_id: source_id.to_string(),
            agent_id: "a".to_string(),
            folder: String::new(),
            content: content.to_string(),
            created_at: "2026-08-05T00:00:00Z".to_string(),
        }
    }

    fn storage_failure() -> RetrievalError {
        RetrievalError::Storage("boom".to_string())
    }

    /// 装配一个完全受控的服务：四个依赖都由调用方摆放。返回仓储与源的句柄以便断言它们收到的
    /// 调用参数。
    fn service(
        configuration: FakeConfigurationRepository,
        embedder: FakeEmbedder,
        vector_candidates: Result<Vec<(String, Vec<f32>)>, RetrievalError>,
        keyword_candidates: Result<Vec<String>, RetrievalError>,
        records: Vec<IndexSourceRecord>,
    ) -> (SearchService, Arc<FakeRepository>, Arc<FakeSource>) {
        let repository = Arc::new(FakeRepository::new(vector_candidates, keyword_candidates));
        let source = Arc::new(FakeSource {
            records,
            fetched: Mutex::new(Vec::new()),
        });
        let search_service = SearchService::new(
            Arc::new(configuration),
            repository.clone(),
            source.clone(),
            Arc::new(embedder),
        );
        (search_service, repository, source)
    }

    /// 两路都健康（已配置 + embedding 成功）的常见装配，测试只需要摆放候选与源记录。
    fn healthy_service(
        vector_candidates: Result<Vec<(String, Vec<f32>)>, RetrievalError>,
        keyword_candidates: Result<Vec<String>, RetrievalError>,
        records: Vec<IndexSourceRecord>,
    ) -> (SearchService, Arc<FakeRepository>, Arc<FakeSource>) {
        service(
            FakeConfigurationRepository::configured(PROFILE, MODEL),
            FakeEmbedder::Succeeds(matching_vector()),
            vector_candidates,
            keyword_candidates,
            records,
        )
    }

    #[test]
    fn both_paths_healthy_yields_no_degradation_and_marks_overlap_as_both() {
        let (service, _repository, _source) = healthy_service(
            Ok(vec![("m1".to_string(), matching_vector())]),
            Ok(vec!["m1".to_string()]),
            vec![record("m1", "uses npm not pnpm")],
        );

        let outcome = service.search(&sample_query("npm", 5)).expect("search");

        assert_eq!(outcome.degraded, None);
        assert_eq!(outcome.hits.len(), 1);
        assert_eq!(outcome.hits[0].source_id, "m1");
        assert_eq!(outcome.hits[0].content, "uses npm not pnpm");
        assert_eq!(outcome.hits[0].matched_via, MatchedVia::Both);
    }

    #[test]
    fn a_hit_found_only_by_the_vector_path_is_marked_vector() {
        let (service, _repository, _source) = healthy_service(
            Ok(vec![("m1".to_string(), matching_vector())]),
            Ok(Vec::new()),
            vec![record("m1", "uses npm")],
        );

        let outcome = service.search(&sample_query("npm", 5)).expect("search");

        // 关键词路是健康的、只是没命中——不是失败，所以不该出现降级。
        assert_eq!(outcome.degraded, None);
        assert_eq!(outcome.hits.len(), 1);
        assert_eq!(outcome.hits[0].matched_via, MatchedVia::Vector);
    }

    #[test]
    fn a_hit_found_only_by_the_keyword_path_is_marked_keyword() {
        let (service, _repository, _source) = healthy_service(
            Ok(Vec::new()),
            Ok(vec!["m1".to_string()]),
            vec![record("m1", "uses npm")],
        );

        let outcome = service.search(&sample_query("npm", 5)).expect("search");

        assert_eq!(outcome.degraded, None);
        assert_eq!(outcome.hits.len(), 1);
        assert_eq!(outcome.hits[0].matched_via, MatchedVia::Keyword);
    }

    #[test]
    fn query_embedding_failure_degrades_to_keyword_only_instead_of_erroring() {
        // fake embedding 返回 Err → outcome.degraded == Some(Degradation::KeywordOnly)，且 hits 非空
        let (service, _repository, _source) = service(
            FakeConfigurationRepository::configured(PROFILE, MODEL),
            FakeEmbedder::Fails,
            Ok(Vec::new()),
            Ok(vec!["m1".to_string()]),
            vec![record("m1", "uses npm")],
        );

        let outcome = service.search(&sample_query("npm", 5)).expect("search");

        assert_eq!(outcome.degraded, Some(Degradation::KeywordOnly));
        assert!(!outcome.hits.is_empty());
    }

    #[test]
    fn keyword_path_failure_degrades_to_vector_only_instead_of_erroring() {
        // fake 仓储的 keyword_candidates 返回 Err → degraded == Some(Degradation::VectorOnly)
        let (service, _repository, _source) = healthy_service(
            Ok(vec![("m1".to_string(), matching_vector())]),
            Err(storage_failure()),
            vec![record("m1", "uses npm")],
        );

        let outcome = service.search(&sample_query("npm", 5)).expect("search");

        assert_eq!(outcome.degraded, Some(Degradation::VectorOnly));
        assert!(!outcome.hits.is_empty());
    }

    #[test]
    fn both_paths_available_but_empty_is_success_not_an_error() {
        // 两路都可用、都没命中 → Ok，hits 为空，degraded 为 None
        let (service, _repository, _source) =
            healthy_service(Ok(Vec::new()), Ok(Vec::new()), Vec::new());

        let outcome = service.search(&sample_query("npm", 5)).expect("search");

        assert_eq!(outcome.degraded, None);
        assert!(outcome.hits.is_empty());
    }

    #[test]
    fn both_paths_failing_reports_unavailable_rather_than_an_empty_result() {
        // fake embedding 返回 Err 且 fake 仓储的 keyword_candidates 也返回 Err
        // → Err(RetrievalError::Unavailable)，**不是** Ok(空列表)。
        // 这一条与上一条成对存在：区分"搜不了"和"没有"正是它们的全部意义。
        let (service, _repository, _source) = service(
            FakeConfigurationRepository::configured(PROFILE, MODEL),
            FakeEmbedder::Fails,
            Ok(Vec::new()),
            Err(storage_failure()),
            Vec::new(),
        );
        let query = sample_query("npm", 5);

        assert_eq!(
            service.search(&query).unwrap_err(),
            RetrievalError::Unavailable
        );
    }

    #[test]
    fn results_are_truncated_to_the_requested_limit() {
        let ids = ["m1", "m2", "m3", "m4", "m5"];
        let records = ids.iter().map(|&id| record(id, "content")).collect();
        let keyword_hits = ids.iter().map(|&id| id.to_string()).collect();
        let (service, _repository, _source) =
            healthy_service(Ok(Vec::new()), Ok(keyword_hits), records);

        let outcome = service.search(&sample_query("npm", 2)).expect("search");

        assert_eq!(outcome.hits.len(), 2);
        assert_eq!(
            outcome
                .hits
                .iter()
                .map(|hit| hit.source_id.as_str())
                .collect::<Vec<_>>(),
            vec!["m1", "m2"]
        );
    }

    #[test]
    fn a_hit_whose_source_row_is_gone_is_skipped_rather_than_returned_stale() {
        // 候选里有 m1、m2，但源快照只有 m1 → 结果只含 m1
        let (service, _repository, _source) = healthy_service(
            Ok(Vec::new()),
            Ok(vec!["m1".to_string(), "m2".to_string()]),
            vec![record("m1", "still here")],
        );

        let outcome = service.search(&sample_query("npm", 5)).expect("search");

        assert_eq!(outcome.hits.len(), 1);
        assert_eq!(outcome.hits[0].source_id, "m1");
    }

    #[test]
    fn an_over_long_query_is_truncated_before_it_reaches_either_path() {
        // query 完全由模型自撰，长度没有任何约束，而索引侧在 embedding 前按
        // `EMBEDDING_CONTENT_LIMIT` 截断。检索侧不截断的话，超长 query 会让 embedding 调用
        // 直接失败——对用户表现为一次无声的 `keyword_only` 降级——同时把几千 token 的短语
        // 交给 FTS。
        let received = Arc::new(Mutex::new(Vec::new()));
        let query_text = "x".repeat(EMBEDDING_CONTENT_LIMIT + 500);
        let (service, repository, _source) = service(
            FakeConfigurationRepository::configured(PROFILE, MODEL),
            FakeEmbedder::Recording(matching_vector(), received.clone()),
            Ok(Vec::new()),
            Ok(Vec::new()),
            Vec::new(),
        );

        service
            .search(&sample_query(&query_text, 5))
            .expect("search");

        let embedded = received.lock().expect("lock");
        assert_eq!(embedded.len(), 1);
        assert_eq!(
            embedded[0].chars().count(),
            EMBEDDING_CONTENT_LIMIT,
            "the embedding call must receive the same bounded text the indexing side sends"
        );
        // `escape_fts_query` 把整串裹进一对引号，所以 FTS 侧比上限多 2 个字符。
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
    fn the_source_is_resolved_by_candidate_id_rather_than_by_a_whole_table_snapshot() {
        // 回归对象：检索路复用 `snapshot()`。那会在生成的工具调用里同步加载并克隆**全部**
        // Agent 的**全部**记忆（`agent_memories` 只增不减），只为解析至多 limit 条 id。
        // `FakeSource::snapshot` 是 `unimplemented!()`，所以退回全量快照会直接 panic；这里
        // 进一步钉死"只问了候选 id"这件事。
        let (service, _repository, source) = healthy_service(
            Ok(vec![("m1".to_string(), matching_vector())]),
            Ok(vec!["m2".to_string()]),
            vec![record("m1", "uses npm"), record("m2", "uses cargo")],
        );

        service.search(&sample_query("npm", 5)).expect("search");

        let fetched = source.fetched.lock().expect("lock");
        assert_eq!(
            fetched.len(),
            1,
            "the source must be consulted exactly once"
        );
        let mut asked = fetched[0].clone();
        asked.sort();
        assert_eq!(asked, vec!["m1".to_string(), "m2".to_string()]);
    }

    #[test]
    fn an_unconfigured_service_reports_not_configured() {
        let query = sample_query("npm", 5);

        let (absent, _repository, _source) = service(
            FakeConfigurationRepository::unconfigured(),
            FakeEmbedder::Succeeds(matching_vector()),
            Ok(Vec::new()),
            Ok(Vec::new()),
            Vec::new(),
        );
        assert_eq!(
            absent.search(&query).unwrap_err(),
            RetrievalError::NotConfigured
        );

        // resolved_model 要求两半都非空；空串 profile id 不该被当作"已配置"——这个分支此前
        // 没有测试覆盖到，顺手在这里一并补上。
        let (empty_profile, _repository, _source) = service(
            FakeConfigurationRepository::configured("", MODEL),
            FakeEmbedder::Succeeds(matching_vector()),
            Ok(Vec::new()),
            Ok(Vec::new()),
            Vec::new(),
        );
        assert_eq!(
            empty_profile.search(&query).unwrap_err(),
            RetrievalError::NotConfigured
        );
    }

    #[test]
    fn the_query_is_escaped_before_reaching_fts() {
        // query 文本 `a OR b` → fake 仓储收到的 query 参数是 "\"a OR b\""
        let (service, repository, _source) =
            healthy_service(Ok(Vec::new()), Ok(Vec::new()), Vec::new());

        service.search(&sample_query("a OR b", 5)).expect("search");

        assert_eq!(
            *repository.received_keyword_query.lock().expect("lock"),
            Some("\"a OR b\"".to_string())
        );
    }
}
