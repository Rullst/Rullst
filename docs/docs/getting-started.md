# Getting Started

Welcome to Rullst! The framework designed to give you the developer experience of Laravel with the extreme performance of Rust.

## Installation

First, make sure you have Rust installed. Then, install the `cargo-rullst` CLI:

```bash
cargo install --path cargo-rullst --force
```

*(Note: Once published to crates.io, you will be able to run `cargo install cargo-rullst`)*

## Creating your first project

Rullst comes with an interactive CLI to scaffold your project and select built-in blueprints:

```bash
cargo rullst new my_app
```

Select your template (e.g., SAAS, Portfolio, LMS, Blog) or start from a Blank Template.

## Running the Development Server

Navigate into your new project and start the dev server:

```bash
cd my_app
cargo rullst dev
```

This starts the AST-based hot-reloading server. Whenever you save a file, Rullst instantly pushes HTML layout updates to the browser via WebSockets, giving you sub-millisecond feedback loops!
