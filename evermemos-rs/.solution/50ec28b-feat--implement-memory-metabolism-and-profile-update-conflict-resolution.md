# Memory Metabolism and Profile Update Conflict Resolution

## Background
The user profile extraction mechanism needed to support conflict resolution ("metabolism") over time when user preferences change (e.g., stopping drinking coffee and starting green tea). Additionally, we needed to make sure the database properly handles schemaless sub-objects in a `SCHEMAFULL` table, and basic spatiotemporal graph relation preparations were introduced.

## Root Cause
When inserting a struct with an untyped JSON inner map into a SurrealDB `SCHEMAFULL` table, the property `option<object>` filters out any inner fields that are not explicitly defined in the schema. This caused the AI-extracted `profile_data` to be saved as an empty object `{}`. Furthermore, binary serialization issues arose during partial object updates (`MERGE`).

## Solution
1. **Schema Fix**: Updated `schema.rs` to mark `user_profile.profile_data`, `custom_profile_data`, and other nested JSON objects as `FLEXIBLE`, bypassing strict inner property filtering.
2. **Upsert Fix**: Refactored `user_profile.rs` `upsert` queries to properly serialize JSON payloads into inline object strings during `MERGE` to bypass serialization bugs with SurrealDB untagged objects.
3. **LLM Metabolism**: Extended `ProfileExtractor` and prompt templates (`en.rs`) to pass existing user profiles into the LLM context, instructing the model to perform conflict resolution explicitly.
4. **Client Validation**: Added an endpoint (`/api/v1/global-user-profile`) and python SDK support for `get_global_profile` to retrieve the latest profile.
5. **Graph Relations**: Initial definitions and code updates to map `user -> experienced/produced -> memory` for upcoming spatiotemporal queries.

## Impacted Files
- `evermemos-rs/src/storage/schema.rs`
- `evermemos-rs/src/storage/repository/user_profile.rs`
- `evermemos-rs/src/memory/profile_extractor.rs`
- `evermemos-rs/src/memory/prompts/en.rs`
- `evermemos-rs/src/api/global_profile_router.rs`
- `evermemos-rs/sdk/python/evermemos_agent_lib/client.py`
- `evermemos-rs/sdk/python/test_metabolism.py`

## Validation
Created and successfully ran `test_metabolism.py` end-to-end, which verifies that an initial preference ("coffee") is accurately superseded and metabolized into a new conflicting preference ("green tea").
