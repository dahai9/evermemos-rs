use anyhow::{Context, Result};
use surrealdb::engine::any::Any;
use surrealdb::Surreal;

use crate::storage::models::ConversationMeta;

#[derive(Clone)]
pub struct ConversationMetaRepo {
    db: Surreal<Any>,
}

impl ConversationMetaRepo {
    pub fn new(db: Surreal<Any>) -> Self {
        Self { db }
    }

    pub async fn get(&self, conv_id: &str) -> Result<Option<ConversationMeta>> {
        let mut resp = self
            .db
            .query("SELECT * FROM conversation_meta WHERE conv_id = $cid LIMIT 1")
            .bind(("cid", conv_id.to_string()))
            .await
            .context("ConversationMetaRepo::get failed")?;
        let rows: Vec<ConversationMeta> = resp.take(0)?;
        Ok(rows.into_iter().next())
    }

    pub async fn upsert(&self, meta: ConversationMeta) -> Result<ConversationMeta> {
        let conv_id = meta.conv_id.clone();
        let existing = self.get(&conv_id).await?;
        if let Some(ex) = existing {
            let id = ex.id.as_ref().map(|t| t.id.to_raw()).unwrap_or_default();
            let updated: Option<ConversationMeta> = self
                .db
                .update(("conversation_meta", id))
                .merge(meta)
                .await
                .context("ConversationMeta update failed")?;
            updated.context("Update returned no record")
        } else {
            let created: Option<ConversationMeta> = self
                .db
                .create("conversation_meta")
                .content(meta)
                .await
                .context("ConversationMeta insert failed")?;
            created.context("Insert returned no record")
        }
    }
}
