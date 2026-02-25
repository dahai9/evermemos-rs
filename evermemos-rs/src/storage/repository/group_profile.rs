use anyhow::{Context, Result};
use chrono::Utc;
use surrealdb::engine::any::Any;
use surrealdb::Surreal;
use uuid::Uuid;

use crate::storage::models::GroupProfile;

#[derive(Clone)]
pub struct GroupProfileRepo {
    db: Surreal<Any>,
}

impl GroupProfileRepo {
    pub fn new(db: Surreal<Any>) -> Self {
        Self { db }
    }

    /// Fetch the current profile for a group (None if not yet created).
    pub async fn get_by_group_id(&self, group_id: &str) -> Result<Option<GroupProfile>> {
        let mut resp = self
            .db
            .query("SELECT * FROM group_profile WHERE group_id = $gid AND is_deleted = false LIMIT 1")
            .bind(("gid", group_id.to_string()))
            .await
            .context("GroupProfileRepo::get_by_group_id failed")?;
        let results: Vec<GroupProfile> = resp.take(0)?;
        Ok(results.into_iter().next())
    }

    /// Upsert the group profile — insert if new, update if existing.
    pub async fn upsert(&self, mut profile: GroupProfile) -> Result<GroupProfile> {
        let group_id = profile.group_id.clone();
        profile.updated_at = Some(Utc::now());

        let mut resp = self
            .db
            .query("SELECT * FROM group_profile WHERE group_id = $gid AND is_deleted = false LIMIT 1")
            .bind(("gid", group_id.clone()))
            .await
            .context("GroupProfile upsert lookup failed")?;

        let existing: Vec<GroupProfile> = resp.take(0)?;
        if let Some(existing) = existing.into_iter().next() {
            let id = existing
                .id
                .as_ref()
                .map(|t| t.id.to_raw())
                .unwrap_or_default();
            let updated: Option<GroupProfile> = self
                .db
                .update(("group_profile", id))
                .merge(profile)
                .await
                .context("GroupProfile update failed")?;
            updated.context("GroupProfile update returned no record")
        } else {
            let rid = Uuid::new_v4().to_string();
            profile.created_at = Some(Utc::now());
            let created: Option<GroupProfile> = self
                .db
                .create(("group_profile", rid))
                .content(profile)
                .await
                .context("GroupProfile insert failed")?;
            created.context("GroupProfile insert returned no record")
        }
    }
}
