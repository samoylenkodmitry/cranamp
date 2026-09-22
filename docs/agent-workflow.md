# Cranamp code workflow

## IDE tools

- Use RustRover MCP (`mcp__rustrover__*`) for code search, understanding, analysis, refactoring and edits: `search_symbol`, `get_symbol_info` and `analyze_calls` for declarations, usages and call graphs; `rename_refactoring` for renames; `apply_patch` and `create_new_file` for edits; `get_file_problems`, `lint_files` and `run_inspection_kts` for analysis; `reformat_file` for formatting. Pass `projectPath` on every call. Use IDE `search_text` and `search_regex` only for strings and comments. Do not replace code intelligence with Bash/grep/rg scans, or code edits with sed, ad hoc scripts or hand edits.
- Run build, test, Git, SSH and other shell commands directly through the shell/exec tool. RustRover's MCP terminal is not required; use it only for a specific IDE-terminal need or an explicit request. This does not permit shell-based code discovery when IDE tools can answer the question.
- If direct IDE tools are unavailable, open the tree in RustRover first (`open -a RustRover <path>`) and retry; explain any remaining limitation before a fallback. The existing `scripts/dev/ide_search.py text|regex|symbol|file <query> [--in <glob>]... [--context N]` helper queries the same IDE server and may serve as a fallback, not the default when direct MCP search works.

## Test placement

- Keep all tests, test helpers, and fixtures in the root `/test/` directory. Never write test bodies, `#[test]` functions, inline test modules, or test-only helpers in implementation files, including `src/`, `tools/`, and `scripts/`. This rule applies to every language and to new generated code. Rust unit tests may retain a `#[cfg(test)]` and `#[path = ".../test/..."]` external module declaration beside the implementation to access private items. Register integration test targets in `Cargo.toml`; put web and platform tests under `/test/` too.
