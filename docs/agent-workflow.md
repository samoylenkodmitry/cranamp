# Cranamp code workflow

## IDE tools

- Prefer RustRover MCP (`mcp__rustrover__*`) for semantic navigation, types/usages, call graphs, analysis and refactoring when ready: `search_symbol`, `get_symbol_info` and `analyze_calls` for code intelligence; `rename_refactoring` for semantic renames; `get_file_problems`, `lint_files` and `run_inspection_kts` for analysis. Pass `projectPath` on every IDE call.
- Use direct file tools for known-file reads, literal searches, documentation and simple edits. IDE `apply_patch`, `create_new_file` and `reformat_file` are optional when useful.
- Run build, test, Git, SSH and other shell commands directly through the shell/exec tool. RustRover's MCP terminal is not required; use it only for a specific IDE-terminal need or an explicit request.
- Open RustRover only when semantic work warrants the setup. If IDE access fails, explain the limitation once and use an appropriate fallback without compulsory reopening or repeated retries. The existing `scripts/dev/ide_search.py text|regex|symbol|file <query> [--in <glob>]... [--context N]` helper queries the same IDE server and is optional when that server is ready and useful.

## Test placement

- Keep all tests, test helpers, and fixtures in the root `/test/` directory. Never write test bodies, `#[test]` functions, inline test modules, or test-only helpers in implementation files, including `src/`, `tools/`, and `scripts/`. This rule applies to every language and to new generated code. Rust unit tests may retain a `#[cfg(test)]` and `#[path = ".../test/..."]` external module declaration beside the implementation to access private items. Register integration test targets in `Cargo.toml`; put web and platform tests under `/test/` too.
