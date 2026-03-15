use anyhow::{Context, Result};
use surrealdb::engine::any::Any;
use surrealdb::Surreal;
use uuid::Uuid;

use super::{DateRange, SearchResult};
use crate::storage::models::ForesightRecord;

#[derive(Clone)]
pub struct ForesightRepo {
    db: Surreal<Any>,
}

impl ForesightRepo {
    pub fn new(db: Surreal<Any>) -> Self {
        Self { db }
    }

    pub async fn insert(&self, mut rec: ForesightRecord) -> Result<ForesightRecord> {
        rec.search_content = Some(rec.foresight.clone());
        let rid = Uuid::new_v4().to_string();
        let created: Option<ForesightRecord> = self
            .db
            .create(("foresight_record", rid.clone()))
            .content(rec)
            .await
            .context("Failed to insert foresight_record")?;
        
        let record = created.context("Insert returned no record")?;

        // Graph Relation: user -> experienced -> foresight_record
        if let Some(user_id) = &record.user_id {
            // Ensure user node exists
            let _ = self.db.query("UPSERT type::thing('user', $user_id) SET user_id = $user_id")
                .bind(("user_id", user_id.clone()))
                .await;

            // RELATE user -> experienced -> foresight_record
            let _ = self.db.query("RELATE type::thing('user', $user_id) -> experienced -> type::thing('foresight_record', $rid) SET timestamp = $ts")
                .bind(("user_id", user_id.clone()))
                .bind(("rid", rid))
                .bind(("ts", record.timestamp.clone()))
                .await;
        }

        Ok(record)
    }

    pub async fn soft_delete(&self, id: &str) -> Result<()> {
        let id = id.to_string();
        self.db
            .query("UPDATE type::thing('foresight_record', $id) SET is_deleted = true, updated_at = time::now()")
            .bind(("id", id))
            .await
            .context("Failed to soft-delete foresight_record")?;
        Ok(())
    }

    pub async fn search_bm25(
        &self,
        tokens: &str,
        user_id: Option<&str>,
        group_id: Option<&str>,
        limit: u32,
        date_range: &DateRange,
    ) -> Result<Vec<SearchResult<ForesightRecord>>> {
        let mut sql = String::from(
            "SELECT *, search::score(0) AS _score FROM foresight_record \
             WHERE search_content @0@ $tokens AND is_deleted = false",
        );
        if user_id.is_some() {
            sql.push_str(" AND user_id = $user_id");
        }
        if group_id.is_some() {
            sql.push_str(" AND group_id = $group_id");
        }
        if date_range.start.is_some() {
            sql.push_str(" AND timestamp >= $start");
        }
        if date_range.end.is_some() {
            sql.push_str(" AND timestamp <= $end");
        }
        sql.push_str(" ORDER BY _score DESC LIMIT $limit");

        let mut q = self
            .db
            .query(sql)
            .bind(("tokens", tokens.to_string()))
            .bind(("limit", limit));
        if let Some(uid) = user_id {
            q = q.bind(("user_id", uid.to_string()));
        }
        if let Some(gid) = group_id {
            q = q.bind(("group_id", gid.to_string()));
        }
        if let Some(s) = &date_range.start {
            q = q.bind(("start", *s));
        }
        if let Some(e) = &date_range.end {
            q = q.bind(("end", *e));
        }

        let mut resp = q.await.context("Foresight BM25 failed")?;

        let records: Vec<ForesightRecord> = resp.take(0)?;
        Ok(records
            .into_iter()
            .enumerate()
            .map(|(i, item)| SearchResult {
                score: 1.0 / (1.0 + i as f32),
                item,
            })
            .collect())
    }

    pub async fn search_vector(
        &self,
        query_vec: &[f32],
        user_id: Option<&str>,
        group_id: Option<&str>,
        limit: u32,
        date_range: &DateRange,
    ) -> Result<Vec<SearchResult<ForesightRecord>>> {
        let mut sql = String::from(
            "SELECT *, vector::similarity::cosine(vector, $vec) AS _score \
             FROM foresight_record WHERE is_deleted = false AND vector IS NOT NONE",
        );
        if user_id.is_some() {
            sql.push_str(" AND user_id = $user_id");
        }
        if group_id.is_some() {
            sql.push_str(" AND group_id = $group_id");
        }
        if date_range.start.is_some() {
            sql.push_str(" AND timestamp >= $start");
        }
        if date_range.end.is_some() {
            sql.push_str(" AND timestamp <= $end");
        }
        sql.push_str(" ORDER BY _score DESC LIMIT $limit");

        let mut q = self
            .db
            .query(sql)
            .bind(("vec", query_vec.to_vec()))
            .bind(("limit", limit));
        if let Some(uid) = user_id {
            q = q.bind(("user_id", uid.to_string()));
        }
        if let Some(gid) = group_id {
            q = q.bind(("group_id", gid.to_string()));
        }
        if let Some(s) = &date_range.start {
            q = q.bind(("start", *s));
        }
        if let Some(e) = &date_range.end {
            q = q.bind(("end", *e));
        }

        let mut resp = q.await.context("Foresight vector search failed")?;

        let records: Vec<ForesightRecord> = resp.take(0)?;
        Ok(records
            .into_iter()
            .enumerate()
            .map(|(i, item)| SearchResult {
                score: 1.0 / (1.0 + i as f32),
                item,
            })
            .collect())
    }
}
