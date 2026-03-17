# Unit Tests for Memory Metabolism and User Profile Updates

## Background
Verification of the recently implemented "Memory Metabolism" and "Conflict Resolution" mechanisms in the Rust implementation of EverMemOS.

## Root Cause
N/A (Feature verification and doctest fix)

## Solution
1. **Repository Testing**: Added `#[cfg(test)]` module to `user_profile.rs`. Utilized `tempfile` to create temporary RocksDB instances for integration testing of the `UPSERT MERGE` logic in `UserProfileRepo`.
2. **Extractor Testing**: Added `#[cfg(test)]` module to `profile_extractor.rs`. Implemented a `MockLlm` to simulate LLM responses for personality (Part 1), demographics (Part 2), and life summary. Verified that the extractor correctly merges new findings into existing profiles while preserving unrelated historical data.
3. **Doctest Fix**: Marked the ASCII architecture diagram in `mcp/mod.rs` as `text` code block to prevent the Rust compiler from attempting to parse it as code during `cargo test`.

## Impacted Files
- `evermemos-rs/src/mcp/mod.rs`
- `evermemos-rs/src/memory/profile_extractor.rs`
- `evermemos-rs/src/storage/repository/user_profile.rs`

## Validation
Successfully executed `cargo test` with 46 passing tests, specifically confirming `test_user_profile_upsert_and_get`, `test_user_profile_upsert_custom_profile`, `test_initial_extraction`, and `test_metabolism_update`.
