### Changed

- Changelog entries are now one file per change under `changelog.d/` instead
  of lines under `## [Unreleased]` in `CHANGELOG.md`, so parallel pull
  requests no longer conflict there. Release preparation merges them with
  `.github/assemble-changelog.py --release <version> --date <date>`, a release
  is not admitted while unmerged fragments remain, and a pull-request check
  refuses new lines under `## [Unreleased]`. See `changelog.d/README.md`.
