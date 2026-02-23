use std::collections::HashMap;

use jieba_rs::Jieba;
use once_cell::sync::Lazy;

/// Global jieba instance (initialised once for Chinese tokenisation).
static JIEBA: Lazy<Jieba> = Lazy::new(Jieba::new);

/// English stopwords (subset, for BM25 pre-filtering).
const EN_STOPWORDS: &[&str] = &[
    "a", "an", "the", "is", "are", "was", "were", "be", "been", "being",
    "have", "has", "had", "do", "does", "did", "will", "would", "could",
    "should", "may", "might", "must", "can", "to", "of", "in", "on", "at",
    "for", "with", "by", "from", "as", "into", "through", "and", "or", "but",
    "if", "that", "this", "these", "those", "i", "you", "he", "she", "we",
    "they", "it", "my", "your", "his", "her", "our", "their", "its",
];

/// Tokenise a query for BM25 search.
/// For Chinese text: uses jieba-rs word segmentation.
/// For English text: splits on whitespace and removes stopwords.
pub fn tokenise(query: &str) -> String {
    let has_chinese = query.chars().any(|c| (c as u32) > 0x2E80);
    if has_chinese {
        JIEBA
            .cut(query, false)
            .into_iter()
            .filter(|t| !t.trim().is_empty())
            .collect::<Vec<_>>()
            .join(" ")
    } else {
        query
            .split_whitespace()
            .map(str::to_lowercase)
            .filter(|w| !EN_STOPWORDS.contains(&w.as_str()))
            .collect::<Vec<_>>()
            .join(" ")
    }
}

/// Reciprocal Rank Fusion (RRF) — combines keyword and vector result lists.
///
/// Formula: `RRF_score(d) = Σ_i  1 / (k + rank_i(d))` across all ranked lists.
/// k=60 is the standard smoothing constant from the original RRF paper.
pub fn reciprocal_rank_fusion<T>(
    lists: Vec<Vec<(String, T)>>, // (id, item) — already ranked, best first
    k: f32,
) -> Vec<(String, f32)> {
    let mut scores: HashMap<String, f32> = HashMap::new();

    for list in lists {
        for (rank, (id, _)) in list.into_iter().enumerate() {
            let rrf = 1.0 / (k + rank as f32 + 1.0);
            *scores.entry(id).or_insert(0.0) += rrf;
        }
    }

    let mut result: Vec<(String, f32)> = scores.into_iter().collect();
    result.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokenise_english_removes_stopwords() {
        let out = tokenise("the quick brown fox");
        assert!(!out.contains("the"), "'the' should be removed");
        assert!(out.contains("quick"));
        assert!(out.contains("brown"));
        assert!(out.contains("fox"));
    }

    #[test]
    fn tokenise_english_lowercases() {
        let out = tokenise("Hello World");
        assert!(out.contains("hello"));
        assert!(out.contains("world"));
    }

    #[test]
    fn tokenise_chinese_segments() {
        // 我爱北京 → tokens separated by spaces
        let out = tokenise("我爱北京");
        assert!(!out.is_empty());
        // jieba should produce at least 2 tokens for this sentence
        let tokens: Vec<&str> = out.split_whitespace().collect();
        assert!(tokens.len() >= 2, "Expected jieba to split '我爱北京', got: {out}");
    }

    #[test]
    fn tokenise_empty_string() {
        assert_eq!(tokenise(""), "");
    }

    #[test]
    fn rrf_single_list() {
        let list = vec![
            ("a".to_string(), 0.9f32),
            ("b".to_string(), 0.8f32),
            ("c".to_string(), 0.7f32),
        ];
        let result = reciprocal_rank_fusion(vec![list], 60.0);
        // Order should be preserved: rank 0 > rank 1 > rank 2
        assert_eq!(result[0].0, "a");
        assert_eq!(result[1].0, "b");
        assert_eq!(result[2].0, "c");
    }

    #[test]
    fn rrf_merges_two_lists() {
        // doc "b" appears in both lists at high rank → should win
        let kw = vec![
            ("a".to_string(), 0.9f32),
            ("b".to_string(), 0.8f32),
        ];
        let vec = vec![
            ("b".to_string(), 0.95f32),
            ("c".to_string(), 0.7f32),
        ];
        let result = reciprocal_rank_fusion(vec![kw, vec], 60.0);
        // "b" appears in both → highest combined score
        assert_eq!(result[0].0, "b");
    }

    #[test]
    fn rrf_empty_lists() {
        let result: Vec<(String, f32)> = reciprocal_rank_fusion::<String>(vec![], 60.0);
        assert!(result.is_empty());
    }
}
