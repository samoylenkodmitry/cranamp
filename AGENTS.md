# Agent Notes for Cranamp

- Keep tool results near 2,000 tokens by default; return relevant excerpts, failures and changed results. Save full logs to files and expand only when needed. Discover only needed tool schemas; reuse unchanged instructions and evidence.
- Batch independent reads/checks. Wait 30–60 seconds for long jobs when supported; report progress between waits. Avoid tight polling, repeated status checks and unchanged log dumps.
- Run targeted checks after meaningful edits and required broad checks before integration; rerun only for changed code, failures or new risks. Documentation-only edits need link/format validation, not product builds.
- Work without subagents unless requested. When requested, use bounded tasks and minimal context; preserve the project's model constraints. After two passes with no measurable progress, revisit the hypothesis or reference and change approach. Do not declare unfinished work complete.

- Use RustRover MCP for code search, understanding, analysis, refactoring and edits; pass `projectPath`. Prefer IDE tools over Bash/grep/rg for code discovery. Run build/test/git and other shell commands directly through the shell tool; RustRover's MCP terminal is not required. Read [IDE workflow](docs/agent-workflow.md) before code work.
- Keep all tests, test helpers and fixtures in root `/test/`; implementation files may contain only external test-module declarations, never test bodies. See [test placement](docs/agent-workflow.md#test-placement).
- Before skin artwork or Skin Studio rendering changes, read [artwork review](docs/skin-studio/artwork-review.md) and use the `cranamp-skin-studio` skill. Preserve the accepted baseline; inspect actual GPU pixels and every marked region. Passing write/export checks never approves composition.
- Classic sprites are opaque, including magenta; runtime overlays do not make underlying bitmap pixels inaccessible. Read [compatibility](docs/skin-studio/classic-compatibility.md) for sprite, transparency or export work; [EQ workbench](docs/skin-studio/equalizer-workbench.md) for EQ work.
