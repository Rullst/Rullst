### Fixed

- **`cargo rullst upgrade`:** a rule that matches several lines of one file
  is reported at each of them. A review rule in Rust sources, a script or
  workflow rule and the project-file rules (`.env`, `Dockerfile`,
  `.cargo/config.toml`, `.gitignore`, Kubernetes YAML) reported only the
  first, so an application's `src/ai.rs` showed one of its three
  `V13-AI-HTTP-STATUS` uses. The plan and the Markdown report list a rule's
  lines in one file on one entry in line order (`src/ai.rs:12, 40, 77`);
  `--json` has one finding per location, sorted by file, line and code, and
  the counts count locations. A `rullst-upgrade: reviewed` marker still
  leaves out only its own location.
