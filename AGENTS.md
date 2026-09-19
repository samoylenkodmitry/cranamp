# Agent Notes for Cranamp

- Do every refactoring, code search and code analysis through the RustRover MCP (`mcp__rustrover__*`), never with ad hoc grep, sed or hand edits for those jobs: `rename_refactoring` for renames, `search_symbol`, `search_text`, `search_regex` and `analyze_calls` for search and call graphs, `get_file_problems`, `lint_files` and `run_inspection_kts` for analysis, `reformat_file` for formatting. Pass `projectPath` on every call; when the tools are missing, open the tree in RustRover first (`open -a RustRover <path>`), and say so before any fallback.
