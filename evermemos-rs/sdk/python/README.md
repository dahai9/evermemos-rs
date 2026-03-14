# evermemos-agent-lib (Python)

A lightweight Python SDK for connecting agent frameworks to an `evermemos-rs` server.

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
