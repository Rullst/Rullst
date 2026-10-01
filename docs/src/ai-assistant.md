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

## 2. Connect a provider

```bash
cargo rullst ai connect
```

Choose OpenAI, Anthropic Claude, Google Gemini, DeepSeek or Ollama, confirm a
model (press Enter for the provider default) and paste the API key (input is
hidden; leave it empty to keep using an environment variable). For Ollama, give
the host instead (`http://127.0.0.1:11434` by default).

The settings are saved for your user, never in the project:
`~/.config/rullst/credentials.toml` (`$XDG_CONFIG_HOME/rullst/...`, or
`%APPDATA%\rullst\credentials.toml` on Windows), readable only by you. An
environment variable such as `OPENAI_API_KEY` always wins over the file, which
suits CI and shared machines. In scripts:

```bash
printf '%s\n' "$OPENAI_API_KEY" | cargo rullst ai connect --provider openai --api-key-stdin
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
| `cargo rullst make:*`, `generate:*` (not `generate:models`), `db:status`, `doctor`, `audit`, `inspect` | `deploy`, `foundry:*`, `upgrade`, `update`, `pkg`, `db:migrate`, `doctor --fix`, `audit --network`, shell commands |
| `cargo check`, `cargo test` (simple flags only) | `cargo run`, `cargo install`, `--manifest-path`, `--config`, `-Z` |

Without an interactive terminal (piped input, `CI` set or `TERM=dumb`) or with
`--dry-run`, proposed actions are printed as a plan and never executed.

## 5. Undo

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

## 6. Safety notes

- Everything that comes from the project, shared files or command output is
  sent as delimited, size-capped untrusted data and checked by the `rullst-ai`
  guardrails; matching content is withheld. Markdown image syntax is broken up
  before sending (`vec![` becomes `vec! [`), because the guardrails treat image
  links as data-exfiltration beacons.
- Model output is printed with terminal control characters escaped, so an
  answer cannot rewrite your screen or clipboard.
- Review every diff: an edit to `build.rs`, `Cargo.toml` or a test runs code on
  the next `cargo check` or `cargo test`. Such files are flagged in the review.
- The CLI shows the answer size and time after each reply. The current
  `rullst-ai` transports do not report token usage, so no token count or cost
  estimate is displayed; check your provider's dashboard for billing.
- An OS keyring is not used; the credentials file is protected by file
  permissions only, like Cargo's own `credentials.toml`.
