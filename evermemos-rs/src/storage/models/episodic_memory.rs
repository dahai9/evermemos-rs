use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use surrealdb::sql::Thing;

/// Episode narrative — the primary retrieval unit.
/// Stored in SurrealDB with BM25 (search_content) + HNSW (vector) indexes,
/// replacing the Python triple-write to MongoDB + Elasticsearch + Milvus.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EpisodicMemory {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<Thing>,

    pub user_id: Option<String>,
    pub user_name: Option<String>,
    pub group_id: Option<String>,
    pub group_name: Option<String>,
    pub timestamp: DateTime<Utc>,
    pub participants: Option<Vec<String>>,

    /// Concise one-line summary
    pub summary: String,
    /// Full narrative episode text (main retrieval field)
    pub episode: String,
    pub subject: Option<String>,
    pub keywords: Option<Vec<String>>,
    pub memcell_ids: Option<Vec<String>>,

    /// 1024-dimensional embedding vector (COSINE HNSW indexed)
    pub vector: Option<Vec<f32>>,
    pub vector_model: Option<String>,

    /// Pre-computed for BM25: `subject + " " + episode`
    pub search_content: Option<String>,

    #[serde(default)]
    pub is_deleted: bool,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
}

impl EpisodicMemory {
    /// Build search_content from subject + episode.
    pub fn compute_search_content(subject: Option<&str>, episode: &str) -> String {
        match subject {
            Some(s) if !s.is_empty() => format!("{s} {episode}"),
            _ => episode.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn search_content_with_subject() {
        let out = EpisodicMemory::compute_search_content(Some("Travel"), "User went to Paris");
        assert_eq!(out, "Travel User went to Paris");
    }

    #[test]
    fn search_content_without_subject() {
        let out = EpisodicMemory::compute_search_content(None, "User went to Paris");
        assert_eq!(out, "User went to Paris");
    }

    #[test]
    fn search_content_empty_subject() {
        let out = EpisodicMemory::compute_search_content(Some(""), "User went to Paris");
        assert_eq!(out, "User went to Paris");
    }

    #[test]
    fn search_content_both_empty() {
        let out = EpisodicMemory::compute_search_content(None, "");
        assert_eq!(out, "");
    }
}
