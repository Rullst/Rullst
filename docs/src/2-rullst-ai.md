# Rullst AI: Developing with Autonomous Agents

Rullst is designed as an **AI-native** Rust framework. Here that term describes
inspectable code-generation and explicit compile-time contracts; it is not a
claim of historical priority or proof that an AI will produce correct code.

Some framework designs use runtime reflection, string-based dependency
injection, or conventions that are harder to discover through static source
inspection. Rullst instead favors visible types and generated Rust, while still
retaining ordinary runtime state and framework abstractions where appropriate.

Rullst favors strong typing and compile-time diagnostics. The compiler catches
type and ownership errors in generated code, but it cannot validate requirements,
security intent, or business correctness; AI-generated changes still require
review and tests.

For runtime LLM integrations, consult the machine-readable and documented
[provider capability matrix](ai-provider-capabilities.md). Streaming, native
tools, built-in deadlines/retries, and explicit cancellation are not implied by
the existence of a provider adapter.

Local Rust tools use a separate [guarded execution boundary](ai-tool-security.md)
with an exact allowlist, principal authorization, closed JSON schema, payload
limits, call budget and mandatory audit sink. Destructive and financial calls
also require a one-use approval bound to the exact payload.

## 1. Repository instructions for coding agents

The Rullst repository maintains its own contributor `AGENTS.md`. The v13 CLI
candidate creates separate application instructions during `cargo rullst new`
and preserves any existing project instructions during regeneration. The
framework repository policy is not copied wholesale into applications.

`cargo rullst generate:ai-context` writes a bounded metadata inventory to
`.llms.txt` and `.rullst/context-map.json`; `--check` verifies freshness without
writing. Source bodies and configuration values are excluded. Read the
[context guide](project-context.md) for the scope, limits and legacy migration.
The generated instructions and inventory do not establish feature readiness or
replace the application's authentication, privacy or deployment decisions.

Conventions to retain when adapting project instructions:
```markdown
1. **Static Dispatch over Dynamic**: Prefer static dispatch (`impl Trait` or generics) over `dyn Trait` to ensure explicit concrete types for AI context tracking and optimization.
2. **Explicit APIs**: Avoid hidden state. Every controller and middleware should be explicit in its arguments.
3. **HTML Macros**: Boolean attributes in the `html!` macro are bare (`required`) or conditional (`disabled?={is_locked}`); never `disabled="false"`, which still disables.
4. **No Panics**: Never use `unwrap()` or `expect()` in production routes.
```

## 2. Rullst's AI-Friendly Patterns

Rullst's API is designed to give coding tools more explicit source landmarks;
this can reduce ambiguity but does not measure or guarantee hallucination rates:

- **Explicit Routes:** The `routes![ ... ]` macro is visual and delimited. The AI knows exactly where to add a new route without having to search across scattered files.
- **Rullst ORM:** `#[derive(Orm)]` generates typed Active Record methods and a fluent query builder over SQLx; a query the builder does not cover is written in SQL with bound values (`Orm::raw` or SQLx). See the [ORM page](crates/orm.md).
- **Explicit background workers:** a worker registers an async handler under a job name (`worker.register("email", ...)`) and is started explicitly; `cargo rullst make:worker` writes both parts.

## 3. How to Get the Best Results

When instructing an AI to add a feature in Rullst:
1. Ask it to read the instruction files that actually exist in your application
   and consult the matching version of the Rullst documentation.
2. Point it at an existing file: "Create a new controller following the pattern in `src/controllers/auth_controller.rs`" (written by `cargo rullst auth`). A concrete example in the project gives the assistant a pattern to follow.
3. Use the generators! Ask the AI to use `cargo rullst make:controller` in the terminal (if it's an autonomous agent), ensuring the correct file structure.
4. Run `cargo rullst generate:ai-context` so the assistant in your editor or
   terminal reads the project's `AGENTS.md` and its `.llms.txt` inventory, and
   point it at the framework's published
   [`llms.txt`](https://rullst.github.io/Rullst/llms.txt). Review every edit it
   proposes. (`cargo rullst ai`, the terminal assistant of the earlier 13.0
   prereleases, was removed in 13.0.0-alpha.4.)
