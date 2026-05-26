# Introduction to RullstPress: The Native SSG

**RullstPress** is a lightning-fast, zero-dependency **Static Site Generator (SSG)** built directly into the Rullst CLI (`cargo-rullst`). It compiles standard Markdown files (`.md`) into highly-optimized, premium, SEO-friendly HTML files.

---

## 🌟 What is RullstPress and What is it For?

A common misconception is that RullstPress is *only* for writing framework documentation. In reality, RullstPress is a **general-purpose high-fidelity publishing engine**. 

You can use it to build:
1. **Technical Documentation & Wikis**: Leverage the automatically generated, responsive sidebar navigation that structure your guides and specifications.
2. **SaaS & Product Landing Pages**: Use the special `index.md` template to render beautiful hero headers, call-to-action buttons, and gorgeous interactive grids without writing a single line of CSS.
3. **Personal Portfolios & Content Sites**: Fast, lightweight personal pages that showcase your projects with premium typography and glassmorphic elements.
4. **Blogs & News Platforms**: Organize articles into folders and generate clean, readable content sheets with beautiful syntax highlighting.

### Why use RullstPress?
* **Zero JS Overhead**: Generates pure static HTML and CSS. Pages load in microseconds with a guaranteed 100/100 Google PageSpeed score.
* **First-Class SEO & Meta-Tags**: Automatically injects semantic metadata, OpenGraph previews (Facebook/LinkedIn), and Twitter cards.
* **No Extra Tooling**: You don't need `npm`, Node.js, Ruby, or external CLI binaries. If you have Rust and the Rullst CLI, you are ready to compile.

---

## 📂 1. Folder Structure

To start building a static website or documentation hub, simply create a `docs/` folder at the root of your project:

```text
my_project/
├── docs/
│   ├── index.md           # The Home Page (special Hero & Feature layout)
│   ├── 1-getting-started.md # A guide (number prefix determines sidebar order)
│   ├── 2-architecture.md  # A guide about architecture
│   └──pt/                 # For multi-language translations (optional)
│       ├── index.md
│       └── 1-getting-started.md
├── src/                   # Your main Rust source code
├── Cargo.toml
└── Rullst.toml
```

> [!TIP]
> **Sidebar Ordering**: By default, RullstPress sorts pages alphabetically. You can use numeric prefixes like `1-getting-started.md` to force a specific reading order. The prefixes are automatically stripped from the rendered titles!

---

## 💻 2. Local Development Server

To preview your website in real-time as you write and edit Markdown files, open your terminal and run:

```bash
cargo rullst docs dev
```

### What happens under the hood?
1. RullstPress scans your `docs/` directory.
2. It compiles all `.md` files into optimized HTML.
3. It boots a high-performance local server at `http://localhost:4000`.
4. It copies static assets like images (`Rullst.png`) to the output path automatically.

Open your browser to `http://localhost:4000` to preview. The routing matches your file structure exactly:
* `docs/1-getting-started.md` → `http://localhost:4000/1-getting-started.html`
* `docs/pt/1-getting-started.md` → `http://localhost:4000/pt/1-getting-started.html`

---

## 🚀 3. The Special Landing Page (`index.md`)

RullstPress implements a **Landing Page Mode** specifically for the root `index.md` file. 

Instead of rendering a classic reading layout with a sidebar, the root `index.md` generates a **SaaS-style Hero page**. It automatically structures:
* A beautiful centered visual logo.
* A large glowing headline title with smooth gradient text.
* A descriptive subtitle.
* A prominent call-to-action button pulsing with elegant animations.
* A beautiful 3-column features card grid.

This layout is automatically rendered when RullstPress detects the `index.md` name, creating a spectacular first impression for your visitors.

---

## 📦 4. Production Build

When you are ready to deploy your site to production hosting (such as GitHub Pages, Netlify, Vercel, or a simple Apache/Nginx web server), run:

```bash
cargo rullst docs build
```

This compiles your entire static site with maximum production optimizations and deposits all final assets into the `docs/dist/` directory. 

To publish, simply upload the contents of `docs/dist/` to your static hosting provider. It's completely self-contained!
