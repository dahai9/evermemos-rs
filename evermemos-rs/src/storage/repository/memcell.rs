use anyhow::{Context, Result};
use surrealdb::engine::any::Any;
use surrealdb::Surreal;
use uuid::Uuid;

use crate::storage::models::MemCell;

#[derive(Clone)]
pub struct MemCellRepo {
    db: Surreal<Any>,
}

impl MemCellRepo {
    pub fn new(db: Surreal<Any>) -> Self {
        Self { db }
    }

    pub async fn insert(&self, cell: MemCell) -> Result<MemCell> {
        let rid = Uuid::new_v4().to_string();
        let created: Option<MemCell> = self
            .db
            .create(("memcell", rid))
            .content(cell)
            .await
            .map_err(|e| anyhow::anyhow!("SurrealDB memcell insert error: {e:#}"))?;
        created.context("Insert returned no record")
    }

    pub async fn soft_delete(&self, id: &str) -> Result<()> {
        let id = id.to_string();
        self.db
            .query("UPDATE type::thing('memcell', $id) SET is_deleted = true, updated_at = time::now()")
            .bind(("id", id))
            .await
            .context("Failed to soft-delete memcell")?;
        Ok(())
    }

    /// Fetch recent non-deleted MemCells for a user/group (used by ClusterManager).
    pub async fn list_recent(
        &self,
        user_id: Option<&str>,
        group_id: Option<&str>,
        limit: u32,
    ) -> Result<Vec<MemCell>> {
        let mut sql = String::from("SELECT * FROM memcell WHERE is_deleted = false");
        if user_id.is_some() {
            sql.push_str(" AND user_id = $user_id");
        }
        if group_id.is_some() {
            sql.push_str(" AND group_id = $group_id");
        }
        sql.push_str(" ORDER BY timestamp DESC LIMIT $limit");

        let mut q = self.db.query(sql).bind(("limit", limit));
        if let Some(uid) = user_id {
            q = q.bind(("user_id", uid.to_string()));
        }
        if let Some(gid) = group_id {
            q = q.bind(("group_id", gid.to_string()));
        }

        let mut resp = q.await.context("MemCellRepo::list_recent failed")?;
        Ok(resp.take(0)?)
    }
}
