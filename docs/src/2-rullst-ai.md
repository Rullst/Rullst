# Rullst AI: Developing with Autonomous Agents

Rullst was designed from the ground up to be the first **"AI-native"** Rust framework. What does this mean in practice?

Traditional frameworks rely heavily on runtime "magic" (reflection, dynamic string-based dependency injection, weak typing, and heavy metaprogramming). While this is great for humans writing short scripts, it is **terrible for AI Agents**, as it prevents the AI from validating whether the code is correct before running it.

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

The Rullst repository maintains a root `AGENTS.md` for contributors. The current
`cargo rullst new` scaffold does **not** copy `AGENTS.md`, `.ai-rules`, or
tool-specific instruction files into an application. Add reviewed project-local
instructions yourself when using an autonomous coding tool; do not assume the
framework's repository policy applies to generated application code.

`cargo rullst generate:ai-context` can generate `.llms.txt` from recognized
project dependencies and source directories. That snapshot can help a coding
assistant navigate the application, but it is not an instruction-policy file
and should be regenerated and reviewed after structural changes.

Example of the default content:
```markdown
1. **Static Dispatch over Dynamic**: Prefer static dispatch (`impl Trait` or generics) over `dyn Trait` to ensure explicit concrete types for AI context tracking and optimization.
2. **Explicit APIs**: Avoid hidden state. Every controller and middleware should be explicit in its arguments.
3. **HTML Macros**: Boolean attributes in the `html!` macro must be quoted (e.g., `required="true"`).
4. **No Panics**: Never use `unwrap()` or `expect()` in production routes.
```

## 2. Rullst's AI-Friendly Patterns

Rullst's API was designed so that the AI rarely hallucinates:

- **Explicit Routes:** The `routes![ ... ]` macro is visual and delimited. The AI knows exactly where to add a new route without having to search across scattered files.
- **Rullst ORM:** Based on Pure SQL (via SQLx) + Derives. AIs are much better at writing pure, correct SQL queries than learning an obscure query builder. Rullst takes advantage of this by using the database in a pure relational way.
- **Clean Background Workers:** The queue system does not require complex global registration; you simply create an async function.

## 3. How to Get the Best Results

When instructing an AI to add a feature in Rullst:
1. Ask it to read the instruction files that actually exist in your application
   and consult the matching version of the Rullst documentation.
2. Say: "Create a new Controller following the pattern established in `auth_controller.rs`". Today's AIs are brilliant at pattern matching. Rullst provides the skeleton, the AI fills in the meat.
3. Use the generators! Ask the AI to use `cargo rullst make:controller` in the terminal (if it's an autonomous agent), ensuring the correct file structure.
