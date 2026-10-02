# Terminal AI assistant (`cargo rullst ai`)

`cargo rullst ai` is a chat in your terminal that knows Rullst's conventions and
your project's layout. It can answer questions and propose concrete changes —
scaffolding commands, file edits and `cargo check` — that you review one at a
time. It is a v13 preview. The full option list is in the
[CLI reference](cli_reference.md#cargo-rullst-ai-v13-preview).

## 1. Try it offline

No account is needed to see how it works. Without a configured provider, a
deterministic offline assistant answers:

```bash
cd my_app
cargo rullst ai "add a posts page"
```

The offline assistant always proposes the same two-step plan: create
`rullst-ai-demo.md`, then edit one line of it. In a terminal you see each
change as a coloured diff and answer `y` (apply), `n` (skip), `a` (apply the
rest of this turn) or `q` (stop the turn). Delete the demo file afterwards.
Started outside a project, it first proposes
`cargo rullst new rullst-ai-demo --default --blueprint blank --skip-initial-migration`
and, once you accept, continues inside the new project.

## 2. Connect a provider

```bash
cargo rullst ai connect
```

Choose OpenAI, Anthropic Claude, Google Gemini, DeepSeek, Ollama or a local
OpenAI-compatible server, confirm a model (press Enter for the provider
default) and paste the API key (input is hidden; leave it empty to keep using
an environment variable). For Ollama, give the host instead
(`http://127.0.0.1:11434` by default). Finally, you can record your own prices
per million input and output tokens to see cost estimates; skip it to see
token counts only.

### Local models

Any server that exposes the OpenAI chat API on your machine works: LM Studio
(`http://127.0.0.1:1234/v1`, the default), llama.cpp server or LocalAI
(`http://127.0.0.1:8080/v1`), vLLM (`http://127.0.0.1:8000/v1`) or Jan
(`http://127.0.0.1:1337/v1`). A `localhost` URL is pinned to `127.0.0.1`
(remote hosts are refused). Use the model name exactly as the server lists it:

```bash
cargo rullst ai connect --provider local --base-url http://127.0.0.1:8080/v1 --model qwen2.5-coder
```

Answers stream; token counts appear when the server reports them.

The settings are saved for your user, never in the project:
`~/.config/rullst/credentials.toml` (`$XDG_CONFIG_HOME/rullst/...`, or
`%APPDATA%\rullst\credentials.toml` on Windows), readable only by you. An
environment variable such as `OPENAI_API_KEY` always wins over the file, which
suits CI and shared machines. In scripts:

```bash
printf '%s\n' "$OPENAI_API_KEY" | cargo rullst ai connect --provider openai --api-key-stdin
cargo rullst ai connect --provider anthropic --input-price-per-mtok 3 --output-price-per-mtok 15
cargo rullst ai status          # shows provider, model and where the key comes from
cargo rullst ai disconnect      # deletes the saved file
```

## 3. Chat and build

```bash
cargo rullst ai
```

```text
Rullst AI · OpenAI · gpt-4o-mini · project my_app
› add a Post model with a title and a published flag, plus a page that lists posts
```

The assistant receives a primer about Rullst (routing, `html!`, models,
migrations, security rules and the `make:*` commands) and a bounded inventory
of your project: package and dependency names, enabled features, configuration
key names and source paths. It does not receive file contents unless you share
them:

```text
› /add src/main.rs
Attached src/main.rs (1843 bytes) to your next message.
› register the posts routes
```

A typical answer explains the plan, then asks about each action:

```text
[1/3] run
  $ cargo rullst make:model Post --migration
  (may create or change project files)
Apply? [y]es / [n]o / [a]ll this turn / [q]uit turn: y
Checkpoint refs/rullst/ai-checkpoints/20261001T120000Z (3f2a9c1d04b7) saved before the first change.
  review:  git diff refs/rullst/ai-checkpoints/20261001T120000Z
  restore: git restore --source=refs/rullst/ai-checkpoints/20261001T120000Z --worktree -- .
```

After the actions run, their results go back to the model, which continues or
summarizes. When files changed, the CLI offers to run `cargo check`. Apply
database migrations yourself with `cargo rullst db:migrate`.

## 4. What the assistant can and cannot do

| Allowed (after your confirmation) | Never allowed |
| --- | --- |
| Create or replace a file below the project root | Paths outside the project, `..`, absolute paths or symlinks |
| Replace one exact text occurrence in a file | `.git/`, `target/`, `.cargo/`, `.env*` (except `.env.example`), keys, credentials, `Cargo.lock`, toolchain files |
| `cargo rullst make:*`, `generate:*` (not `generate:models`), `db:status`, `doctor`, `audit`, `inspect routes`/`models`/`schema` | `deploy`, `foundry:*`, `upgrade`, `update`, `pkg`, `db:rollback`, `db:seed`, `doctor --fix`, `audit --network`, `inspect <file>`, shell commands |
| `cargo rullst db:migrate` in a development or test project, confirmed on its own | `db:migrate` when the environment the application would use (process `RULLST_ENV`/`APP_ENV` first, then `.env`, then `[app].env`) is staging or production |
| Outside a project: `cargo rullst new <name> --default [--blueprint …] [--database …]`, confirmed on its own | `new` inside a project or over an existing directory |
| `cargo check`, `cargo test` (simple flags only) | `cargo run`, `cargo install`, `--manifest-path`, `--config`, `-Z` |

Without an interactive terminal (piped input, `CI` set or `TERM=dumb`) or with
`--dry-run`, proposed actions are printed as a plan and never executed.

## 5. Build an application step by step

Describe the product and let the assistant drive the steps:

```text
› let's build a small course platform with courses and lessons
```

Outside a project it starts with `cargo rullst new` (choosing a blueprint such
as `lms`, `saas` or `blank`) and continues inside the new directory. Then it
works through models with migrations, `db:migrate` (development only),
controllers and `routes!`, `html!` views, and tests, running `cargo check`
between steps. Each step is a set of actions you review; send another message
to continue when a turn reaches its step limit.

## 6. Upgrade the framework

```bash
cargo rullst ai upgrade
```

The command first prints the plan of `cargo rullst upgrade --dry-run`. When
the plan reports source findings (for example a `render_page` call that now
needs a page language, or a dynamic `onclick={...}` in `html!`), the assistant
works through them, must-change findings first, with the same reviewed edits,
checkpoint and `cargo check`. It receives only the migration rows of those
findings and the affected files the path policy allows; it never edits Rullst
dependency versions, which `cargo rullst upgrade` applies. The
[assisted upgrade tutorial](tutorials/36-assisted-framework-upgrades.md#assisted-fixes-with-cargo-rullst-ai-upgrade)
explains the order of the two commands.

## 7. Undo

The first change of each session is preceded by a git checkpoint stored under
`refs/rullst/ai-checkpoints/`. It is built in a temporary index, so your staged
changes, stash and files are not touched, and it excludes `.env*` files and
`target/`. To review or undo everything since the checkpoint, run the printed
commands from the project root:

```bash
git diff refs/rullst/ai-checkpoints/<timestamp>
git restore --source=refs/rullst/ai-checkpoints/<timestamp> --worktree -- .
```

Files created after the checkpoint are not deleted by `git restore`; `git
status` lists them. Remove old checkpoints with
`git update-ref -d refs/rullst/ai-checkpoints/<timestamp>`. Outside a git
repository the CLI asks before changing anything without a checkpoint.

## 8. Safety notes

- Everything that comes from the project, shared files or command output is
  sent as delimited, size-capped untrusted data and checked by the `rullst-ai`
  guardrails; matching content is withheld and the rest of the conversation
  continues.
- Model output is printed with terminal control characters escaped, so an
  answer cannot rewrite your screen or clipboard.
- Review every diff: an edit to `build.rs`, `Cargo.toml` or a test runs code on
  the next `cargo check` or `cargo test`. Such files are flagged in the review.
- After each reply the CLI shows the tokens the provider reported and, on exit,
  the session totals. Nothing is estimated when a provider reports no usage. A
  cost appears only at prices you configured and is labelled as an estimate
  (cache discounts are not modelled); your provider's dashboard is the billing
  record.
- A database migration cannot be undone by the git checkpoint; it is offered
  only for development or test projects and always asks on its own.
- An OS keyring is not used; the credentials file is protected by file
  permissions only, like Cargo's own `credentials.toml`.
