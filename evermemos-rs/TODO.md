# EverMemOS-RS TODO

This file tracks the future directions and improvements for the Rust implementation of EverMemOS.

## 🚀 Priority: Local-first & Privacy
- [ ] **Local Embedding Integration**: Integrate `candle` or `llama.cpp` for on-device vector generation.
- [ ] **Offline LLM Extraction**: Support local-inference models (via `ollama` or native rust bindings) for memory extraction on NixOS without external API calls.
- [ ] **Encrypted Storage**: Leverage SurrealDB's security features to ensure data-at-rest encryption for sensitive personal memories.

## 🧠 Memory "Metabolism" & Intelligence
- [ ] **Conflict Resolution Mechanism**: Identify and handle contradictory memories (e.g., changing preferences over time).
- [ ] **Dynamic Profile Updates**: Automatically evolve `user_profile` based on new conversational evidence.
- [ ] **Spatiotemporal Graphs**: Utilize SurrealDB's graph capabilities to link memories with "When" (Timeline) and "Where" (Location context).

## ❄️ NixOS & Infrastructure
- [ ] **NixOS Module**: Create a `services.evermemos` nix module for declarative deployment via `configuration.nix`.
- [ ] **Flake Support**: Provide a `flake.nix` for reproducible builds and development environments.
- [ ] **Simplified Deployment**: Optimize `docker-compose.otel.yaml` for a "one-click" developer experience.

## 🎨 Visualization & UX
- [ ] **Memory Nebula UI**: Build a Web-based Admin UI (using `Leptos` or `Dioxus`) to visualize memory clusters.
- [ ] **Manual Pruning**: Allow users to manually edit, delete, or merge memory fragments extracted by AI.
- [ ] **MCP Extension**: Expand MCP tools to support direct profile editing and advanced filtering.

## 🛠️ Internal Optimizations
- [ ] **Performance Benchmarking**: Stress test SurrealDB HNSW vs. BM25 hybrid search performance.
- [ ] **Advanced Tokenization**: Refine `jieba-rs` integration for mixed-language (CN/EN) technical contexts.
- [ ] **OTEL Observability**: Enhance tracing spans to include specific LLM latency and SurrealDB query costs.
