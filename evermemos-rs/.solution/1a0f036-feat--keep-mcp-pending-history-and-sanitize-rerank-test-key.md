## Background
The MCP memory ingestion path needed to preserve short-term turn context between incremental calls, and a new rerank backend test script plus memory tool instructions were added.

## Root Cause
- MCP single-message writes did not forward pending turn history across calls.
- The rerank test script contained a hardcoded API key, which is unsafe for source control.

## Solution
- Added in-process pending history cache in MCP (`PENDING_HISTORY`) keyed by `group_id::user_id`.
- Included `history` in `/api/v1/memories` payload and updated cache based on backend status (`accumulating`/`extracted`).
- Replaced hardcoded Cohere key with `COHERE_API_KEY` environment variable in test script.
- Added memory tool usage guidance file under `.github/instructions/`.

## Impacted Files
- evermemos-rs/src/mcp/mod.rs
- evermemos-rs/test_rerank_backend.py
- .github/instructions/memory.instructions.md

## Validation
- Commit created successfully and includes the above file changes.
- Sensitive key no longer hardcoded in repository files.
