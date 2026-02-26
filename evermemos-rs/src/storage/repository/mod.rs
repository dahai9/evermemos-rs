pub mod behavior_history;
pub mod cluster_state;
pub mod conversation_meta;
pub mod episodic_memory;
pub mod event_log;
pub mod foresight;
pub mod group_profile;
pub mod memcell;
pub mod request_log;
pub mod user_profile;

pub use behavior_history::BehaviorHistoryRepo;
pub use cluster_state::ClusterStateRepo;
pub use conversation_meta::ConversationMetaRepo;
pub use episodic_memory::EpisodicMemoryRepo;
pub use event_log::EventLogRepo;
pub use foresight::ForesightRepo;
pub use group_profile::GroupProfileRepo;
pub use memcell::MemCellRepo;
pub use request_log::MemoryRequestLogRepo;
pub use user_profile::UserProfileRepo;

use chrono::{DateTime, Utc};

/// Common time-range filter used across repositories.
#[derive(Debug, Clone, Default)]
pub struct DateRange {
    pub start: Option<DateTime<Utc>>,
    pub end: Option<DateTime<Utc>>,
}

/// Result item from a full-text or vector search.
#[derive(Debug, Clone)]
pub struct SearchResult<T> {
    pub item: T,
    pub score: f32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn date_range_default_is_unbounded() {
        let dr = DateRange::default();
        assert!(dr.start.is_none());
        assert!(dr.end.is_none());
    }

    #[test]
    fn date_range_with_bounds() {
        use chrono::TimeZone;
        let start = chrono::Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap();
        let end = chrono::Utc.with_ymd_and_hms(2024, 12, 31, 0, 0, 0).unwrap();
        let dr = DateRange {
            start: Some(start),
            end: Some(end),
        };
        assert!(dr.start.unwrap() < dr.end.unwrap());
    }

    #[test]
    fn search_result_stores_score() {
        let r = SearchResult {
            item: "hello",
            score: 0.85,
        };
        assert!((r.score - 0.85).abs() < 1e-6);
        assert_eq!(r.item, "hello");
    }
}
