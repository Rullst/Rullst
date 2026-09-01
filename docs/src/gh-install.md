# GitHub CLI installation and safe login

The GitHub CLI (`gh`) lets a maintainer inspect workflow runs, pull requests,
issues, Dependabot state, and Code Scanning alerts from a terminal. It does not
gain access merely by being installed: access begins only after the maintainer
completes GitHub's browser authorization flow.

Official references:

- [GitHub CLI installation](https://github.com/cli/cli#installation)
- [`gh auth login` manual](https://cli.github.com/manual/gh_auth_login)
- [Code Scanning API permissions](https://docs.github.com/en/rest/code-scanning/code-scanning)

## Current Rullst workstation

On the Linux workstation used for the v12 release work, `gh` was installed for
the current user at `~/.local/bin/gh`. The downloaded GitHub release archive was
verified against its official SHA-256 checksum. Confirm that the command is on
the shell path:

```bash
command -v gh
gh --version
```

If the first command prints nothing, start a new terminal. For the current
terminal only, this adds the user-local directory without changing system
files:

```bash
export PATH="${PATH}:$HOME/.local/bin"
```

For another machine, use GitHub's current official installation instructions.
Common package-manager entry points are:

```bash
# macOS with Homebrew
brew install gh

# Windows with WinGet
winget install --id GitHub.cli
```

Linux repository commands vary by distribution and can change; copy them from
the [official Linux installation guide](https://github.com/cli/cli/blob/trunk/docs/install_linux.md)
instead of an old blog post.

## Browser login for Rullst maintenance

Run this command yourself in the terminal:

```bash
gh auth login --hostname github.com --git-protocol https --web --scopes security_events
```

GitHub will show a one-time code and open its authorization page. Verify that
the browser is on `github.com`, sign in as the intended Rullst maintainer, read
the requested permissions, and approve only if they match the task. The
additional `security_events` scope allows the CLI to query Code Scanning data;
GitHub CLI's web flow also maintains its documented baseline scopes.

Do not paste an access token into chat, a repository file, shell history, an
issue, or a commit. Do not use `--insecure-storage`. The browser flow asks the
system credential store to keep the credential; if `gh` reports that no secure
credential store is available, stop and configure one before continuing.

Verify the active account without printing its token:

```bash
gh auth status --hostname github.com --active
```

Never add `--show-token` to a command whose output may be shared. Once the
status is healthy, a read-only check of Rullst's open Code Scanning alerts is:

```bash
gh api 'repos/Rullst/Rullst/code-scanning/alerts?state=open&per_page=100' \
  --jq '.[] | {number, rule: .rule.id, severity: .rule.security_severity_level, url: .html_url}'
```

Authentication makes inspection possible; it does not authorize dismissing an
alert, merging a pull request, changing repository settings, publishing a
release, or modifying secrets. Those actions still require an explicit task
and an evidence-based review.

## Logout and revocation

Remove the local GitHub CLI session with:

```bash
gh auth logout --hostname github.com
```

The logout command removes the locally stored authentication entry but does
not revoke the OAuth grant. To revoke it, open
[GitHub authorized applications](https://github.com/settings/applications),
select **GitHub CLI**, review the impact on other machines, and choose
**Revoke Access**.

After logout, confirm that this workstation no longer has an active session:

```bash
gh auth status --hostname github.com
```

