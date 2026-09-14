AGENTS: Recommended agent context for rullst-orm

Purpose: provide quick context and example prompts for AI agents and contributors.

Reference spec: see the public [framework specification](https://rullst.github.io/Rullst/book/spec.html).

Suggested prompts:
- "Summarize the macro expansion for `#[derive(Orm)]` in one paragraph."
- "List public API surface for `rullst-orm` and their expected types."
- "Run `cargo clippy` and suggest minimal fixes for any warnings."

CI hints:
- Add `cargo test --workspace --all-features`, `cargo clippy --workspace --all-features --all-targets -- -D warnings`, and `cargo audit` as required checks.

Maintainers: add any project-specific agent prompts here.
