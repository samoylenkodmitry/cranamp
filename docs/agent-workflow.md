# Cranamp code workflow

## IDE tools

- Do every refactoring, code search, code analysis and code edit through the RustRover MCP (`mcp__rustrover__*`), never with grep, sed, Python heredoc scripts or hand edits for those jobs: `rename_refactoring` for renames, `apply_patch` and `create_new_file` for edits, `search_symbol`, `get_symbol_info` and `analyze_calls` for declarations, usages and call graphs (the IDE knows the code; a regex over its text does not, so `search_text` and `search_regex` are for strings and comments only), `get_file_problems`, `lint_files` and `run_inspection_kts` for analysis, `reformat_file` for formatting; for text searches run `scripts/dev/ide_search.py text|regex|symbol|file <query> [--in <glob>]... [--context N]` from the project root, which asks the same IDE server and prints each hit as path, line and matched text. Pass `projectPath` on every call; when the tools are missing, open the tree in RustRover first (`open -a RustRover <path>`), and say so before any fallback. Bash is for building, testing and git.

## Test placement

- Keep all tests, test helpers, and fixtures in the root `/test/` directory. Never write test bodies, `#[test]` functions, inline test modules, or test-only helpers in implementation files, including `src/`, `tools/`, and `scripts/`. This rule applies to every language and to new generated code. Rust unit tests may retain a `#[cfg(test)]` and `#[path = ".../test/..."]` external module declaration beside the implementation to access private items. Register integration test targets in `Cargo.toml`; put web and platform tests under `/test/` too.
