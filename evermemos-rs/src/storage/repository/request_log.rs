use anyhow::{Context, Result};
use surrealdb::engine::any::Any;
use surrealdb::Surreal;

use crate::storage::models::MemoryRequestLog;

#[derive(Clone)]
pub struct MemoryRequestLogRepo {
    db: Surreal<Any>,
}

impl MemoryRequestLogRepo {
    pub fn new(db: Surreal<Any>) -> Self {
        Self { db }
    }

    pub async fn get_by_message_id(&self, message_id: &str) -> Result<Option<MemoryRequestLog>> {
        let mut resp = self
            .db
            .query("SELECT * FROM memory_request_log WHERE message_id = $mid LIMIT 1")
            .bind(("mid", message_id.to_string()))
            .await
            .context("MemoryRequestLogRepo::get_by_message_id failed")?;
        let rows: Vec<MemoryRequestLog> = resp.take(0)?;
        Ok(rows.into_iter().next())
    }
}
