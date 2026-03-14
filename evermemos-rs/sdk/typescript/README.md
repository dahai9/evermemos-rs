# @evermemos/agent-lib (TypeScript)

A lightweight TypeScript SDK for connecting agent frameworks to an `evermemos-rs` server.

## Install

```bash
cd evermemos-rs/sdk/typescript
npm install
npm run build
```

## Quick Start

```ts
import { EverMemOSClient, MemoryContextBuilder } from "@evermemos/agent-lib";

const client = new EverMemOSClient({
  baseUrl: "http://localhost:8080",
  userId: "alice",
  groupId: "my_chat",
  orgId: "my-org",
});

await client.memorize({
  content: "I like roasted whole lamb.",
  sender: "Alice",
  role: "user",
});

const memories = await client.search("What food does Alice like?", {
  retrieveMethod: "HYBRID",
  topK: 5,
});

const builder = new MemoryContextBuilder(client);
const memoryContext = await builder.build("What food does Alice like?");

console.log(memoryContext);
console.log(memories);
```
