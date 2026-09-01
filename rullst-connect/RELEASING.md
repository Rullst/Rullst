# Releasing `rullst-connect`

`rullst-connect` is not an independently versioned release. It is one package
in the synchronized Rullst monorepo train.

1. Follow the root [`RELEASE_GUIDE.md`](../RELEASE_GUIDE.md).
2. Synchronize every publishable package to the exact approved release version.
3. Run the complete release gates on the exact candidate commit.
4. Push only the approved `vX.Y.Z[-pre]` tag.
5. Confirm the tag-only `.github/workflows/release.yml` run verifies, packages,
   attests, and publishes the crates in `.github/release-order.json` order.

Do not run `cargo publish` or `cargo release` from a workstation. The workflow
uses crates.io Trusted Publishing for registered packages; a narrowly scoped,
short-lived bootstrap token is permitted only for a crate name that has never
been registered, as documented in `docs/src/release-recovery.md`.
