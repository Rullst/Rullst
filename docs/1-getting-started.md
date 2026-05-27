# Getting Started

Welcome to the Rullst initial tutorial.

## 1. Installation

First, you'll need Rust installed. The official and recommended way is to visit [https://rust-lang.org/tools/install/](https://rust-lang.org/tools/install/).

**For macOS and Linux:**
You can install via `rustup` by running the following command in your terminal:
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

**For Windows:**
Download and run the `rustup-init.exe` from the official website mentioned above. Follow the on-screen instructions.

Next, install the Rullst CLI:

```bash
cargo install cargo-rullst
```

> **Adding to an existing project?**
> If you already have a Rust project and just want to install the framework package without the CLI, you can simply run:
> ```bash
> cargo add rullst
> ```
> This command tells Cargo (Rust's package manager) to download the `rullst` library and add it as a dependency in your `Cargo.toml` file.

## 2. Creating Your First Project

With the CLI installed, you can generate a complete project using our interactive wizard:

```bash
cargo rullst new

# The wizard will prompt you with a few options:
# 🚀 App name? (no spaces allowed) -> my_portfolio
# 🏗️ What would you like to build? -> Full-Stack Web App (SaaS, Portfolio, Blog)
# 🔥 Enable Hot Reloading by default? -> Yes
# 🗄️ Will your project need a Data Base? -> Yes (or No for simple projects)
# 💾 Select a DB Provider -> Sqlite / Postgres / MySQL/MariaDB

cd my_portfolio
cargo run
```

> **Note about Cargo:** The first time you run `cargo run`, Cargo will download and compile all the necessary dependencies (like Tokio, Axum, SQLx). This is completely normal in Rust and might take a few minutes. However, it will cache everything, so the next times you run `cargo run` it will take only a second!

> **Note:** The project name cannot contain spaces (use `my_portfolio` instead of `my portfolio`).

Done! Go to `http://localhost:3000` to see your framework running.

After doing this, you can already put in your portfolio that you've started learning Rullst, the best Full-Stack web framework! :)

---

## 3. Creating a Developer Portfolio

This section will guide you step-by-step through creating a lightning-fast, highly-reactive Developer Portfolio using **Rullst**, **Tailwind CSS**, and **HTMX**.

By the end of this tutorial, you will have a beautiful dark-mode portfolio that renders in under 5 milliseconds!

### Using Tailwind CSS with the `html!` macro

Open your project in your code editor. 
Since this is a Web App, Rullst automatically configured a frontend entry point for you in `src/lib.rs`.

Rullst has a powerful `html!` macro that allows you to write standard HTML5 directly inside Rust. 
It supports dynamic Rust variables natively, escaping them automatically to prevent XSS attacks.

Let's create an incredible Hero Section using Tailwind CSS classes. Replace the `home` function in `src/lib.rs` with this:

```rust
use rullst::{html, response::IntoResponse, htmx::{HtmxRequest, render_page}};

pub async fn home(htmx: HtmxRequest) -> impl IntoResponse {
    let name = "Alex Developer";
    let role = "Full-Stack Rust Engineer";
    let description = "I build blazingly fast and highly scalable applications.";

    let content = html! {
        <div class="flex flex-col items-center justify-center min-h-screen bg-slate-950 text-slate-100 p-6 font-sans selection:bg-sky-500/30">
            
            <div class="max-w-3xl text-center space-y-8 animate-fade-in-up">
                
                <div class="inline-block px-4 py-1.5 rounded-full border border-sky-500/30 bg-sky-500/10 text-sky-400 text-sm font-semibold tracking-wide backdrop-blur-md">
                    "Available for hire"
                </div>

                <h1 class="text-6xl md:text-7xl font-extrabold tracking-tight">
                    "Hi, I'm " 
                    <span class="bg-gradient-to-r from-sky-400 via-indigo-400 to-purple-500 bg-clip-text text-transparent">
                        {name}
                    </span>
                </h1>
                
                <p class="text-2xl font-medium text-slate-300">
                    {role}
                </p>

                <p class="text-lg text-slate-400 max-w-2xl mx-auto leading-relaxed">
                    {description}
                </p>

                <!-- Project Loader Section -->
                <div id="projects-container" class="pt-8">
                    <button hx-get="/projects" 
                            hx-target="#projects-container" 
                            hx-swap="outerHTML" 
                            class="px-8 py-3.5 bg-slate-100 text-slate-900 font-bold rounded-xl shadow-lg shadow-slate-100/20 hover:scale-105 active:scale-95 transition-all duration-200 cursor-pointer">
                        "View My Projects"
                    </button>
                </div>

            </div>
        </div>
    };

    render_page(&htmx, "Alex's Portfolio", content)
}
```

Notice the `<button hx-get="/projects">`! We are using **HTMX** to fetch projects directly from the backend without writing a single line of JavaScript!

### Creating the HTMX Interactive Route

Now, let's create the `/projects` route that will respond to the button click and render a beautiful grid of projects.

Add this new handler below the `home` function in `src/lib.rs`:

```rust
// A simple struct to represent a project
struct Project {
    title: &'static str,
    tech: &'static str,
}

pub async fn load_projects() -> impl IntoResponse {
    let projects = vec![
        Project { title: "Rullst CLI", tech: "Rust, clap" },
        Project { title: "E-Commerce API", tech: "Rust, Axum, PostgreSQL" },
        Project { title: "Social Dashboard", tech: "React, Tailwind, HTMX" },
    ];

    rullst::response::Html(html! {
        <div id="projects-container" class="grid grid-cols-1 md:grid-cols-3 gap-6 pt-8 w-full animate-fade-in">
            { 
                projects.into_iter().map(|p| html! {
                    <div class="group p-6 rounded-2xl bg-slate-900/50 border border-slate-800 hover:border-sky-500/50 transition-all duration-300 hover:-translate-y-1">
                        <h3 class="text-xl font-bold text-slate-200 group-hover:text-sky-400 transition-colors">
                            {p.title}
                        </h3>
                        <p class="text-sm font-mono text-slate-500 mt-2">
                            {p.tech}
                        </p>
                    </div>
                }).collect::<String>()
            }
        </div>
    })
}
```

### Adding the Route

Finally, don't forget to register this new route in your `rullst_router_init` function at the bottom of the file:

```rust
#[unsafe(no_mangle)]
pub extern "C" fn rullst_router_init() -> *mut rullst::Router {
    let router = rullst::routes![
        get("/" => home),
        get("/projects" => load_projects),
    ];
    Box::into_raw(Box::new(router))
}
```

### Run and Enjoy!

If you generated your project with **Hot-Reloading** enabled, your server has already recompiled and updated the routes dynamically! 
Just visit [http://localhost:3000](http://localhost:3000) and click the "View My Projects" button!

You now have an incredibly robust, SEO-friendly, and blazing-fast portfolio powered by Rust!
