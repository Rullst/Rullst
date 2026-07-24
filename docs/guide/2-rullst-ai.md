# Rullst AI: Developing with Autonomous Agents

Rullst was designed from the ground up to be the first **"AI-native"** Rust framework. What does this mean in practice?

Traditional frameworks rely heavily on runtime "magic" (reflection, dynamic string-based dependency injection, weak typing, and heavy metaprogramming). While this is great for humans writing short scripts, it is **terrible for AI Agents**, as it prevents the AI from validating whether the code is correct before running it.

In Rullst, we opted for **Strong Typing and Compile-Time Guarantees**. This allows the Rust compiler to act as an "absolute supervisor" for the AI. If the AI makes a mistake, the code won't compile, and the AI can read the detailed error and fix the problem instantly, creating a perfect feedback loop.

## 1. The Agent Manifesto (`AGENTS.md`)

Every Rullst project generated via `cargo rullst new` automatically includes two vital files in the root: `.ai-rules` and `AGENTS.md`.

These are the heart of AI-assisted development. The `AGENTS.md` file acts as the "Bible" of your project for any autonomous agent (like Cursor, Github Copilot, Gemini, or Claude). It tells the AI exactly how it should behave in your codebase.

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
1. Ask it to **read** the `docs/spec.md` and `AGENTS.md` files first.
2. Say: "Create a new Controller following the pattern established in `auth_controller.rs`". Today's AIs are brilliant at pattern matching. Rullst provides the skeleton, the AI fills in the meat.
3. Use the generators! Ask the AI to use `cargo rullst make:controller` in the terminal (if it's an autonomous agent), ensuring the correct file structure.
