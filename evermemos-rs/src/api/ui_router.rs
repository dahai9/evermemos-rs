use axum::{response::Html, routing::get, Router};

static BEHAVIOR_HISTORY_HTML: &str =
    include_str!("../../static/behavior_history.html");

async fn behavior_history_ui() -> Html<&'static str> {
    Html(BEHAVIOR_HISTORY_HTML)
}

pub fn ui_routes() -> Router {
    Router::new().route("/ui/behavior-history", get(behavior_history_ui))
}
