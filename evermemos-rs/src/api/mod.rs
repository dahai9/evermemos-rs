#[cfg(feature = "behavior-history")]
pub mod behavior_history_router;
pub mod dto;
pub mod global_profile_router;
pub mod health_router;
pub mod memory_router;
pub mod middleware;
pub mod ui_router;

#[cfg(feature = "behavior-history")]
pub use behavior_history_router::{behavior_history_routes, BehaviorHistoryState};
pub use global_profile_router::{global_profile_routes, GlobalProfileState};
pub use health_router::health_routes;
pub use memory_router::memory_routes;
pub use ui_router::ui_routes;
