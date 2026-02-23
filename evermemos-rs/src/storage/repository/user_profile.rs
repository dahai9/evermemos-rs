use anyhow::{Context, Result};
use surrealdb::engine::any::Any;
use surrealdb::Surreal;
use uuid::Uuid;

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
        Ok(results.into_iter().next())
    }

    pub async fn upsert(&self, profile: UserProfile) -> Result<UserProfile> {
        let user_id = profile.user_id.clone();
        // Try update first, then insert
        let mut resp = self
            .db
            .query(
                "SELECT * FROM user_profile WHERE user_id = $uid AND is_deleted = false LIMIT 1",
            )
            .bind(("uid", user_id.clone()))
            .await
            .context("UserProfile upsert lookup failed")?;

        let existing: Vec<UserProfile> = resp.take(0)?;
        if let Some(existing) = existing.into_iter().next() {
            let id = existing
                .id
                .as_ref()
                .map(|t| t.id.to_raw())
                .unwrap_or_default();
            let updated: Option<UserProfile> = self
                .db
                .update(("user_profile", id))
                .merge(profile)
                .await
                .context("UserProfile update failed")?;
            updated.context("UserProfile update returned no record")
        } else {
            let rid = Uuid::new_v4().to_string();
            let created: Option<UserProfile> = self
                .db
                .create(("user_profile", rid))
                .content(profile)
                .await
                .context("UserProfile insert failed")?;
            created.context("UserProfile insert returned no record")
        }
    }
}
