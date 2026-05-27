# 📝 RullstPress: The Native Static Site Generator

**RullstPress** is a lightning-fast, zero-dependency **Static Site Generator (SSG)** built directly into the Rullst CLI (`cargo-rullst`). It compiles standard Markdown files (`.md`) into highly-optimized, premium, SEO-friendly HTML files.

## 🌟 What can I build with it?

A common misconception is that RullstPress is *only* for writing framework documentation. In reality, RullstPress is a **general-purpose high-fidelity publishing engine**. 

You can use it to build:
1. **Technical Documentation & Wikis**: Leverage the automatically generated, responsive sidebar navigation.
2. **SaaS & Product Landing Pages**: Use the special `index.md` template to render beautiful hero headers and gorgeous interactive grids without writing CSS.
3. **Personal Portfolios & Content Sites**: Fast, lightweight personal pages showcasing your projects with premium typography and glassmorphic elements.
4. **Blogs & News Platforms**: Organize articles into folders and generate clean, readable content sheets.

---

## 🛠️ Step 1: Installation

RullstPress is bundled inside the Rullst CLI. If you haven't installed the CLI yet, run:

```bash
cargo install cargo-rullst
```

*(You must have Rust and Cargo installed on your system. If not, visit [rustup.rs](https://rustup.rs/))*

---

## 📂 Step 2: Project Structure

To start building a static website or documentation hub, create a new directory for your project and add a `docs/` folder inside it. 

```bash
mkdir my-awesome-site
cd my-awesome-site
mkdir docs
```

Inside the `docs/` folder, create your markdown files:

```text
my-awesome-site/
└── docs/
    ├── index.md             # The Home Page (special Hero & Feature layout)
    ├── 1-getting-started.md # A guide (number prefix determines sidebar order)
    ├── 2-architecture.md    # Another guide
    └── logo.png             # Your custom logo
```

> [!TIP]
> **Sidebar Ordering**: By default, RullstPress sorts pages alphabetically. Use numeric prefixes like `1-getting-started.md` to force a specific reading order in the sidebar. The prefixes are automatically hidden from the rendered titles!

### The Magic `index.md`

RullstPress has a **Landing Page Mode**. If you name a file `index.md`, instead of rendering a classic reading layout with a sidebar, it generates a **SaaS-style Hero page**. It automatically structures:
* A centered visual logo (looks for an image link in the markdown).
* A large glowing headline title with smooth gradient text.
* A prominent call-to-action button pulsing with elegant animations.

---

## 💻 Step 3: Local Development Server

To preview your website in real-time as you write, run the development server at the root of your project (`my-awesome-site/`):

```bash
cargo rullst docs dev
```

**What happens under the hood?**
1. RullstPress scans your `docs/` directory.
2. It compiles all `.md` files into optimized HTML.
3. It copies your images and static assets to a temporary output folder.
4. It boots a high-performance local server at `http://localhost:4000`.

Open your browser to [http://localhost:4000](http://localhost:4000) to see your site. 
The URLs match your file structure:
* `docs/1-getting-started.md` → `http://localhost:4000/1-getting-started.html`

---

## 📦 Step 4: Production Build

When you are ready to deploy your site, stop the development server (`Ctrl+C`) and run:

```bash
cargo rullst docs build
```

This command compiles your entire static site with maximum production optimizations and deposits all final, minified HTML/CSS assets into the `docs/dist/` directory. 

You can upload the contents of the `docs/dist/` folder to any static hosting provider (Vercel, Netlify, Apache, Nginx).

---

## 🌍 Step 5: Hosting on GitHub Pages (Free)

Hosting your RullstPress site on GitHub Pages is incredibly easy and completely free. 

### 1. Push your code to GitHub
Initialize a git repository, commit your `docs/` folder, and push it to a GitHub repository. 

> [!IMPORTANT]
> Don't forget to push the `docs/dist/` folder! Run `cargo rullst docs build` before pushing so the `dist/` folder has your latest changes.

### 2. Configure GitHub Pages
1. Open your repository on GitHub.
2. Go to the **Settings** tab.
3. On the left sidebar, click on **Pages**.
4. Under **Build and deployment**, set the **Source** to `Deploy from a branch`.
5. Under **Branch**, select your `main` (or `master`) branch, and change the folder dropdown from `/(root)` to `/docs/dist`.
6. Click **Save**.

### 3. Wait a minute
GitHub will start a background action to deploy your site. In about a minute, your website will be live at:
`https://<your-username>.github.io/<your-repo-name>/`

**Congratulations! Your premium, high-speed website is now live!**
