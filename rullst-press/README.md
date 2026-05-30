# RullstPress 🦀

[![Crates.io](https://img.shields.io/crates/v/rullst-press.svg)](https://crates.io/crates/rullst-press)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

> **The blazingly fast, native Rust Static Site Generator for premium documentation.**

RullstPress is the "VitePress of the Rust ecosystem" — a CLI tool that reads your `docs/**/*.md` Markdown files and generates a stunning, dark-mode, responsive HTML documentation site with zero JavaScript dependencies.

## Features

- ⚡ **Blazing Fast** — Pure Rust, zero Node.js. Compiles your entire docs in milliseconds.
- 🌑 **Premium Dark Mode** — A stunning, glassmorphism-inspired dark theme out-of-the-box.
- 📋 **Copy-to-Clipboard** — Automatically adds a "Copy" button to every code block.
- 📱 **Fully Responsive** — Mobile-first, collapsible sidebar.
- 🚀 **GitHub Pages Ready** — Output goes to `docs/dist/`, perfect for CI/CD deployment.
- 🔧 **Built-in Dev Server** — Run a local server to preview your docs live.

## Installation

```bash
cargo install rullst-press
```

## Usage

Inside a project with a `docs/` folder containing `.md` files:

```bash
# Build the static site into docs/dist/
rullst-press build

# Start a local dev server at http://localhost:4000
rullst-press dev
```

## Project Structure

```
my-project/
├── docs/
│   ├── index.md          # Homepage (uses a special hero layout)
│   ├── 1-getting-started.md
│   ├── 2-advanced.md
│   └── dist/             # Generated HTML output (auto-created)
└── Cargo.toml
```

## GitHub Pages CI/CD

Add this to `.github/workflows/deploy-docs.yml`:

```yaml
- name: Install RullstPress
  run: cargo install rullst-press

- name: Build Docs
  run: rullst-press build

- name: Deploy to GitHub Pages
  uses: peaceiris/actions-gh-pages@v3
  with:
    github_token: ${{ secrets.GITHUB_TOKEN }}
    publish_dir: ./docs/dist
```

## License

MIT — Made with ❤️ by the [Rullst](https://github.com/venelouis/Rullst) team.
