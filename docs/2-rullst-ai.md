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

---

## 4. 🧠 Autonomous Tiered AI Architecture (Local-First Strategy)

One of the largest hurdles in modern software development is the **uncontrolled financial cost of Cloud LLM tokens** (OpenAI GPT-4o, Anthropic Claude 3.5 Sonnet, Google Gemini 1.5 Pro). Sending thousands of lines of raw source code across network boundaries for every routine task (such as error diagnostics, bot classification, or security log analysis) leads to rapid token inflation.

To solve this, Rullst introduces a **Hybrid Local-First 3-Tier AI Architecture**. By routing high-frequency runtime operations to zero-cost local LLMs while reserving Cloud LLMs exclusively for complex reasoning, Rullst cuts LLM operational costs by **up to 95%**.

### 🛡️ 3-Tier Execution Matrix

| Tier | Engine / Provider | Rullst Domain & Use Cases | Token Cost | Resource Footprint |
| :--- | :--- | :--- | :--- | :--- |
| **Tier 1: Local Runtime & SOC** | **Local Ollama** (`llama3:8b`, `qwen2.5-coder`, `mistral`) | AI Threat Sentinel, Honeypot Bot Classification, RASP Anomaly Detection, Dev Self-Healing Error Console. | **$0.00** (Free) | Runs locally on VPS / Server |
| **Tier 2: Vector Semantic Search** | **Local Embeddings** (`bge-small`, `nomic-embed`) | Vector RAG semantic search in `rullst-ai` across codebase documentation, database schemas, and knowledge bases. | **$0.00** (Free) | Runs in CPU memory |
| **Tier 3: Cloud Agent Orchestration** | **Cloud LLM APIs** (Gemini 1.5 Pro, Claude 3.5 Sonnet, GPT-4o) | `cargo rullst audit --ai`, complex multi-file architectural refactoring, custom blueprint generation from scratch. | **Pay-per-use on-demand** | Remote Cloud API |

---

## 5. ⚡ How Rullst Reduces Token Consumption by 95%

### 5.1. AST Schema Context Compression (`rullst-schema.json`)

Piping full Rust `.rs` files into an LLM prompt easily consumes **50,000+ tokens per request**. Rullst solves this using **Abstract Syntax Tree (AST) Compression**:
- `cargo-rullst` automatically compiles a lightweight JSON representation of your project (`rullst-schema.json`).
- This file contains **only exact struct definitions, handler signatures, and SQL database schemas** without method bodies.
- Prompts sent to AI agents inject only the compact AST representation, reducing context payload from **50,000 tokens to ~1,200 tokens (a 97.6% savings per prompt)**.

```
┌────────────────────────────────────────────────────────┐
│               RAW SOURCE CODE (.rs)                    │
│   50,000 Tokens ──► Heavy Cloud Costs ($0.30/call)     │
└──────────────────────────┬─────────────────────────────┘
                           │ Rullst AST Compiler
                           ▼
┌────────────────────────────────────────────────────────┐
│             COMPACT AST SCHEMA (JSON)                  │
│    1,200 Tokens ──► Free Local Ollama ($0.00/call)     │
└────────────────────────────────────────────────────────┘
```

### 5.2. Local Ollama Priority Dispatcher

The `rullst-ai` crate implements an automated **Priority Fallback Dispatcher**:
1. When a security anomaly or runtime error occurs, Rullst first checks if a local model is available (`OLLAMA_HOST`).
2. If available, the task (such as classifying an aggressive bot or generating a self-healing patch during `cargo rullst dash`) is executed **100% locally at $0 cost**.
3. If deep reasoning is required (e.g. generating an enterprise billing system from scratch), Rullst escalates the prompt to Tier 3 Cloud APIs.

### 5.3. Self-Healing Error Console in Dev Mode

During local development with `cargo rullst dash`:
- When Rust compilation errors occur, `rullst-core` intercepts the compiler output.
- The local Tier 1 model diagnoses lifetime or type mismatches instantly and suggests precise code diff patches directly in the terminal interface.

---

## 6. 🛠️ Universal Provider Configuration & Zero Lock-In

Rullst AI is **100% provider-agnostic**. You can connect to **ANY AI model** (local or cloud) simply by adding credentials to your project's `.env` file:

```dotenv
# ── Local & Offline Models (100% Free / Zero Token Cost) ─────────────
OLLAMA_HOST="http://127.0.0.1:11434"

# ── Cloud LLM Providers (On-Demand Pay-per-Use) ──────────────────────
GEMINI_API_KEY="AIzaSyYourGeminiApiKeyHere"
OPENAI_API_KEY="sk-YourOpenAiKeyHere"
ANTHROPIC_API_KEY="sk-ant-YourClaudeKeyHere"
DEEPSEEK_API_KEY="sk-YourDeepSeekKeyHere"
GROQ_API_KEY="gsk-YourGroqKeyHere"
```


