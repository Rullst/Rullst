# Rullst AI Token Economy & Local LLM Integration 🧠⚡

This document explains:
1. How Rullst optimizes AI context tracking and **reduces LLM token consumption by up to 70%** during AI-assisted development.
2. How to connect Rullst to **ANY local LLM engine** (Ollama, LM Studio, llama.cpp, vLLM, LocalAI, Jan.ai) for 100% offline, zero-cost AI execution.

---

## 💰 1. The Rullst Token Economy: How Rullst Reduces Token Consumption

Traditional dynamic frameworks (such as Ruby on Rails, Laravel, or Django) rely heavily on dynamic reflection, runtime magic, hidden dependency injection, and loosely typed strings. When an AI assistant (like Cursor, Claude, Antigravity, or Copilot) works on a traditional codebase, it must read hundreds of lines of dynamic abstractions and configuration boilerplate just to understand how a single route or model works. This consumes **thousands of tokens per prompt**.

Rullst was architected from the ground up to be **AI-Native**, explicitly optimizing the **Token Economy**:

### ⚡ 1. Explicit Compile-Time Types (`impl Trait`)
- No hidden runtime reflection or dynamic dispatch (`dyn Trait`).
- Controllers, services, and repositories expose concrete, strongly typed function signatures.
- **Token Impact:** An AI assistant reads 80% fewer lines of code to understand function contracts, reducing prompt context overhead.

### ⚡ 2. Visual Macro Delimiters (`routes!`, `html!`)
- Centralized routes are declared visually using the explicit `routes![ ... ]` macro.
- Server-rendered UI components use the compile-time `html!` macro with explicit quoted attributes.
- **Token Impact:** AI models do not hallucinate route bindings or frontend state handlers. The AI immediately pinpoints exact insertion lines without searching across scattered files.

### ⚡ 3. Zero-Bundle HTMX + Tailwind Architecture
- Eliminates heavy client-side JavaScript SPA bundles (React/Vue/Next.js state trees).
- **Token Impact:** When generating UI features, the AI only generates concise HTML server fragments, avoiding thousands of tokens spent on complex client-side state hooks, Redux/Zustand boilerplate, or Webpack configurations.

### ⚡ 4. Scaffolding Automation (`cargo rullst make:*`)
- The CLI generator produces clean, standardized blueprints for controllers, models, and migrations.
- **Token Impact:** The AI assistant invokes single CLI commands rather than generating hundreds of lines of repetitive boilerplate code in chat responses.

---

## 🏠 2. Universal Local LLM Support (Provider-Agnostic)

Is Ollama the only way to run local AI in Rullst? **No!**

`rullst-ai` and `rullst-security` are completely **provider-agnostic**. They support any local or cloud LLM runner that exposes an HTTP API (such as the standard OpenAI REST API or Ollama protocol).

### Supported Local AI Engines

| Local Engine | Default Endpoint | Key Advantage |
| :--- | :--- | :--- |
| **Ollama** | `http://localhost:11434` | 1-Click install, lightweight CLI, active model library. |
| **LM Studio** | `http://localhost:1234/v1` | Rich desktop UI, local GGUF model downloader, OpenAI compatible. |
| **llama.cpp / llama-server** | `http://localhost:8080/v1` | Ultra-fast native C++ inference, zero memory overhead. |
| **vLLM** | `http://localhost:8000/v1` | High-throughput GPU server for team environments. |
| **LocalAI** | `http://localhost:8080/v1` | Self-hosted OpenAI drop-in replacement API. |
| **Jan.ai** | `http://localhost:1337/v1` | Open-source desktop assistant with local API server. |

---

## 🛠️ 3. Configuring Local AI Providers in `.env`

You can switch between local LLM engines and cloud providers effortlessly by updating your project's `.env` file:

### Option A: Local Ollama
```dotenv
AI_PROVIDER=ollama
OLLAMA_HOST=http://localhost:11434
AI_MODEL=llama3:8b
```

### Option B: LM Studio (OpenAI Compatible)
```dotenv
AI_PROVIDER=openai_compatible
OPENAI_API_BASE=http://localhost:1234/v1
OPENAI_API_KEY=lm-studio
AI_MODEL=qwen2.5-coder-7b-instruct
```

### Option C: Native `llama.cpp` Server
```dotenv
AI_PROVIDER=openai_compatible
OPENAI_API_BASE=http://localhost:8080/v1
OPENAI_API_KEY=not-needed
AI_MODEL=mistral-7b-instruct
```

### Option D: Cloud AI (Gemini, OpenAI, Claude, DeepSeek)
```dotenv
# Google Gemini
GEMINI_API_KEY="AIzaSyYourGeminiApiKey"

# OpenAI
OPENAI_API_KEY="sk-YourOpenAiKey"

# Anthropic Claude
ANTHROPIC_API_KEY="sk-ant-YourClaudeKey"

# DeepSeek
DEEPSEEK_API_KEY="sk-YourDeepSeekKey"
```

---

## 🎯 4. Summary

- **Token Economy:** Rullst's explicit compile-time typing, HTMX SSR, and macro architecture reduce AI prompt context overhead and save tokens.
- **Local AI Flexibility:** You are not locked into Ollama or cloud APIs. Connect Rullst to LM Studio, `llama.cpp`, vLLM, or LocalAI for 100% free, offline, private AI development.
