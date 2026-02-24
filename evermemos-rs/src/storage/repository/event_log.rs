use anyhow::{Context, Result};
use surrealdb::engine::any::Any;
use surrealdb::Surreal;
use uuid::Uuid;

use super::{DateRange, SearchResult};
use crate::storage::models::EventLogRecord;

#[derive(Clone)]
pub struct EventLogRepo {
    db: Surreal<Any>,
}

impl EventLogRepo {
    pub fn new(db: Surreal<Any>) -> Self {
        Self { db }
    }

    pub async fn insert(&self, mut rec: EventLogRecord) -> Result<EventLogRecord> {
        rec.search_content = Some(rec.atomic_fact.clone());
        let rid = Uuid::new_v4().to_string();
        let created: Option<EventLogRecord> = self
            .db
            .create(("event_log_record", rid))
            .content(rec)
            .await
            .context("Failed to insert event_log_record")?;
        created.context("Insert returned no record")
    }

    pub async fn soft_delete(&self, id: &str) -> Result<()> {
        let id = id.to_string();
        self.db
            .query("UPDATE type::thing('event_log_record', $id) SET is_deleted = true, updated_at = time::now()")
            .bind(("id", id))
            .await
            .context("Failed to soft-delete event_log_record")?;
        Ok(())
    }

    pub async fn search_bm25(
        &self,
        tokens: &str,
        user_id: Option<&str>,
        group_id: Option<&str>,
        limit: u32,
        date_range: &DateRange,
    ) -> Result<Vec<SearchResult<EventLogRecord>>> {
        let mut sql = String::from(
            "SELECT *, search::score(0) AS _score FROM event_log_record \
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

        let mut resp = q.await.context("EventLog BM25 failed")?;

        let records: Vec<EventLogRecord> = resp.take(0)?;
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
    ) -> Result<Vec<SearchResult<EventLogRecord>>> {
        let mut sql = String::from(
            "SELECT *, vector::similarity::cosine(vector, $vec) AS _score \
             FROM event_log_record WHERE is_deleted = false AND vector IS NOT NONE",
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

        let mut resp = q.await.context("EventLog vector search failed")?;

        let records: Vec<EventLogRecord> = resp.take(0)?;
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
