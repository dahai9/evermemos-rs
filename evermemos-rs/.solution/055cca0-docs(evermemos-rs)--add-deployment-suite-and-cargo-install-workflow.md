## Background
The Rust rewrite needed documentation parity with the Python project, especially around deployment, configuration, file usage, and operational onboarding. A new requirement also asked for direct binary installation of server and MCP via cargo install.

## Root Cause
- `evermemos-rs` lacked a top-level README and structured docs index.
- There was no complete operator-focused deployment/config/file guide set.
- Binary installation workflow (`cargo install`) was not documented or exposed in task recipes.

## Solution
- Added a top-level Rust README as an entry point.
- Added docs index, deployment guide, config reference, and file guide under `evermemos-rs/docs`.
- Updated parity matrix doc to reflect current rewrite stage and completed milestones.
- Added `just` recipes for binary installation/uninstallation:
  - `install`
  - `install-behavior-history`
  - `uninstall`
- Documented `cargo install` flows for both local path install and git install.

## Impacted Files
- evermemos-rs/justfile
- evermemos-rs/README.md
- evermemos-rs/docs/README.md
- evermemos-rs/docs/DEPLOYMENT.md
- evermemos-rs/docs/CONFIG_REFERENCE.md
- evermemos-rs/docs/FILE_GUIDE.md
- evermemos-rs/docs/RUST_VS_PYTHON.md

## Validation
- Commit created successfully with all documentation and justfile updates.
- Working tree is clean after amend.
- Installation commands are now documented and available via `just`.
