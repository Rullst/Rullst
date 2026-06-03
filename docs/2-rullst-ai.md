# Rullst AI: Developing with AI Agents

Rullst is designed from the ground up to be **"AI-native"**. This means the architecture intentionally avoids "magic" reflection at runtime in favor of strong typing and compile-time guarantees, which helps AI coding assistants (like Copilot, Claude, or Gemini) reason about the code effectively.

## 1. Setting up AI Guardrails

To get the best results when pairing with an AI to develop your Rullst applications, you should provide the AI with strict guidelines.

Rullst projects typically include a `.ai-rules` and `AGENTS.md` file in the root directory.

### The `AGENTS.md` File

Create an `AGENTS.md` file in the root of your project:

```markdown
# AI Agents Guidelines for My Rullst App

Welcome! If you are an autonomous AI or coding assistant contributing to this app, please follow these directives:

1. **Strict Type Safety**: Do not use `dyn Trait` unless absolutely necessary. Rely on Rust's strong typing to guarantee safety.
2. **Explicit APIs**: Avoid hidden state or implicit magic. Every controller, middleware, and model should be explicit.
3. **HTML Macros**: Always quote boolean attributes in the `html!` macro (e.g. `required="true"`).
4. **Error Handling**: Use the typed `AppError` enum and never `panic!()`, `unwrap()`, or `expect()` in production paths.
```

## 2. Best Practices for AI Generation

When asking an AI to generate a new controller or page in Rullst, always mention the following rules:

1. **Routing Macros**: The `routes!` macro syntax requires parentheses: `get("/" => handler),`.
2. **Zero-Allocation**: Tell the AI to use the `html!` macro for rendering.
3. **Database Concurrency**: If the AI writes a background worker, remind it to use a `sleep` delay during initialization to avoid SQLx panics.

By setting up these guardrails, your AI assistant will become a highly efficient co-pilot, seamlessly writing native Rust code that perfectly aligns with Rullst's strict type safety and high-performance philosophy!
