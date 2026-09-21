# Generated project instructions and context

The v13 CLI candidate generates a bounded project inventory and creates an
application `AGENTS.md` when none exists. It preserves existing instructions.
This is a source-navigation aid; the map does not establish that a feature is
implemented, a test passed, a provider is configured or a deployment is secure.

```bash
cargo rullst generate:ai-context
cargo rullst generate:ai-context --check
```

The first command writes `.llms.txt` and `.rullst/context-map.json`. The second
recomputes their expected content without writing and returns a nonzero exit
status if either output is missing, modified or stale. New projects receive the
same inventory and instructions. Existing automatic scaffold hooks refresh the
map; a refresh failure is reported while preserving the completed scaffold.

The versioned `rullst.project-context.v1` map contains the project and dependency
names, normalized dependency requirements, requested dependency features,
declared Cargo feature names,
configuration key names, source-file paths/roles/sizes and an input SHA-256.
Dependency requirements inherited from a workspace or omitted for path/git
sources are identified explicitly. It does not resolve Cargo's complete feature
graph or inspect external workspace members. It scans the selected Cargo root
and its `src` directory; generate a separate map at each application root.

The fingerprint covers Rust source bytes and the metadata actually inventoried.
Configuration **values** and dependency URLs are excluded even from this digest.
Changing only those values therefore does not make the inventory stale. The
fingerprint is an aid to detecting source changes, not a signed attestation.

## Boundaries

| Input or output | Limit or behavior |
| :--- | :--- |
| `Cargo.toml`, optional `Rullst.toml`, optional `.env.example` | At most 256 KiB each; errors do not echo their contents. |
| `.env.example` | Single-line assignments, including quoted values and `export`; only key names are recorded. Multiline declarations fail before continuation content can become a key. No variable interpolation occurs. |
| Rust sources | Regular `.rs` files below `src`; at most 512 files, 1 MiB each and 8 MiB combined. |
| Traversal | At most 4,096 observed entries and 16 nested directories; linked/special files, hidden descendants, `target`, `vendor` and `node_modules` are excluded. A linked source root or configuration file fails. |
| Names | Bounded ASCII names and relative paths; unsupported names fail rather than being silently renamed. |
| Outputs | At most 256 KiB each; linked or unrecognized output is rejected before replacement. |
| Project instructions | Existing `AGENTS.md` is neither read into the map nor replaced. Edit it to add application-specific requirements. |

Source bodies, configuration values, credentials, live `.env` files, database
files, dependency URLs and absolute workstation paths are not copied into the
outputs. The map does not infer route authorization, model relationships or
provider readiness from filenames. Read the source before changing behavior.
The workspace must be trusted against concurrent adversarial filesystem edits;
ordinary replacement uses the CLI's preflight and atomic rollback helpers.

## Existing applications

Regeneration recognizes the prior CLI's generated `.llms.txt` header and replaces
that source-concatenation format with the inventory. A legacy file larger than
the output budget or an unrecognized manually maintained file is refused; move
it aside yourself after reviewing what it contains. Regeneration updates only
recognized generated outputs and never replaces project instructions.

Current local fixtures exercise deterministic output, stale checks, malformed
configuration, private-value omission, source limits, links, output conflicts,
legacy migration and real project/scaffold commands. Combined hosted and
installed-archive acceptance passed in
[PR #221](https://github.com/Rullst/Rullst/pull/221); final release evidence is
tracked separately in the [v13 delivery plan](v13-delivery-plan.md).
