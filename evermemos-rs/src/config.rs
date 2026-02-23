use serde::Deserialize;

/// Top-level application configuration, loaded from environment variables
/// and optional config.toml. All fields map 1-to-1 with .env.template.
#[derive(Debug, Clone, Deserialize)]
pub struct AppConfig {
    pub llm: LlmConfig,
    pub vectorize: VectorizeConfig,
    pub rerank: RerankConfig,
    pub surreal: SurrealConfig,
    pub nats: NatsConfig,
    pub server: ServerConfig,
    pub tenant: TenantConfig,
    pub api_key: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LlmConfig {
    pub base_url: String,
    pub api_key: String,
    pub model: String,
    pub temperature: f32,
    pub max_tokens: u32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct VectorizeConfig {
    pub base_url: String,
    pub api_key: String,
    pub model: String,
    pub dimensions: usize,
    pub batch_size: usize,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RerankConfig {
    pub base_url: String,
    pub api_key: String,
    pub model: String,
    /// Whether reranking is available (requires base_url + model to be set)
    pub enabled: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SurrealConfig {
    /// e.g. "rocksdb://./data/surreal" | "ws://localhost:8000" | "mem://"
    pub endpoint: String,
    pub ns: String,
    pub db: String,
    pub user: String,
    pub pass: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct NatsConfig {
    pub url: String,
    pub stream: String,
    pub subject_memorize: String,
    /// Whether NATS is enabled (set to false to skip NATS connection)
    pub enabled: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TenantConfig {
    pub org_header: String,
    pub space_header: String,
}

impl AppConfig {
    /// Load configuration from environment variables (highest priority)
    /// and an optional `config.toml` file (lower priority).
    pub fn load() -> anyhow::Result<Self> {
        // Load .env if present — ignore error if file doesn't exist
        let _ = dotenvy::dotenv();

        let cfg = config::Config::builder()
            // Defaults
            .set_default("llm.base_url", "https://api.openai.com/v1")?
            .set_default("llm.model", "gpt-4o-mini")?
            .set_default("llm.temperature", 0.3)?
            .set_default("llm.max_tokens", 16384)?
            .set_default("vectorize.base_url", "https://api.openai.com/v1")?
            .set_default("vectorize.model", "text-embedding-3-small")?
            .set_default("vectorize.dimensions", 1024)?
            .set_default("vectorize.batch_size", 10)?
            .set_default("rerank.base_url", "")?
            .set_default("rerank.api_key", "")?
            .set_default("rerank.model", "")?
            .set_default("rerank.enabled", false)?
            .set_default("surreal.endpoint", "rocksdb://./data/surreal")?
            .set_default("surreal.ns", "evermem")?
            .set_default("surreal.db", "main")?
            .set_default("surreal.user", "root")?
            .set_default("surreal.pass", "root")?
            .set_default("nats.url", "nats://127.0.0.1:4222")?
            .set_default("nats.stream", "EVERMEM")?
            .set_default("nats.subject_memorize", "evermem.memorize")?
            .set_default("nats.enabled", false)?
            .set_default("server.host", "0.0.0.0")?
            .set_default("server.port", 8080)?
            .set_default("tenant.org_header", "X-Organization-Id")?
            .set_default("tenant.space_header", "X-Space-Id")?
            .set_default("api_key", "none")?
            // Optional config.toml file
            .add_source(
                config::File::with_name("config")
                    .format(config::FileFormat::Toml)
                    .required(false),
            )
            // Environment variables override everything (separated by __)
            // e.g. LLM__BASE_URL maps to llm.base_url
            // e.g. NATS__SUBJECT_MEMORIZE maps to nats.subject_memorize
            .add_source(
                config::Environment::default()
                    .separator("__")
                    .try_parsing(true)
                    .ignore_empty(true),
            )
            .build()?;

        Ok(cfg.try_deserialize()?)
    }
}
