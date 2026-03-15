use anyhow::{Context, Result};
use chrono::Utc;
use surrealdb::engine::any::Any;
use surrealdb::Surreal;
use tracing::debug;

use crate::storage::models::UserProfile;

#[derive(Clone)]
pub struct UserProfileRepo {
    db: Surreal<Any>,
}

impl UserProfileRepo {
    pub fn new(db: Surreal<Any>) -> Self {
        Self { db }
    }

    pub async fn get_by_user_id(&self, user_id: &str) -> Result<Option<UserProfile>> {
        let mut resp = self
            .db
            .query("SELECT * FROM user_profile WHERE user_id = $uid AND is_deleted = false LIMIT 1")
            .bind(("uid", user_id.to_string()))
            .await
            .context("UserProfileRepo::get_by_user_id failed")?;
        
        let results: Vec<UserProfile> = resp.take(0)?;
        
        if let Some(ref p) = results.first() {
            debug!("RAW SURREALDB RECORD for {}: {:?}", user_id, p);
        }

        Ok(results.into_iter().next())
    }

    pub async fn upsert(&self, profile: UserProfile) -> Result<UserProfile> {
        let user_id = profile.user_id.clone();
        let mut resp = self
            .db
            .query("SELECT * FROM user_profile WHERE user_id = $uid AND is_deleted = false LIMIT 1")
            .bind(("uid", user_id.clone()))
            .await
            .context("UserProfile upsert lookup failed")?;

        let existing: Vec<UserProfile> = resp.take(0)?;
        
        // Use existing record ID or user_id directly
        let id_val = if let Some(existing_record) = existing.into_iter().next() {
            existing_record.id.as_ref().map(|t| t.id.to_raw()).unwrap_or(user_id.clone())
        } else {
            user_id.clone()
        };
        
        // Serialize JSON to string. If None, write {}. This is safe because we use MERGE.
        let profile_data_str = match &profile.profile_data {
            Some(pd) => serde_json::to_string(pd).unwrap_or_else(|_| "{}".to_string()),
            None => "{}".to_string(),
        };

        // UPSERT + MERGE perfectly solves the race condition!
        // We inject the JSON string to bypass SurrealDB binary serialization bugs on untagged enums.
        let sql = format!(
            "UPSERT type::thing('user_profile', $id) MERGE {{ user_id: $uid, profile_data: {}, life_summary: $life_summary, is_deleted: false, updated_at: time::now() }}",
            profile_data_str
        );
        
        debug!("DEBUG UPSERT SQL: {}", sql);

        let mut q = self.db.query(sql).bind(("id", id_val));
        q = q.bind(("uid", profile.user_id));
        q = q.bind(("life_summary", profile.life_summary));

        let mut updated_resp = q.await.context("UserProfile UPSERT failed")?;
        let mut updated_records: Vec<UserProfile> = updated_resp.take(0)?;
        
        let updated = updated_records.pop();
        updated.context("UserProfile UPSERT returned no record")
    }

    /// Merge `custom_profile_data` into the user's profile record (create if absent).
    pub async fn upsert_custom_profile(
        &self,
        user_id: &str,
        custom_profile_data: serde_json::Value,
    ) -> Result<UserProfile> {
        let existing = self.get_by_user_id(user_id).await?;
        
        let custom_data_str = serde_json::to_string(&custom_profile_data).unwrap_or_else(|_| "{}".to_string());

        let id_val = if let Some(ex) = existing {
            ex.id.as_ref().map(|t| t.id.to_raw()).unwrap_or(user_id.to_string())
        } else {
            user_id.to_string()
        };

        let sql = format!(
            "UPSERT type::thing('user_profile', $id) MERGE {{ user_id: $uid, custom_profile_data: {}, is_deleted: false, updated_at: time::now() }}",
            custom_data_str
        );

        let mut q = self.db.query(sql).bind(("id", id_val));
        q = q.bind(("uid", user_id.to_string()));
        
        let mut updated_resp = q.await.context("UserProfile custom_profile_data UPSERT failed")?;
        let mut updated_records: Vec<UserProfile> = updated_resp.take(0)?;
        
        let updated = updated_records.pop();
        updated.context("Update returned no record")
    }
}