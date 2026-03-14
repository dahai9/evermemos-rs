# evermemos-agent-lib (Rust)

A lightweight Rust SDK for connecting agent frameworks to an `evermemos-rs` server.

## Install

Add to your `Cargo.toml`:

```toml
[dependencies]
evermemos-agent-lib = { path = "/absolute/path/to/evermemos-rs/sdk/rust" }
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
```

## Quick Start

```rust
use evermemos_agent_lib::{EverMemOSClient, MemoryContextBuilder};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = EverMemOSClient::builder()
        .base_url("http://localhost:8080")
        .org_id("my-org")
        .user_id("alice")
        .group_id("my_chat")
        .build()?;

    client
        .memorize(
            evermemos_agent_lib::MemorizePayload::new("I like roasted whole lamb.")
                .sender("Alice")
                .role("user"),
        )
        .await?;

    let memories = client
        .search("What food does Alice like?", evermemos_agent_lib::SearchOptions::default())
        .await?;

    let builder = MemoryContextBuilder::new(client.clone());
    let context = builder.build("What food does Alice like?").await?;

    println!("{}", context);
    println!("{} memories", memories.len());
    Ok(())
}
```
