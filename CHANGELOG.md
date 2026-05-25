# Changelog 📝

All notable changes to the **Rullst Framework** will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [0.1.1] - 2026-05-25 ✨

### Added
- **AI-Native Engineering & AI-Friendliness** added to core pillars in `README.md` and `README.pt.md`.
- **Master Plan Roadmap update:** Introduced the AI-Native Design Pillar at the top of the development roadmap (`ROADMAP.md` and `ROADMAP.pt.md`).
- **CLI Code Generator:** Added the first code generator subcommand `cargo rullst make:controller <Name>` in `cargo-rullst`.
  - Normalizes controller name inputs (e.g. `UsersController` -> `users_controller`).
  - Scaffolds REST endpoints (`index`, `show`) pre-configured with the JSX-like `html!` macro.
  - Automatically manages mod declarations in `src/controllers/mod.rs` and injects `pub mod controllers;` in `src/main.rs`.
- **CLI Path Normalization:** Normalized workspace and package names when scaffolding projects using path expressions (e.g. `cargo rullst new ..\my_project`).

---

## [0.1.0] - 2026-05-24 🚀

### Added
- **Core Crate (`rullst`):** Wrapped Axum server, routing macro `routes!`, lifecycle DB injection, and response models.
- **Macros Engine (`rullst-macros`):** Built procedural compiler-level `html!` JSX macro with static memory-string concatenation and dynamic XSS protection.
- **Developer CLI (`cargo-rullst`):** Scaffolds complete starter workspaces with integrated sqlite in-memory testing out-of-the-box.
- **Manifestos:** Created rich English (`README.md`) and Portuguese (`README.pt.md`) project overviews.
