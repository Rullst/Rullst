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
