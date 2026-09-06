# Tutorial 01: From Zero to Hello Rullst 🚀

**What you will build:** one complete web application with a typed route and
server-rendered HTML. No database account or AI provider is needed. By the end,
you will be able to point to the handler that produced the page in your browser.

[Choose a different starting path](../start-here.md) ·
[Next: CLI generators](02-cli-generators.md)

This tutorial takes a new developer from installing Rust to a running Rullst
web application. It uses the unreleased v12 development snapshot documented by
this site. It is not a production recommendation. A future production adoption
needs a supported release and reviewed immutable artifacts; neither moving
`main` nor merely pinning end-of-life v5 satisfies that requirement.

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

```bash
rustc --version
cargo --version
```

Rullst v12 requires the MSRV recorded in the
[compatibility policy](../compatibility-policy.md).

## 2. Create a project

```bash
cargo new my_first_app
cd my_first_app
```

Every command below must run in this directory, where `Cargo.toml` lives.

## 3. Add the v12 preview

Until v12 is published, select the development source explicitly:

```bash
cargo add rullst --git https://github.com/Rullst/Rullst.git --branch main
cargo add tokio --features full
```

Cargo records the resolved Git commit in `Cargo.lock`. This makes one checkout
repeatable, but a future dependency update can select a newer `main` commit. Do
not use this mutable preview source in production.

Applications that must remain on end-of-life v5 should use its
[versioned API documentation](https://docs.rs/rullst/5.0.0/rullst/) instead;
the v12 API below is intentionally different. That reference preserves the old
API, not a promise of ongoing maintenance or a deployment recommendation.

## 4. Define the first route

Replace `src/main.rs` with:

```rust,no_run
use rullst::{html, response::Html, routes, Server};

async fn home() -> Html<String> {
    Html(html! {
        <div class="min-h-screen bg-slate-900 text-emerald-400 flex flex-col items-center justify-center font-sans">
            <h1 class="text-5xl font-extrabold mb-4">"Hello, Rullst! 📜🦀"</h1>
            <p class="text-slate-400 text-lg">"Your first typed route is running."</p>
        </div>
    })
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

```bash
cargo run
```

Open [http://localhost:3000](http://localhost:3000). Stop it with `Ctrl+C`.

**Make it yours:** change the heading to your project name, restart with
`cargo run`, and confirm the response changed. The example's utility class
names do not install Tailwind by themselves; seeing an unstyled page is not a
server failure. Use a generated HTML blueprint for a bundled local stylesheet.

If the port is already occupied, stop the conflicting process you own or change
the `.run(3000)` port above. Keep the terminal output: it is the first place to
look for a build or startup diagnostic.

## 6. Continue with the CLI

The v12 CLI can generate complete starters and project modules. While working
from a source checkout, install the same revision locally:

```bash
git clone --branch main https://github.com/Rullst/Rullst.git
cd Rullst
cargo install --locked --path cargo-rullst
cargo rullst --help
```

The CLI's `new` generator will target the CLI's framework version. Until v12 is
published, a pre-release CLI built from this checkout emits absolute path
dependencies to that exact checkout, including when invoked elsewhere. Keep the
checkout in place and review those sources before sharing the generated project.
See the [CLI reference](../cli_reference.md) for every command and boundary.

## Key takeaways

- The `rullst::html!` macro generates ordinary Rust string-building code and
  escapes dynamic values. Rendering still performs the allocations/work implied
  by the generated template.
- All boolean HTML attributes inside `html!` must be explicitly quoted (e.g. `required="true"`).
- Fallible handlers can return `Result<Response, YourAppError>` using an
  application-defined error that converts the relevant typed framework/domain
  errors; server startup propagates `ServerError` with `?`.
- The v12 `main` branch is an evaluation source, not a stable release channel.
