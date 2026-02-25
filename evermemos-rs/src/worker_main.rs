/// Standalone NATS worker binary.
/// Run with: `evermemos-worker` (see [[bin]] in Cargo.toml).
/// Useful for deploying a dedicated consumer process separate from the HTTP server.

use std::sync::Arc;
use tracing::info;

use evermemos_rs::biz::memorize::MemorizeService;
use evermemos_rs::config::AppConfig;
use evermemos_rs::core::{cache::Caches, tracing as app_tracing};
use evermemos_rs::llm::{
    apply_cassette,
    openai::OpenAiProvider,
    rerank::OpenAiReranker,
    vectorize::OpenAiVectorizer,
};
use evermemos_rs::memory::{
    manager::MemoryManager,
    memcell_extractor::MemCellExtractor,
    prompts::Locale,
};
use evermemos_rs::storage::{
    db,
    repository::{
        ClusterStateRepo, ConversationMetaRepo, EpisodicMemoryRepo, EventLogRepo,
        ForesightRepo, GroupProfileRepo, MemCellRepo, UserProfileRepo,
    },
};
use evermemos_rs::tasks::nats_worker::NatsWorker;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    app_tracing::init();

    let cfg = AppConfig::load()?;
    info!("Worker starting, NATS={}", cfg.nats.url);

    if !cfg.nats.enabled {
        tracing::warn!("NATS is disabled in config. Enable it with NATS__ENABLED=true");
        return Ok(());
    }

    // SurrealDB
    let db = db::init(&cfg.surreal).await?;

    // Repositories
    let ep_repo  = EpisodicMemoryRepo::new(db.clone());
    let fs_repo  = ForesightRepo::new(db.clone());
    let el_repo  = EventLogRepo::new(db.clone());
    let mc_repo  = MemCellRepo::new(db.clone());
    let up_repo  = UserProfileRepo::new(db.clone());
    let gp_repo  = GroupProfileRepo::new(db.clone());
    let _cs_repo = ClusterStateRepo::new(db.clone());
    let _cm_repo = ConversationMetaRepo::new(db.clone());

    // Caches + LLM stack
    let caches = Caches::new();
    let llm: Arc<dyn evermemos_rs::llm::provider::LlmProvider> = Arc::new(OpenAiProvider::new(&cfg.llm));
    let vectorizer: Arc<dyn evermemos_rs::llm::vectorize::VectorizeService> =
        Arc::new(OpenAiVectorizer::new(&cfg.vectorize, caches));
    let reranker: Arc<dyn evermemos_rs::llm::rerank::RerankService> =
        Arc::from(OpenAiReranker::build(&cfg.rerank));

    let (llm, vectorizer, _reranker) = apply_cassette(llm, vectorizer, reranker);

    let locale = Locale::default();
    let boundary_detector = MemCellExtractor::new(Arc::clone(&llm), locale);

    let memory_manager = Arc::new(MemoryManager::new(
        Arc::clone(&llm),
        Arc::clone(&vectorizer),
        ep_repo.clone(),
        fs_repo.clone(),
        el_repo.clone(),
        up_repo.clone(),
        gp_repo.clone(),
        locale,
        cfg.vectorize.model.clone(),
    ));

    let memorize_svc = Arc::new(MemorizeService::new(
        boundary_detector,
        Arc::clone(&memory_manager),
        mc_repo,
    ));

    // Start the NATS worker (blocks until shutdown)
    let worker = NatsWorker::new(Arc::clone(&memorize_svc), cfg.nats);
    info!("evermemos-worker ready");
    worker.start().await?;

    Ok(())
}
