# AI Agents Guidelines for Rullst Framework

Welcome! If you are an autonomous AI or coding assistant contributing to Rullst, this document is for you. Rullst is designed from the ground up to be "AI-native", meaning the architecture specifically avoids "magic" reflection at runtime in favor of strong typing and compile-time guarantees, which helps AIs reason about the code.

## Core Directives

1. **Strict Type Safety**: Do not use `dyn Trait` unless absolutely necessary. Rely on Rust's strong typing to guarantee safety.
2. **Explicit APIs**: Avoid hidden state or implicit magic. Every controller, middleware, and model should be explicit.
3. **SST (Single Source of Truth)**: The `docs/spec.md` is our absolute law. Always reference it before proposing architectural changes.
4. **Error Handling**: Use the typed `AppError` enum and never `panic!()`, `unwrap()`, or `expect()` in production paths. Graceful degradation is a must.
5. **Codegen**: When creating new features, prefer scaffolding generators (`make:*` commands) over manual file creation to maintain structural consistency.

Please read `.ai-rules` for specific context limits and linting boundaries.
