/// 存储格式：f32 little-endian 连续字节。选它而不是 JSON 数组，是因为向量路要在候选集上做
/// 暴力扫描，反序列化开销直接进热路径。
pub(crate) fn encode_embedding(values: &[f32]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(values.len() * 4);
    for value in values {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    bytes
}

pub(crate) fn decode_embedding(bytes: &[u8]) -> Option<Vec<f32>> {
    if !bytes.len().is_multiple_of(4) {
        return None;
    }
    Some(
        bytes
            .as_chunks::<4>()
            .0
            .iter()
            .map(|chunk| f32::from_le_bytes(*chunk))
            .collect(),
    )
}

/// 维度不一致或任一侧为零向量时返回 `None`——没有有意义的相似度可言，交由调用方跳过该候选，
/// 而不是伪造一个 0.0 混进排名。
pub(crate) fn cosine_similarity(left: &[f32], right: &[f32]) -> Option<f32> {
    if left.len() != right.len() || left.is_empty() {
        return None;
    }
    let mut dot = 0.0_f64;
    let mut left_norm = 0.0_f64;
    let mut right_norm = 0.0_f64;
    for (a, b) in left.iter().zip(right.iter()) {
        dot += f64::from(*a) * f64::from(*b);
        left_norm += f64::from(*a) * f64::from(*a);
        right_norm += f64::from(*b) * f64::from(*b);
    }
    if left_norm == 0.0 || right_norm == 0.0 {
        return None;
    }
    Some((dot / (left_norm.sqrt() * right_norm.sqrt())) as f32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedding_round_trips_through_its_blob_form() {
        let original = vec![0.0_f32, 1.5, -2.25, 1e-7];
        let decoded = decode_embedding(&encode_embedding(&original)).expect("decodes");
        assert_eq!(decoded, original);
    }

    #[test]
    fn blob_with_a_length_that_is_not_a_multiple_of_four_is_rejected() {
        assert_eq!(decode_embedding(&[0, 0, 0]), None);
    }

    #[test]
    fn cosine_of_identical_vectors_is_one() {
        let similarity = cosine_similarity(&[1.0, 2.0, 3.0], &[1.0, 2.0, 3.0]).expect("similarity");
        assert!((similarity - 1.0).abs() < 1e-6, "got {similarity}");
    }

    #[test]
    fn cosine_of_orthogonal_vectors_is_zero() {
        let similarity = cosine_similarity(&[1.0, 0.0], &[0.0, 1.0]).expect("similarity");
        assert!(similarity.abs() < 1e-6, "got {similarity}");
    }

    #[test]
    fn cosine_of_opposite_vectors_is_negative_one() {
        let similarity = cosine_similarity(&[1.0, 2.0], &[-1.0, -2.0]).expect("similarity");
        assert!((similarity + 1.0).abs() < 1e-6, "got {similarity}");
    }

    #[test]
    fn cosine_rejects_dimension_mismatch_and_zero_vectors() {
        assert_eq!(cosine_similarity(&[1.0, 2.0], &[1.0]), None);
        assert_eq!(cosine_similarity(&[0.0, 0.0], &[1.0, 2.0]), None);
    }
}
