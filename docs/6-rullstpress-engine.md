# RullstPress: The SSG Engine

**RullstPress** is a lightning-fast, zero-dependency **Static Site Generator (SSG)** built directly into the Rullst Framework. It compiles standard Markdown files into highly-optimized, premium HTML files with an automatically generated sidebar and responsive layout.

## The Core Concept

RullstPress is designed for everything from simple technical documentation to fully-fledged SaaS landing pages and developer portfolios.

To begin, all you need is a `docs/` folder in the root of your project.

```text
my_project/
└── docs/
    ├── index.md             # The Home Page (Special Hero Layout)
    ├── 1-getting-started.md # Article 1
    └── 2-architecture.md    # Article 2
```

## Running the Engine

Rullst CLI provides built-in commands to work with your documentation.

### Development Server
```bash
cargo rullst docs dev
```
This command instantly compiles your markdown, launches a blazing-fast local server at `http://localhost:4000`, and watches your files for changes. When you save a markdown file, the server will hot-reload your browser!

### Production Build
```bash
cargo rullst docs build
```
This command compiles your site with maximum optimizations and outputs static HTML, CSS, and JS to the `docs/dist/` directory. You can host this `dist` folder directly on GitHub Pages, Vercel, or Netlify.

## The Magic `index.md`

If RullstPress finds an `index.md` file, it activates **Hero Landing Page Mode**. Instead of generating a standard documentation reading layout, it generates a beautiful, responsive landing page featuring:
- A centered visual logo
- Glowing headlines with smooth CSS gradients
- Animated Call-to-Action buttons

You don't need to write any CSS to achieve this—RullstPress automatically injects its premium design system.

## Navigation and Ordering

RullstPress automatically builds a beautiful sidebar navigation menu for you. 

By default, it sorts pages alphabetically. To control the exact order, prefix your markdown filenames with a number (e.g., `1-intro.md`, `2-setup.md`). The generator is smart enough to hide these numbers from the rendered titles in the sidebar!

Enjoy building fast, gorgeous documentation sites without ever leaving the Rust ecosystem!
