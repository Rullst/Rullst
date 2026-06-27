# AI Agents Guidelines for Rullst Framework

Welcome! If you are an autonomous AI or coding assistant contributing to Rullst, this document is for you. Rullst is designed from the ground up to be "AI-native", meaning the architecture specifically avoids "magic" reflection at runtime in favor of strong typing and compile-time guarantees, which helps AIs reason about the code.

## Core Directives

1. **Static Dispatch over Dynamic**: Prefer static dispatch (`impl Trait` or generics) over `dyn Trait` to ensure explicit concrete types for AI context tracking and optimization.
2. **Explicit APIs**: Avoid hidden state or implicit magic. Every controller, middleware, and model should be explicit.
3. **SST (Single Source of Truth)**: The `docs/spec.md` is our absolute law. Always reference it before proposing architectural changes.
4. **Error Handling**: Use the typed `AppError` enum and never `panic!()`, `unwrap()`, or `expect()` in production paths. Graceful degradation is a must. In tests, however, `unwrap()` and `expect()` are fully allowed and encouraged for assertions.
5. **Codegen**: When creating new features, prefer scaffolding generators (`make:*` commands) over manual file creation to maintain structural consistency.
6. **HTML Macros**: Boolean attributes in the `html!` macro must be explicitly quoted (e.g., `required="true"`).

Please read `.ai-rules` for specific context limits and linting boundaries.
