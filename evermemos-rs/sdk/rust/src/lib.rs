mod adapters;
mod client;
mod error;
mod types;

pub use adapters::{
    build_langchain_messages, build_llamaindex_chat_history, build_openai_messages,
    compose_system_prompt, MemoryContextBuilder,
};
pub use client::{EverMemOSClient, EverMemOSClientBuilder, EverMemOSConfig};
pub use error::EverMemOSError;
pub use types::{
    ApiEnvelope, DeleteOptions, FetchMemoriesResult, FetchOptions, GenericMapResult,
    MemorizePayload, MemoryItem, RoleContentMessage, SearchOptions, SearchResult,
};
