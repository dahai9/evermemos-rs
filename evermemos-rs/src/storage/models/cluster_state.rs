use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use surrealdb::sql::Thing;

/// Represents the clustered state of MemCells produced by the ClusterManager.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterState {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<Thing>,

    pub user_id: Option<String>,
    pub group_id: Option<String>,
    pub cluster_id: String,
    pub memcell_ids: Vec<String>,
    /// Centroid of the cluster (average of all MemCell vectors in the cluster)
    pub centroid: Option<Vec<f32>>,
    pub last_updated: Option<DateTime<Utc>>,
    pub created_at: Option<DateTime<Utc>>,
}
