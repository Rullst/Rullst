use super::TEXT_MARKER;

pub(super) fn text(json: &str) -> String {
    let mut output = String::from(TEXT_MARKER);
    output.push_str("# Rullst project inventory\n\nThis generated map contains metadata only. Read AGENTS.md and the relevant source before editing. It does not establish feature readiness, deployment state or passing tests.\n\n");
    output.push_str("Regenerate with `cargo rullst generate:ai-context`; verify freshness with `cargo rullst generate:ai-context --check`. Configuration value changes are intentionally excluded from the fingerprint.\n\n");
    output.push_str("```json\n");
    // Serialization of this private, finite data model cannot contain raw code.
    output.push_str(json);
    output.push_str("\n```\n");
    output
}

pub(super) fn instructions() -> &'static str {
    "# Instructions for this Rullst application\n\nThis file belongs to the project maintainer. Rullst will preserve it on regeneration.\n\n- Read `Cargo.toml`, the current project instructions and the source you will change. Use `.rullst/context-map.json` or `.llms.txt` to locate files, not as proof of implemented behavior. Run `cargo rullst generate:ai-context --check` before relying on the inventory; regenerate after changing source.\n- Keep authentication, tenant membership and resource ownership explicit. Derive identity from the application's authenticated context, never from submitted owner or tenant fields.\n- Parameterize SQL. Retain the production CSRF, WAF and secure-header layers. Verify provider signatures and reconcile authoritative state before granting paid features. Offline provider results are development fixtures.\n- Return typed errors from production code; avoid `unwrap`, `expect` and panics outside tests. Keep modules focused and use explicit typed APIs.\n- Inspect the actual enabled dependencies and features. Do not assume a blueprint enables optional age verification, consent, supervision, live payments or legal compliance.\n- Run formatting, relevant tests and strict Clippy for the changed application features. Compilation alone does not establish provider, browser or deployment acceptance.\n- Keep credentials, live `.env` values, keys, database files and user records out of generated context and commits. The inventory records names and paths only; source content and configuration values must be reviewed separately when needed.\n\nThe generated map deliberately excludes source bodies, configuration values, absolute workstation paths, credentials, databases, build output and linked paths. Its input digest is a freshness aid, not a signature.\n"
}
