# Tutorial 01: From Zero to Hello Rullst 🚀

**What you will build:** one complete web application with a typed route and
server-rendered HTML. No database account or AI provider is needed. By the end,
you will be able to point to the handler that produced the page in your browser.

[Choose a different starting path](../start-here.md) ·
[Next: CLI generators](02-cli-generators.md)

This tutorial takes a new developer from installing Rust to a running Rullst
web application. It targets the v13 prerelease `13.0.0-alpha.4`. The v12 line
reached end of life on 16 October 2026 (see the [release record](../v12.md));
existing v12 applications follow the [v13 migration guide](../migration-v13.md).

## 1. Install Rust and Cargo

On Linux or macOS, use the official rustup installer:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

On Windows, download rustup from [rustup.rs](https://rustup.rs) or run:

```powershell
winget install --id Rustlang.Rustup
```

Restart the terminal if necessary, then verify the toolchain:

<!-- first-hour: toolchain -->
```bash
rustc --version
cargo --version
```

Rullst v13 requires Rust 1.96.0 or newer, the MSRV recorded in the
[compatibility policy](../compatibility-policy.md). Run `rustup update` if
`rustc --version` reports an older release.

## 2. Create a project

<!-- first-hour: create -->
```bash
cargo new my_first_app
cd my_first_app
```

Every command below must run in this directory, where `Cargo.toml` lives.

## 3. Add Rullst

Add the v13 prerelease from crates.io. Name its exact version: Cargo never
selects a prerelease on its own, and the v12 line reached end of life on
16 October 2026, so a plain `cargo add rullst` would pick an unmaintained 12.x:

<!-- first-hour: add -->
```bash
cargo add rullst@=13.0.0-alpha.4
cargo add tokio --features full
```

Cargo records the resolved dependency graph in `Cargo.lock`. Commit that file
for an application so its checkout remains reproducible, and review future
dependency updates before deployment.

## 4. Define the first route

Replace `src/main.rs` with:

<!-- first-hour: main write=src/main.rs -->
```rust,no_run
use rullst::{html, routes, Markup, Server};

async fn home() -> Markup {
    html! {
        <div class="min-h-screen bg-slate-900 text-emerald-400 flex flex-col items-center justify-center font-sans">
            <h1 class="text-5xl font-extrabold mb-4">"Hello, Rullst! 📜🦀"</h1>
            <p class="text-slate-400 text-lg">"Your first typed route is running."</p>
        </div>
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let app = routes![
        get("/" => home)
    ];

    Server::new(app).run(3000).await?;
    Ok(())
}
```

The fallible server startup uses `?`, so bind and runtime failures are returned
to the process instead of causing a framework-originated panic.

## 5. Run the application

Start the server:

<!-- first-hour: run -->
```bash
cargo run
```

The first build compiles every dependency and can take several minutes
([Build times](../build-times.md)). When the terminal prints
`Rullst framework serving on http://127.0.0.1:3000`, open
[http://127.0.0.1:3000](http://127.0.0.1:3000). Stop the server with `Ctrl+C`.

**Make it yours:** change the heading to your project name, restart with
`cargo run`, and confirm the response changed. The example's utility class
names do not install Tailwind by themselves; seeing an unstyled page is not a
server failure. The Blank and SaaS starters of the CLI ship a local stylesheet
(`static/rullst.css`).

If the port is already occupied, stop the conflicting process you own or start
on another port with `PORT=3001 cargo run`: the `PORT` variable (from the
environment, then `.env`) takes precedence over the `.run(3000)` argument. Keep
the terminal output: it is the first place to look for a build or startup
diagnostic.

## 6. Continue with the CLI

The CLI can generate complete starters and project modules. Install the v13
prerelease from crates.io (name the exact version; Cargo never picks a
prerelease on its own):

<!-- first-hour: cli -->
```bash
cargo install cargo-rullst --version =13.0.0-alpha.4 --locked
cargo rullst --version
```

The second command prints `rullst 13.0.0-alpha.4`. [Tutorial 02](02-cli-generators.md)
creates a project with it; the hand-written `my_first_app` is not needed there.

The CLI's `new` generator targets the CLI's own framework version. A CLI built
from a source checkout of `main` is the v13 development line: it emits absolute
path dependencies to that exact checkout, including when invoked elsewhere.
Keep such a checkout in place and review its sources before sharing a generated
project. See the [CLI reference](../cli_reference.md) for every command and
boundary.

## Key takeaways

- The `rullst::html!` macro generates ordinary Rust string-building code and
  escapes dynamic values. It returns `Markup`, which a handler returns as
  `text/html` and another `html!` inserts as HTML. Rendering still performs
  the allocations/work implied by the generated template.
- Boolean HTML attributes inside `html!` are written bare (`required`) or
  conditionally (`checked?={is_done}` writes `checked` or nothing). Presence is
  the value, so `disabled="false"` would still disable the control.
- Fallible handlers can return `Result<Response, YourAppError>` using an
  application-defined error that converts the relevant typed framework/domain
  errors; server startup propagates `ServerError` with `?`.
- The moving `main` branch is an integration source, not an immutable release
  artifact. For a reproducible application, pin the exact crates.io version
  (`=13.0.0-alpha.4`) and commit `Cargo.lock`.
