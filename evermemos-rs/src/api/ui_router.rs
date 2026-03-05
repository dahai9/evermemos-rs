use axum::Router;
#[cfg(feature = "behavior-history")]
use axum::{response::Html, routing::get};

#[cfg(feature = "behavior-history")]
static BEHAVIOR_HISTORY_HTML: &str =
    include_str!("../../static/behavior_history.html");

#[cfg(feature = "behavior-history")]
async fn behavior_history_ui() -> Html<&'static str> {
    Html(BEHAVIOR_HISTORY_HTML)
}

pub fn ui_routes() -> Router {
    let router = Router::new();

    #[cfg(feature = "behavior-history")]
    let router = router.route("/ui/behavior-history", get(behavior_history_ui));

    router
}
