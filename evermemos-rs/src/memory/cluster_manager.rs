use anyhow::Result;
use chrono::Utc;
use uuid::Uuid;

use crate::storage::models::ClusterState;
use crate::storage::repository::ClusterStateRepo;

/// Configuration for MemCell clustering.
#[derive(Debug, Clone)]
pub struct ClusterConfig {
    /// Cosine similarity threshold to consider two MemCells part of the same cluster.
    pub similarity_threshold: f32,
    /// Maximum time gap (in days) between MemCells to allow them in the same cluster.
    pub max_time_gap_days: f64,
}

impl Default for ClusterConfig {
    fn default() -> Self {
        Self {
            similarity_threshold: 0.7,
            max_time_gap_days: 7.0,
        }
    }
}

/// Manages MemCell clustering using cosine + temporal similarity.
/// Mirrors Python `ClusterManager`.
pub struct ClusterManager {
    repo: ClusterStateRepo,
    config: ClusterConfig,
}

impl ClusterManager {
    pub fn new(repo: ClusterStateRepo, config: ClusterConfig) -> Self {
        Self { repo, config }
    }

    /// Attempt to add a MemCell vector to an existing cluster or create a new one.
    /// Returns the cluster_id if a cluster was completed.
    pub async fn process_vector(
        &self,
        user_id: Option<&str>,
        group_id: Option<&str>,
        memcell_id: &str,
        vector: &[f32],
        timestamp: chrono::DateTime<chrono::Utc>,
    ) -> Result<Option<String>> {
        // Load existing clusters for this user/group
        let clusters = self.repo.list_by_user(user_id, group_id).await?;

        // Find the best matching cluster
        let mut best_cluster: Option<(String, f32)> = None;
        for cluster in &clusters {
            if let Some(centroid) = &cluster.centroid {
                let sim = cosine_similarity(vector, centroid);
                // Check temporal constraint
                let time_ok = cluster
                    .last_updated
                    .map(|lu| {
                        let days = (timestamp - lu).num_seconds().abs() as f64 / 86400.0;
                        days <= self.config.max_time_gap_days
                    })
                    .unwrap_or(true);

                if sim >= self.config.similarity_threshold
                    && time_ok
                    && best_cluster.as_ref().map_or(true, |(_, s)| sim > *s)
                {
                    best_cluster = Some((cluster.cluster_id.clone(), sim));
                }
            }
        }

        if let Some((cluster_id, _)) = best_cluster {
            // Add to existing cluster
            if let Ok(Some(mut cluster)) = self.repo.get_by_cluster_id(&cluster_id).await {
                cluster.memcell_ids.push(memcell_id.to_string());
                cluster.centroid = Some(update_centroid(
                    cluster.centroid.as_deref().unwrap_or(&[]),
                    vector,
                    cluster.memcell_ids.len(),
                ));
                cluster.last_updated = Some(Utc::now());
                self.repo.upsert(cluster).await?;
                return Ok(Some(cluster_id));
            }
        }

        // Create new cluster
        let cluster_id = Uuid::new_v4().to_string();
        let new_cluster = ClusterState {
            id: None,
            user_id: user_id.map(String::from),
            group_id: group_id.map(String::from),
            cluster_id: cluster_id.clone(),
            memcell_ids: vec![memcell_id.to_string()],
            centroid: Some(vector.to_vec()),
            last_updated: Some(Utc::now()),
            created_at: Some(Utc::now()),
        };
        self.repo.upsert(new_cluster).await?;

        // A brand-new single-item cluster doesn't trigger profile extraction
        Ok(None)
    }
}

/// Cosine similarity between two equal-length vectors.
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }
    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm_a == 0.0 || norm_b == 0.0 {
        return 0.0;
    }
    (dot / (norm_a * norm_b)).clamp(-1.0, 1.0)
}

/// Incremental centroid update (running average).
pub(crate) fn update_centroid(old_centroid: &[f32], new_vec: &[f32], n: usize) -> Vec<f32> {
    if old_centroid.is_empty() {
        return new_vec.to_vec();
    }
    let n = n as f32;
    old_centroid
        .iter()
        .zip(new_vec.iter())
        .map(|(o, v)| (o * (n - 1.0) + v) / n)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cosine_identical_vectors() {
        let v = vec![1.0_f32, 0.0, 0.0];
        assert!((cosine_similarity(&v, &v) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn cosine_orthogonal_vectors() {
        let a = vec![1.0_f32, 0.0];
        let b = vec![0.0_f32, 1.0];
        assert!(cosine_similarity(&a, &b).abs() < 1e-6);
    }

    #[test]
    fn cosine_mismatched_lengths() {
        assert_eq!(cosine_similarity(&[1.0_f32], &[1.0, 2.0]), 0.0);
    }

    #[test]
    fn cosine_zero_vector() {
        assert_eq!(cosine_similarity(&[0.0_f32, 0.0], &[1.0, 0.0]), 0.0);
    }

    #[test]
    fn update_centroid_first_item() {
        let result = update_centroid(&[], &[1.0, 2.0, 3.0], 1);
        assert_eq!(result, vec![1.0, 2.0, 3.0]);
    }

    #[test]
    fn update_centroid_running_average() {
        let old = vec![2.0_f32, 4.0];
        // n=2: new_centroid[i] = (old[i] * 1 + new[i]) / 2
        let result = update_centroid(&old, &[4.0, 8.0], 2);
        assert!((result[0] - 3.0).abs() < 1e-6);
        assert!((result[1] - 6.0).abs() < 1e-6);
    }
}
