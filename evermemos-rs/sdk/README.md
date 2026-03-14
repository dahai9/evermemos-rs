# evermemos-agent-lib

SDK workspace for connecting agent frameworks to an `evermemos-rs` server.

## Packages

- Python package: [evermemos-rs/sdk/python](evermemos-rs/sdk/python)
- TypeScript package: [evermemos-rs/sdk/typescript](evermemos-rs/sdk/typescript)
- Rust package: [evermemos-rs/sdk/rust](evermemos-rs/sdk/rust)

## Canonical layout

The canonical SDK layout is:

- [evermemos-rs/sdk/python](evermemos-rs/sdk/python)
- [evermemos-rs/sdk/typescript](evermemos-rs/sdk/typescript)
- [evermemos-rs/sdk/rust](evermemos-rs/sdk/rust)

Legacy compatibility paths are temporarily retained at [evermemos-rs/sdk](evermemos-rs/sdk) for Python and [evermemos-rs/sdk/ts](evermemos-rs/sdk/ts) for TypeScript.

## Python SDK

## Install

```bash
cd evermemos-rs/sdk/python
pip install -e .
```

## Quick Start

```python
import asyncio
from evermemos_agent_lib import EverMemOSClient, MemoryContextBuilder

async def main() -> None:
    client = EverMemOSClient(
        base_url="http://localhost:8080",
        user_id="alice",
        group_id="my_chat",
        org_id="my-org",
    )

    await client.memorize(content="I like roasted whole lamb.", sender="Alice", role="user")
    memories = await client.search("What food does Alice like?", retrieve_method="HYBRID", top_k=5)

    builder = MemoryContextBuilder(client)
    memory_context = await builder.build("What food does Alice like?")
    print(memory_context)
    print(memories)

    await client.aclose()

asyncio.run(main())
```

## Framework Adapters

- `build_langchain_messages(...)`
- `build_openai_messages(...)`
- `build_llamaindex_chat_history(...)`

All adapters return plain role/content message objects so you can pass them to your framework with minimal conversion.

## TypeScript SDK

See [evermemos-rs/sdk/typescript](evermemos-rs/sdk/typescript) for the TypeScript package and usage examples.

## Rust SDK

See [evermemos-rs/sdk/rust](evermemos-rs/sdk/rust) for the Rust crate and usage examples.
