# Agent Notes for Cranamp

- Keep tool results near 2,000 tokens by default; return relevant excerpts, failures and changed results. Save full logs to files and expand only when needed. Discover only needed tool schemas; reuse unchanged instructions and evidence.
- Batch independent reads/checks. Wait 30–60 seconds for long jobs when supported; report progress between waits. Avoid tight polling, repeated status checks and unchanged log dumps.
- Run targeted checks after meaningful edits and required broad checks before integration; rerun only for changed code, failures or new risks. Documentation-only edits need link/format validation, not product builds.
- Work without subagents unless requested. When requested, use bounded tasks and minimal context; preserve the project's model constraints. After two passes with no measurable progress, revisit the hypothesis or reference and change approach. Do not declare unfinished work complete.

- Use RustRover MCP for code search, understanding, analysis, refactoring and edits; pass `projectPath`. Prefer IDE tools over Bash/grep/rg for code discovery. Run build/test/git and other shell commands directly through the shell tool; RustRover's MCP terminal is not required. Read [IDE workflow](docs/agent-workflow.md) before code work.
- Keep all tests, test helpers and fixtures in root `/test/`; implementation files may contain only external test-module declarations, never test bodies. See [test placement](docs/agent-workflow.md#test-placement).
- Before skin artwork or Skin Studio rendering changes, read [artwork review](docs/skin-studio/artwork-review.md) and use the `cranamp-skin-studio` skill. Preserve the accepted baseline; inspect actual GPU pixels and every marked region. Passing write/export checks never approves composition.
- Classic sprites are opaque, including magenta; runtime overlays do not make underlying bitmap pixels inaccessible. Read [compatibility](docs/skin-studio/classic-compatibility.md) for sprite, transparency or export work; [EQ workbench](docs/skin-studio/equalizer-workbench.md) for EQ work.

## Simple speech

Never leave prepositions trailing at the ends of clauses (e.g., use "the version by which..." instead of "the version... by"), and keep modifiers close to the words they modify. Use simple words in their original meaning. no poetic, no jargon. No contrastive sentences. No special symbols. No Trailing Participial Phrases. NO for any of these: Overused Buzzwords, Empty Transition Openers, unnecessary adjectives that try to sell an ordinary fact, list things in triples (e.g., "fast, efficient, and reliable" or "streamline, optimize, and scale")., Not only... but also..., wrap-up summaries that don't add actual data. | **No sales pitch or hype:** Adopt a neutral, engineering-first tone. Never use marketing fluff, exclamation points for enthusiasm, or words designed to "sell" a feature (e.g., *effortless, supercharge, magical, lightning-fast*). State facts and mechanics directly.
