use anyhow::{Context, Result};
use surrealdb::engine::any::Any;
use surrealdb::Surreal;
use tracing::debug;
use uuid::Uuid;

use super::{DateRange, SearchResult};
use crate::storage::models::EpisodicMemory;

#[derive(Clone)]
pub struct EpisodicMemoryRepo {
    db: Surreal<Any>,
}

impl EpisodicMemoryRepo {
    pub fn new(db: Surreal<Any>) -> Self {
        Self { db }
    }

    /// Insert a new EpisodicMemory, returning the created record.
    pub async fn insert(&self, mut mem: EpisodicMemory) -> Result<EpisodicMemory> {
        // Pre-compute search_content
        mem.search_content = Some(EpisodicMemory::compute_search_content(
            mem.subject.as_deref(),
            &mem.episode,
        ));

        let rid = Uuid::new_v4().to_string();
        let created: Option<EpisodicMemory> = self
            .db
            .create(("episodic_memory", rid.clone()))
            .content(mem)
            .await
            .context("Failed to insert episodic_memory")?;

        let record = created.context("Insert returned no record")?;

        // Graph Relation: user -> experienced -> episodic_memory
        if let Some(user_id) = &record.user_id {
            // Ensure user node exists
            let _ = self.db.query("UPSERT type::thing('user', $user_id) SET user_id = $user_id")
                .bind(("user_id", user_id.clone()))
                .await;

            // RELATE user -> experienced -> episodic_memory
            let _ = self.db.query("RELATE type::thing('user', $user_id) -> experienced -> type::thing('episodic_memory', $rid) SET timestamp = $ts")
                .bind(("user_id", user_id.clone()))
                .bind(("rid", rid.clone()))
                .bind(("ts", record.timestamp.clone()))
                .await;
        }

        // Graph Relation: memcell -> produced -> episodic_memory
        if let Some(memcell_ids) = &record.memcell_ids {
            for cell_id in memcell_ids {
                let _ = self.db.query("RELATE type::thing('memcell', $cell_id) -> produced -> type::thing('episodic_memory', $rid)")
                    .bind(("cell_id", cell_id.clone()))
                    .bind(("rid", rid.clone()))
                    .await;
            }
        }

        Ok(record)
    }

    /// Update an existing EpisodicMemory by its SurrealDB record ID.
    pub async fn update(&self, id: &str, mem: EpisodicMemory) -> Result<EpisodicMemory> {
        let updated: Option<EpisodicMemory> = self
            .db
            .update(("episodic_memory", id))
            .merge(mem)
            .await
            .context("Failed to update episodic_memory")?;

        updated.context("Update returned no record")
    }

    /// Soft-delete by SurrealDB record ID.
    pub async fn soft_delete(&self, id: &str) -> Result<()> {
        let id = id.to_string();
        self.db
            .query("UPDATE type::thing('episodic_memory', $id) SET is_deleted = true, updated_at = time::now()")
            .bind(("id", id))
            .await
            .context("Failed to soft-delete episodic_memory")?;
        Ok(())
    }

    /// Soft-delete all records matching user_id or group_id.
    pub async fn soft_delete_by_filter(
        &self,
        user_id: Option<&str>,
        group_id: Option<&str>,
    ) -> Result<u64> {
        let mut conditions = vec!["is_deleted = false"];
        let mut bindings: Vec<(&str, String)> = vec![];

        if let Some(uid) = user_id {
            conditions.push("user_id = $user_id");
            bindings.push(("user_id", uid.to_string()));
        }
        if let Some(gid) = group_id {
            conditions.push("group_id = $group_id");
            bindings.push(("group_id", gid.to_string()));
        }

        let where_clause = conditions.join(" AND ");
        let sql = format!(
            "UPDATE episodic_memory SET is_deleted = true, updated_at = time::now() WHERE {where_clause} RETURN BEFORE"
        );

        let mut q = self.db.query(sql);
        for (k, v) in bindings {
            q = q.bind((k, v));
        }
        let mut resp = q.await.context("soft_delete_by_filter failed")?;
        let deleted: Vec<EpisodicMemory> = resp.take(0)?;
        Ok(deleted.len() as u64)
    }

    /// Paginated list retrieval (GET /api/v1/memories).
    pub async fn list(
        &self,
        user_id: Option<&str>,
        group_id: Option<&str>,
        date_range: &DateRange,
        limit: u32,
        offset: u32,
    ) -> Result<Vec<EpisodicMemory>> {
        let mut sql = String::from("SELECT * FROM episodic_memory WHERE is_deleted = false");

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

        sql.push_str(" ORDER BY timestamp DESC LIMIT $limit START $offset");

        let mut q = self
            .db
            .query(sql)
            .bind(("limit", limit))
            .bind(("offset", offset));

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

        let mut resp = q.await.context("EpisodicMemoryRepo::list failed")?;
        Ok(resp.take(0)?)
    }

    /// BM25 full-text keyword search.
    /// `tokens` should be space-separated (pre-tokenised by jieba-rs for zh).
    pub async fn search_bm25(
        &self,
        tokens: &str,
        user_id: Option<&str>,
        group_id: Option<&str>,
        limit: u32,
        date_range: &DateRange,
    ) -> Result<Vec<SearchResult<EpisodicMemory>>> {
        debug!("EpisodicMemoryRepo::search_bm25 tokens={tokens}");

        let mut sql = String::from(
            "SELECT *, search::score(0) AS _score \
             FROM episodic_memory \
             WHERE search_content @0@ $tokens \
             AND is_deleted = false",
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

        let mut resp = q
            .await
            .map_err(|e| anyhow::anyhow!("BM25 search failed: {e:#}"))?;

        // Deserialize directly — avoids #[serde(flatten)] + Option<Thing> incompatibility.
        // Score is not extracted from SQL; rank position serves as the relevance proxy.
        let records: Vec<EpisodicMemory> = resp.take(0)?;
        Ok(records
            .into_iter()
            .enumerate()
            .map(|(i, item)| SearchResult {
                score: 1.0 / (1.0 + i as f32),
                item,
            })
            .collect())
    }

    /// HNSW ANN vector search (COSINE, 1024-dim).
    pub async fn search_vector(
        &self,
        query_vec: &[f32],
        user_id: Option<&str>,
        group_id: Option<&str>,
        limit: u32,
        radius: Option<f32>,
        date_range: &DateRange,
    ) -> Result<Vec<SearchResult<EpisodicMemory>>> {
        debug!("EpisodicMemoryRepo::search_vector limit={limit}");

        let mut sql = String::from(
            "SELECT *, vector::similarity::cosine(vector, $vec) AS _score \
             FROM episodic_memory \
             WHERE is_deleted = false AND vector IS NOT NONE",
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
        if radius.is_some() {
            sql.push_str(" AND vector::similarity::cosine(vector, $vec) >= $radius");
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
        if let Some(r) = radius {
            q = q.bind(("radius", r));
        }
        if let Some(s) = &date_range.start {
            q = q.bind(("start", *s));
        }
        if let Some(e) = &date_range.end {
            q = q.bind(("end", *e));
        }

        let mut resp = q.await.context("Vector search failed")?;

        let records: Vec<EpisodicMemory> = resp.take(0)?;
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
