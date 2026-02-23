use anyhow::{Context, Result};
use surrealdb::engine::any::Any;
use surrealdb::Surreal;
use uuid::Uuid;

use crate::storage::models::ForesightRecord;
use super::{DateRange, SearchResult};

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
            .create(("foresight_record", rid))
            .content(rec)
            .await
            .context("Failed to insert foresight_record")?;
        created.context("Insert returned no record")
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
             WHERE search_content @0@ $tokens AND is_deleted = false"
             // search::score(0) — 0-indexed predicate,
        );
        if user_id.is_some()  { sql.push_str(" AND user_id = $user_id"); }
        if group_id.is_some() { sql.push_str(" AND group_id = $group_id"); }
        if date_range.start.is_some() { sql.push_str(" AND timestamp >= $start"); }
        if date_range.end.is_some()   { sql.push_str(" AND timestamp <= $end"); }
        sql.push_str(" ORDER BY _score DESC LIMIT $limit");

        let mut q = self.db.query(sql)
            .bind(("tokens", tokens.to_string()))
            .bind(("limit", limit));
        if let Some(uid) = user_id  { q = q.bind(("user_id", uid.to_string())); }
        if let Some(gid) = group_id { q = q.bind(("group_id", gid.to_string())); }
        if let Some(s) = &date_range.start { q = q.bind(("start", *s)); }
        if let Some(e) = &date_range.end   { q = q.bind(("end", *e)); }

        let mut resp = q.await.context("Foresight BM25 failed")?;

        #[derive(serde::Deserialize)]
        struct Row { #[serde(flatten)] rec: ForesightRecord, #[serde(rename="_score", default)] score: f32 }
        let rows: Vec<Row> = resp.take(0)?;
        Ok(rows.into_iter().map(|r| SearchResult { item: r.rec, score: r.score }).collect())
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
        if user_id.is_some()  { sql.push_str(" AND user_id = $user_id"); }
        if group_id.is_some() { sql.push_str(" AND group_id = $group_id"); }
        if date_range.start.is_some() { sql.push_str(" AND timestamp >= $start"); }
        if date_range.end.is_some()   { sql.push_str(" AND timestamp <= $end"); }
        sql.push_str(" ORDER BY _score DESC LIMIT $limit");

        let mut q = self.db.query(sql).bind(("vec", query_vec.to_vec())).bind(("limit", limit));
        if let Some(uid) = user_id  { q = q.bind(("user_id", uid.to_string())); }
        if let Some(gid) = group_id { q = q.bind(("group_id", gid.to_string())); }
        if let Some(s) = &date_range.start { q = q.bind(("start", *s)); }
        if let Some(e) = &date_range.end   { q = q.bind(("end", *e)); }

        let mut resp = q.await.context("Foresight vector search failed")?;

        #[derive(serde::Deserialize)]
        struct Row { #[serde(flatten)] rec: ForesightRecord, #[serde(rename="_score", default)] score: f32 }
        let rows: Vec<Row> = resp.take(0)?;
        Ok(rows.into_iter().map(|r| SearchResult { item: r.rec, score: r.score }).collect())
    }
}
