use std::collections::HashMap;

/// RRF 的平滑常数。60 是原论文的取值，作用是压低头部名次的边际优势，让"两路都中游"
/// 胜过"一路头名、另一路缺席"——这正是混合检索想要的行为。
const RRF_SMOOTHING: f64 = 60.0;

/// 输入是若干条已排好序的 id 列表（每条代表一路召回），输出按融合分降序。
/// 同分时按 id 升序，保证同样输入永远给出同样顺序——否则测试与 UI 都会闪。
pub(crate) fn fuse_with_rrf(rankings: &[Vec<String>]) -> Vec<(String, f64)> {
    let mut scores: HashMap<&str, f64> = HashMap::new();
    for ranking in rankings {
        for (index, id) in ranking.iter().enumerate() {
            let rank = index as f64 + 1.0;
            *scores.entry(id.as_str()).or_insert(0.0) += 1.0 / (RRF_SMOOTHING + rank);
        }
    }
    let mut fused: Vec<(String, f64)> = scores
        .into_iter()
        .map(|(id, score)| (id.to_string(), score))
        .collect();
    fused.sort_by(|left, right| {
        right
            .1
            .partial_cmp(&left.1)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| left.0.cmp(&right.0))
    });
    fused
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ids(ranking: &[(String, f64)]) -> Vec<&str> {
        ranking.iter().map(|(id, _)| id.as_str()).collect()
    }

    #[test]
    fn a_document_ranked_first_by_both_paths_wins() {
        let vector = vec!["a".to_string(), "b".to_string(), "c".to_string()];
        let keyword = vec!["a".to_string(), "c".to_string()];
        assert_eq!(ids(&fuse_with_rrf(&[vector, keyword]))[0], "a");
    }

    #[test]
    fn a_document_found_by_both_paths_beats_one_found_only_higher_by_a_single_path() {
        // b: 两路各第 2 名 → 2/62；a: 只有向量路第 1 名 → 1/61。两次中游胜过一次头名。
        let vector = vec!["a".to_string(), "b".to_string()];
        let keyword = vec!["c".to_string(), "b".to_string()];
        assert_eq!(ids(&fuse_with_rrf(&[vector, keyword]))[0], "b");
    }

    #[test]
    fn an_empty_path_does_not_affect_the_other_path_order() {
        let vector = vec!["a".to_string(), "b".to_string()];
        assert_eq!(ids(&fuse_with_rrf(&[vector, Vec::new()])), vec!["a", "b"]);
    }

    #[test]
    fn fusing_nothing_yields_nothing() {
        assert!(fuse_with_rrf(&[Vec::new(), Vec::new()]).is_empty());
    }

    #[test]
    fn ties_are_broken_deterministically_by_id() {
        let first = vec!["b".to_string(), "a".to_string()];
        let second = vec!["a".to_string(), "b".to_string()];
        assert_eq!(ids(&fuse_with_rrf(&[first, second])), vec!["a", "b"]);
    }
}
