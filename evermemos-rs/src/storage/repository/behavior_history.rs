use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use surrealdb::engine::any::Any;
use surrealdb::Surreal;
use tracing::debug;
use uuid::Uuid;

use crate::storage::models::BehaviorHistory;

#[derive(Clone)]
pub struct BehaviorHistoryRepo {
    db: Surreal<Any>,
}

impl BehaviorHistoryRepo {
    pub fn new(db: Surreal<Any>) -> Self {
        Self { db }
    }

    /// Insert a new BehaviorHistory record.
    pub async fn insert(&self, record: BehaviorHistory) -> Result<BehaviorHistory> {
        let rid = Uuid::new_v4().to_string();
        let created: Option<BehaviorHistory> = self
            .db
            .create(("behavior_history", rid))
            .content(record)
            .await
            .context("Failed to insert behavior_history")?;
        created.context("Insert returned no record")
    }

    /// Soft-delete by SurrealDB record ID.
    pub async fn soft_delete(&self, id: &str) -> Result<()> {
        let id = id.to_string();
        self.db
            .query(
                "UPDATE type::thing('behavior_history', $id) \
                 SET is_deleted = true, updated_at = time::now()",
            )
            .bind(("id", id))
            .await
            .context("Failed to soft-delete behavior_history")?;
        Ok(())
    }

    /// Get all behavior records for a user, sorted newest-first.
    pub async fn get_by_user_id(
        &self,
        user_id: &str,
        limit: u32,
    ) -> Result<Vec<BehaviorHistory>> {
        debug!("BehaviorHistoryRepo::get_by_user_id user={user_id}");
        let mut resp = self
            .db
            .query(
                "SELECT * FROM behavior_history \
                 WHERE user_id = $uid AND is_deleted = false \
                 ORDER BY timestamp DESC LIMIT $limit",
            )
            .bind(("uid", user_id.to_string()))
            .bind(("limit", limit))
            .await
            .context("get_by_user_id failed")?;
        let records: Vec<BehaviorHistory> = resp.take(0)?;
        Ok(records)
    }

    /// Get records for a user within a time range.
    pub async fn get_by_time_range(
        &self,
        user_id: Option<&str>,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
        limit: u32,
    ) -> Result<Vec<BehaviorHistory>> {
        let sql = if user_id.is_some() {
            "SELECT * FROM behavior_history \
             WHERE user_id = $uid AND timestamp >= $start AND timestamp <= $end \
             AND is_deleted = false ORDER BY timestamp DESC LIMIT $limit"
        } else {
            "SELECT * FROM behavior_history \
             WHERE timestamp >= $start AND timestamp <= $end \
             AND is_deleted = false ORDER BY timestamp DESC LIMIT $limit"
        };

        let mut q = self
            .db
            .query(sql)
            .bind(("start", start))
            .bind(("end", end))
            .bind(("limit", limit));

        if let Some(uid) = user_id {
            q = q.bind(("uid", uid.to_string()));
        }

        let mut resp = q.await.context("get_by_time_range failed")?;
        let records: Vec<BehaviorHistory> = resp.take(0)?;
        Ok(records)
    }

    /// Get records filtered by behavior_type tag.
    pub async fn get_by_type(
        &self,
        user_id: &str,
        behavior_type: &str,
        limit: u32,
    ) -> Result<Vec<BehaviorHistory>> {
        let mut resp = self
            .db
            .query(
                "SELECT * FROM behavior_history \
                 WHERE user_id = $uid AND $btype INSIDE behavior_type \
                 AND is_deleted = false ORDER BY timestamp DESC LIMIT $limit",
            )
            .bind(("uid", user_id.to_string()))
            .bind(("btype", behavior_type.to_string()))
            .bind(("limit", limit))
            .await
            .context("get_by_type failed")?;
        let records: Vec<BehaviorHistory> = resp.take(0)?;
        Ok(records)
    }

    /// Soft-delete all records for a user.
    pub async fn soft_delete_by_user(&self, user_id: &str) -> Result<u64> {
        self.db
            .query(
                "UPDATE behavior_history SET is_deleted = true, updated_at = time::now() \
                 WHERE user_id = $uid AND is_deleted = false",
            )
            .bind(("uid", user_id.to_string()))
            .await
            .context("soft_delete_by_user failed")?;
        Ok(0)
    }

    /// Total count for a user (non-deleted).
    pub async fn count_by_user(&self, user_id: &str) -> Result<u64> {
        let mut resp = self
            .db
            .query(
                "SELECT count() as n FROM behavior_history \
                 WHERE user_id = $uid AND is_deleted = false GROUP ALL",
            )
            .bind(("uid", user_id.to_string()))
            .await
            .context("count_by_user failed")?;
        let rows: Vec<serde_json::Value> = resp.take(0)?;
        let n = rows
            .first()
            .and_then(|r| r["n"].as_u64())
            .unwrap_or(0);
        Ok(n)
    }
}
