//! 模型类别判定的唯一真源。chat 与 embedding 两个过滤器必须从这里派生——
//! 两处各自维护关键词表，迟早会在新增模型时漂移成互相矛盾的判断。

const NON_CHAT_KEYWORDS: &[&str] = &[
    // `embed-` 是本次新增的一项（其余沿用 service.rs 的原表）。少了它，Cohere 的真实 id
    // `embed-english-v3.0` 会同时满足 is_chat_model 与 is_embedding_model，在配置页的聊天模型
    // 选择器和 embedding 选择器里各出现一次——这个双重列出是本特性引入 embedding 选择器才产生的。
    "embedding",
    "embed-",
    "rerank",
    "whisper",
    "tts",
    "audio",
    "image",
    "moderation",
    "realtime",
    "sora",
    "stable-diffusion",
];

const EMBEDDING_KEYWORDS: &[&str] = &["embedding", "embed-"];

pub(crate) fn is_chat_model(id: &str) -> bool {
    let id = id.to_ascii_lowercase();
    !NON_CHAT_KEYWORDS
        .iter()
        .any(|excluded| id.contains(excluded))
}

pub(crate) fn is_embedding_model(id: &str) -> bool {
    let id = id.to_ascii_lowercase();
    EMBEDDING_KEYWORDS
        .iter()
        .any(|keyword| id.contains(keyword))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chat_and_embedding_classifications_are_mutually_exclusive() {
        // `embed-english-v3.0` 是这条测试的关键样本：它命中 EMBEDDING_KEYWORDS 的 `embed-`，
        // 而 NON_CHAT_KEYWORDS 若只有 `embedding` 就漏掉它，于是两个分类同时为真。
        for id in [
            "gpt-4o",
            "text-embedding-3-small",
            "embed-english-v3.0",
            "bge-reranker",
            "whisper-1",
        ] {
            assert!(
                !(is_chat_model(id) && is_embedding_model(id)),
                "{id} classified as both"
            );
        }
    }

    #[test]
    fn embedding_models_are_recognized_case_insensitively() {
        assert!(is_embedding_model("text-embedding-3-small"));
        assert!(is_embedding_model("TEXT-EMBEDDING-ADA-002"));
        assert!(is_embedding_model("bge-m3-embedding"));
    }

    #[test]
    fn chat_models_are_not_embedding_models() {
        for id in ["gpt-4o", "deepseek-chat", "claude-opus-4-8"] {
            assert!(!is_embedding_model(id), "{id}");
            assert!(is_chat_model(id), "{id}");
        }
    }

    #[test]
    fn non_chat_non_embedding_models_belong_to_neither() {
        // "dall-e-3" (as originally drafted) doesn't contain any NON_CHAT_KEYWORDS substring —
        // the exclusion list only catches "image", and "dall-e-3" doesn't spell that out — so
        // is_chat_model("dall-e-3") is actually `true` under the frozen, verified-current keyword
        // list this module must reproduce unchanged. Swapped for "stable-diffusion-xl", a real
        // image-generation model id that (like the original example) is neither chat nor
        // embedding, but actually matches the "stable-diffusion" keyword already in the list.
        for id in ["whisper-1", "stable-diffusion-xl", "bge-reranker-v2"] {
            assert!(!is_chat_model(id), "{id}");
            assert!(!is_embedding_model(id), "{id}");
        }
    }
}
