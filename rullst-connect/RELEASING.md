# Releasing `rullst-connect`

This repository already has the release automation in place:

- `cargo-release` prepares the version bump, updates `README.md`, and inserts the new entry in `CHANGELOG.md`.
- `.github/workflows/publish.yml` publishes the crate to crates.io when a `vX.Y.Z` tag is pushed.
- The same workflow can also be started manually from GitHub Actions.

## Prerequisites

Before releasing, make sure the repository has these secrets configured in GitHub:

- `CARGO_REGISTRY_TOKEN` for crates.io publishing.

## Normal release flow

1. Make sure `CHANGELOG.md` has the current work under `## [Unreleased]`.
2. Run the release command locally:

```bash
cargo release patch --execute
```

3. Use `minor` or `major` instead of `patch` when the next version requires it.
4. Push the generated commit and tag to GitHub.
5. GitHub Actions will pick up the tag and publish the crate to crates.io.

## If you need a manual publish

1. Open the repository Actions tab.
2. Run the `Publish to Crates.io` workflow manually.
3. Confirm the workflow finishes successfully before announcing the release.

## Preparing the next release

After a release is published, keep new work in `CHANGELOG.md` under `## [Unreleased]`.
`cargo-release` will create the next dated section automatically the next time you run it.
