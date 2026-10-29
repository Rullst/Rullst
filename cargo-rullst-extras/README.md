# cargo-rullst-extras

Optional commands for the [Rullst](https://github.com/Rullst/Rullst) CLI,
kept out of `cargo-rullst` so the main CLI stays small. **Tier:**
Experimental (it may change between 13.x releases and is not validated in
every scenario; see the
[maturity tiers](https://rullst.github.io/Rullst/book/maturity.html)).

| Command | What it does |
| :--- | :--- |
| `cargo rullst ai` | A terminal assistant that knows Rullst and your project. It proposes allowlisted `cargo rullst` commands, file edits and `cargo check`/`cargo test`, each previewed and confirmed; without a configured provider a deterministic offline assistant answers. [Guide](https://rullst.github.io/Rullst/book/ai-assistant.html). |
| `cargo rullst green` | Measured efficiency figures for an app: release binary size, a local container image's size (`--image`), idle resident memory, the `Cargo.lock` dependency count and, with `--bench`, CPU time per request. It measures no energy; a carbon figure appears only as an SCI estimate labelled `estimate`, from the power figure and grid intensity you supply. [Guide](https://rullst.github.io/Rullst/book/green.html). |

## Install

Install it with exactly the same version as `cargo-rullst`:

```bash
cargo install cargo-rullst --version =13.0.0-alpha.4 --locked
cargo install cargo-rullst-extras --version =13.0.0-alpha.4 --locked
```

`cargo rullst ai …` and `cargo rullst green …` run the `cargo-rullst-extras`
executable found in an absolute `PATH` directory, the way Cargo finds its own
`cargo-<name>` plugins, with the same arguments and exit status. Without it,
the main CLI prints the install command above and exits with status 1. The
extras also run on their own: `cargo-rullst-extras ai …` or
`cargo rullst-extras green …`.

The extras depend on the `cargo-rullst` library with an exact version
requirement. That library's `extras` module is the seam they use; it is not a
stable API.

## License

MIT
