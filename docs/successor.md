# Successor maintainer checklist

This checklist turns the continuity requirement in
[GOVERNANCE.md](../GOVERNANCE.md) into concrete steps. The goal: if the
maintainer becomes unavailable, a trusted successor can still create and close
issues, accept changes and publish releases within a week. OpenSSF Best
Practices Silver asks for the same capability.

**Status:** not established. Update this line and GOVERNANCE.md only after the
verification drill in section 5 passes.

This file is public. Never add passwords, recovery codes, tokens, phone
numbers or personal contact details to it, or to any issue, pull request or
chat. Those belong in the private recovery document (section 4).

## 1. What the successor needs first

- A GitHub account with two-factor authentication (2FA). Prefer an
  authenticator app or a security key over SMS. Keep the GitHub recovery codes
  in their own password manager.
- A crates.io account: sign in once at `https://crates.io` with that GitHub
  account. Owner invitations cannot be accepted before this.
- A password manager of their own.

## 2. Access to grant

State recorded on 8 October 2026.

| Asset | Today | Action | How to check |
| --- | --- | --- | --- |
| GitHub organization `Rullst` | Owners are listed under organization Settings → People | Invite the successor with the **Owner** role (organization Settings → People) | The successor sees the organization Settings tab |
| Repository `Rullst/Rullst`, Pages, security advisories | Administered by organization owners | Covered by the Owner role | The successor opens repository Settings → Rules and Security → Advisories |
| `crates-io` release environment | Only the maintainer is a required reviewer | Add the successor as a second required reviewer (repository Settings → Environments → `crates-io`); one approval from either is enough | Both names appear under required reviewers |
| crates.io ownership | The maintainer owns all 16 published crates | Add the successor as an owner of each crate (commands below); the successor accepts at `https://crates.io/me/pending-invites` | `cargo owner --list <crate>` shows both owners |
| crates.io Trusted Publishing | Releases publish from `release.yml` through Trusted Publishing; no registry token is stored in the repository | Nothing extra: crate owners can view and change this setting | The successor sees the Trusted Publishing settings of a crate |
| Security inbox `officialrullst@gmail.com` (named in SECURITY.md) | Only the maintainer can sign in | Choose one recovery path (below) | Depends on the path chosen |
| OpenSSF Best Practices entry | Edited by the maintainer | Grant the successor edit rights on the project page | The successor can open the edit form |
| Maintainer's personal GitHub account | No successor set | Optional: Settings → Account → Successor settings. This covers repositories owned by the personal account only, not the organization | The successor accepts the invitation |
| Domains | None; the book is served from `rullst.github.io` | Add a row here if a domain is ever registered | — |

### Adding crates.io owners

The maintainer needs a crates.io API token with the `change-owners` scope
(crates.io → Account Settings → API Tokens), used once with `cargo login` and
revoked afterwards. Replace `SUCCESSOR_LOGIN` with the successor's GitHub login:

```bash
for crate in rullst-macros rullst-orm-macros rullst-orm rullst-core \
  rullst-messaging rullst-connect rullst-iot rullst-security rullst-ai \
  rullst-capital rullst-mail rullst-auth rullst-nexus rullst-studio \
  rullst cargo-rullst; do
  cargo owner --add SUCCESSOR_LOGIN "$crate"
  sleep 2
done
```

Repeat for every crate published later, such as `rullst-privacy`,
`rullst-supervision`, `rullst-media` and `rullst-labs` after their first
release. `.github/release-order.json` lists the publishable packages.

Add the successor as an individual owner. A GitHub team owner can publish but
cannot manage owners, which defeats the purpose.

### Security inbox recovery: pick one

1. Set an address controlled by the successor as the account's recovery email.
2. Use Google Inactive Account Manager: after a chosen period of inactivity,
   the successor is notified and gets access.
3. Keep the password and the Google backup codes in a shared password-manager
   vault with emergency access.

## 3. Optional hardening once everyone has 2FA

Enable "Require two-factor authentication" in the organization settings.
GitHub removes members without 2FA when this is switched on, so confirm every
member has it first.

## 4. Private recovery document

This is the "what to do if I disappear" note. It lives **outside** the
repository: in the shared password-manager vault, or printed and kept in a
sealed envelope the successor can reach. Never commit it, paste it into an
issue or chat, or give it to an AI tool.

It should contain:

- every account involved (GitHub, crates.io, the security inbox, the Best
  Practices entry), and where each password and recovery code is kept;
- how 2FA recovery works for each account;
- how to cut a release: the public release workflow and
  `.github/release-order.json`, plus the approval step in the `crates-io`
  environment;
- the first steps: pin an issue announcing the change, read any open private
  security reports, then decide between continuing and maintenance mode;
- support contacts: GitHub Support, crates.io help (`help@crates.io`), Google
  account recovery;
- the date it was last updated.

## 5. Verification drill

Run this once after granting access, then yearly and after any account change.
It publishes nothing.

1. The successor signs in to GitHub with 2FA and opens the organization
   Settings.
2. They see themselves as a required reviewer of the `crates-io` environment.
3. `cargo owner --list` lists them for every published crate.
4. They open a test issue in `Rullst/Rullst`, label it and close it.
5. They confirm the security inbox recovery path works, for example the
   recovery email shows as verified, or the emergency vault opens.
6. They read the private recovery document and confirm they could run a
   release from it.
7. The maintainer writes down the date.

## 6. Public record

After the drill passes, replace the "Continuity: outstanding operational
requirement" section of GOVERNANCE.md with a short record. Include only its
existence, scope and dates, for example:

```markdown
## Continuity

A succession arrangement was established on YYYY-MM-DD. Scope: GitHub
organization ownership, release approval, ownership of every published crate
on crates.io, and recovery of the security inbox. Last verified: YYYY-MM-DD.
It is re-verified every year.
```

Naming the successor's GitHub login there is the maintainer's choice. Then
update the CII-Best-Practices row in
[the OpenSSF Scorecard notes](src/openssf-scorecard.md) and change the status
line at the top of this file.

## 7. What it changes, honestly

- **OpenSSF Best Practices Silver:** this resolves the documented blocker
  (access continuity). Silver has other criteria still to check, such as test
  coverage, an assurance case and signed releases.
- **OpenSSF Scorecard:** CII-Best-Practices moves from 5 to 7 only if Silver is
  granted. Code-Review rises only when the successor really reviews and
  approves pull requests. Making that approval mandatory would block routine
  maintenance, so it stays optional.
- **Bus factor:** access alone does not transfer knowledge. Report the bus
  factor honestly until the successor actually contributes.
