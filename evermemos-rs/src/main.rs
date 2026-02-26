use std::sync::Arc;
use tracing::info;

use evermemos_rs::agentic::manager::AgenticManager;
use evermemos_rs::api::{
    behavior_history_routes, global_profile_router::GlobalProfileState, global_profile_routes,
    health_routes, memory_router::AppState, memory_routes, ui_routes, BehaviorHistoryState,
};
use evermemos_rs::biz::memorize::MemorizeService;
use evermemos_rs::config::AppConfig;
use evermemos_rs::core::{cache::Caches, tenant::TenantContext, tracing as app_tracing};
use evermemos_rs::llm::{
    apply_cassette, openai::OpenAiProvider, rerank::OpenAiReranker, vectorize::OpenAiVectorizer,
};
use evermemos_rs::memory::{
    manager::MemoryManager, memcell_extractor::MemCellExtractor, prompts::Locale,
};
use evermemos_rs::storage::{
    db,
    repository::{
        BehaviorHistoryRepo, ClusterStateRepo, ConversationMetaRepo, EpisodicMemoryRepo,
        EventLogRepo, ForesightRepo, GroupProfileRepo, MemCellRepo, MemoryRequestLogRepo,
        UserProfileRepo,
    },
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // ── 1. Tracing ────────────────────────────────────────────────────────────
    app_tracing::init();

    // ── 2. Config ─────────────────────────────────────────────────────────────
    let cfg = AppConfig::load()?;
    info!(
        "Config loaded. Server will listen on {}:{}",
        cfg.server.host, cfg.server.port
    );

    // ── 3. SurrealDB (embedded RocksDB) ──────────────────────────────────────
    let db = db::init(&cfg.surreal).await?;
    info!("SurrealDB connected at {}", cfg.surreal.endpoint);

    // ── 4. Repositories ───────────────────────────────────────────────────────
    let ep_repo = EpisodicMemoryRepo::new(db.clone());
    let fs_repo = ForesightRepo::new(db.clone());
    let el_repo = EventLogRepo::new(db.clone());
    let mc_repo = MemCellRepo::new(db.clone());
    let up_repo = UserProfileRepo::new(db.clone());
    let gp_repo = GroupProfileRepo::new(db.clone());
    let _cs_repo = ClusterStateRepo::new(db.clone());
    let cm_repo = ConversationMetaRepo::new(db.clone());
    let req_log_repo = MemoryRequestLogRepo::new(db.clone());
    let bh_repo = BehaviorHistoryRepo::new(db.clone());

    // ── 5. Caches ─────────────────────────────────────────────────────────────
    let caches = Caches::new();

    // ── 6. LLM / Vectorize / Rerank ───────────────────────────────────────────
    let llm_provider: Arc<dyn evermemos_rs::llm::provider::LlmProvider> =
        Arc::new(OpenAiProvider::new(&cfg.llm));

    let vectorizer: Arc<dyn evermemos_rs::llm::vectorize::VectorizeService> =
        Arc::new(OpenAiVectorizer::new(&cfg.vectorize, caches.clone()));

    let reranker: Arc<dyn evermemos_rs::llm::rerank::RerankService> =
        Arc::from(OpenAiReranker::build(&cfg.rerank));

    // ── 6b. Cassette (record/replay) — wraps providers if LLM_CASSETTE_MODE != off ──
    let (llm_provider, vectorizer, reranker) = apply_cassette(llm_provider, vectorizer, reranker);

    // ── 7. Memory extraction layer ────────────────────────────────────────────
    let locale = Locale::default(); // can be made config-driven later

    let boundary_detector = MemCellExtractor::new(Arc::clone(&llm_provider), locale);

    let memory_manager = Arc::new(MemoryManager::new(
        Arc::clone(&llm_provider),
        Arc::clone(&vectorizer),
        ep_repo.clone(),
        fs_repo.clone(),
        el_repo.clone(),
        up_repo.clone(),
        gp_repo.clone(),
        locale,
        cfg.vectorize.model.clone(),
    ));

    // ── 8. Business / Agentic services ────────────────────────────────────────
    let memorize_svc = Arc::new(MemorizeService::new(
        boundary_detector,
        Arc::clone(&memory_manager),
        mc_repo,
    ));

    let agentic = Arc::new(AgenticManager::new(
        Arc::clone(&llm_provider),
        Arc::clone(&vectorizer),
        Arc::clone(&reranker),
        ep_repo.clone(),
        fs_repo.clone(),
        el_repo.clone(),
        up_repo.clone(),
        bh_repo.clone(),
    ));

    // ── 9. Optional NATS worker (fire-and-forget) ─────────────────────────────
    if cfg.nats.enabled {
        let worker = evermemos_rs::tasks::nats_worker::NatsWorker::new(
            Arc::clone(&memorize_svc),
            cfg.nats.clone(),
        );
        tokio::spawn(async move {
            if let Err(e) = worker.start().await {
                tracing::error!("NATS worker terminated: {e}");
            }
        });
        info!(
            "NATS worker started (subject={})",
            cfg.nats.subject_memorize
        );
    }

    // ── 10. Axum router ───────────────────────────────────────────────────────
    let state = AppState {
        memorize_svc,
        agentic,
        ep_repo,
        conv_meta_repo: cm_repo,
        req_log_repo,
    };

    let global_profile_state = GlobalProfileState {
        up_repo: up_repo.clone(),
    };

    let bh_state = BehaviorHistoryState {
        bh_repo: bh_repo.clone(),
    };

    // Capture config values to use inside middleware closures
    let api_key = cfg.api_key.clone();
    let org_header = "X-Organization-Id";
    let space_header = "X-Space-Id";

    let app = memory_routes(state)
        .merge(health_routes())
        .merge(global_profile_routes(global_profile_state))
        .merge(behavior_history_routes(bh_state))
        .merge(ui_routes())
        // Tenant middleware — extract org/space from headers, inject TenantContext extension
        .layer(axum::middleware::from_fn({
            let org = org_header;
            let space = space_header;
            move |mut req: axum::extract::Request, next: axum::middleware::Next| async move {
                let org_id = req
                    .headers()
                    .get(org)
                    .and_then(|v| v.to_str().ok())
                    .unwrap_or("default")
                    .to_string();
                let space_id = req
                    .headers()
                    .get(space)
                    .and_then(|v| v.to_str().ok())
                    .unwrap_or("")
                    .to_string();
                req.extensions_mut()
                    .insert(TenantContext::new(org_id, space_id));
                next.run(req).await
            }
        }))
        // Auth middleware — check Bearer token
        .layer(axum::middleware::from_fn({
            let key = api_key;
            move |req: axum::extract::Request, next: axum::middleware::Next| {
                let key = key.clone();
                async move {
                    if key == "none" || key.is_empty() {
                        return Ok(next.run(req).await);
                    }
                    let token = req
                        .headers()
                        .get(axum::http::header::AUTHORIZATION)
                        .and_then(|v| v.to_str().ok())
                        .and_then(|v| v.strip_prefix("Bearer ").map(str::to_string));
                    match token {
                        Some(t) if t == key => Ok(next.run(req).await),
                        _ => Err(axum::http::StatusCode::UNAUTHORIZED),
                    }
                }
            }
        }))
        // Tower-http request tracing
        .layer(
            tower_http::trace::TraceLayer::new_for_http()
                .make_span_with(
                    tower_http::trace::DefaultMakeSpan::new().level(tracing::Level::INFO),
                )
                .on_response(
                    tower_http::trace::DefaultOnResponse::new().level(tracing::Level::INFO),
                ),
        );

    // ── 11. Start server ──────────────────────────────────────────────────────
    let addr = format!("{}:{}", cfg.server.host, cfg.server.port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    info!("evermemos-rs listening on http://{addr}");

    axum::serve(listener, app).await?;
    Ok(())
}
