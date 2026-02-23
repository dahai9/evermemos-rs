use anyhow::{Context, Result};
use surrealdb::engine::any::Any;
use surrealdb::Surreal;
use uuid::Uuid;

use crate::storage::models::ClusterState;

#[derive(Clone)]
pub struct ClusterStateRepo {
    db: Surreal<Any>,
}

impl ClusterStateRepo {
    pub fn new(db: Surreal<Any>) -> Self {
        Self { db }
    }

    pub async fn get_by_cluster_id(&self, cluster_id: &str) -> Result<Option<ClusterState>> {
        let mut resp = self
            .db
            .query("SELECT * FROM cluster_state WHERE cluster_id = $cid LIMIT 1")
            .bind(("cid", cluster_id.to_string()))
            .await
            .context("ClusterStateRepo::get_by_cluster_id failed")?;
        let results: Vec<ClusterState> = resp.take(0)?;
        Ok(results.into_iter().next())
    }

    pub async fn upsert(&self, state: ClusterState) -> Result<ClusterState> {
        let cluster_id = state.cluster_id.clone();
        let existing = self.get_by_cluster_id(&cluster_id).await?;

        if let Some(ex) = existing {
            let id = ex.id.as_ref().map(|t| t.id.to_raw()).unwrap_or_default();
            let updated: Option<ClusterState> = self
                .db
                .update(("cluster_state", id))
                .merge(state)
                .await
                .context("ClusterState update failed")?;
            updated.context("ClusterState update returned no record")
        } else {
            let rid = Uuid::new_v4().to_string();
            let created: Option<ClusterState> = self
                .db
                .create(("cluster_state", rid))
                .content(state)
                .await
                .context("ClusterState insert failed")?;
            created.context("ClusterState insert returned no record")
        }
    }

    /// List all cluster states for a user/group, ordered by last_updated desc.
    pub async fn list_by_user(
        &self,
        user_id: Option<&str>,
        group_id: Option<&str>,
    ) -> Result<Vec<ClusterState>> {
        let mut sql = String::from("SELECT * FROM cluster_state WHERE 1=1");
        if user_id.is_some()  { sql.push_str(" AND user_id = $uid"); }
        if group_id.is_some() { sql.push_str(" AND group_id = $gid"); }
        sql.push_str(" ORDER BY last_updated DESC");

        let mut q = self.db.query(sql);
        if let Some(uid) = user_id  { q = q.bind(("uid", uid.to_string())); }
        if let Some(gid) = group_id { q = q.bind(("gid", gid.to_string())); }

        let mut resp = q.await.context("ClusterStateRepo::list_by_user failed")?;
        Ok(resp.take(0)?)
    }
}
