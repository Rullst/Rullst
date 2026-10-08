# Successor maintainer checklist

This checklist turns the continuity requirement in
[GOVERNANCE.md](../GOVERNANCE.md) into concrete steps. The goal: if the
maintainer becomes unavailable, a trusted successor can still create and close
issues, accept changes and publish releases within a week. OpenSSF Best
Practices Silver asks for the same capability.

**Status:** in progress. The organization Owner invitation was sent on
8 October 2026 and awaits acceptance; the other steps are pending. Change this
line to "established", and update GOVERNANCE.md, only after the verification
drill in section 8 passes.

This file is public. Never add passwords, recovery codes, tokens, phone
numbers or personal contact details to it, or to any issue, pull request or
chat. Those belong in the private recovery document (section 6).

## 1. The successor's role

- The successor is not expected to maintain Rullst day to day. They hold
  enough access for the project to continue, and they act only if the
  maintainer becomes unavailable or asks them to.
- They take part in a short verification drill once a year (section 8).
- Reviewing pull requests is welcome but optional.
- Extra accounts held by the maintainer do not count: continuity needs a
  different person.

## 2. Successor: prepare your accounts

### 2.1 GitHub with two-factor authentication

1. Sign in to GitHub and open Settings → Password and authentication
   (`https://github.com/settings/security`).
2. Choose "Enable two-factor authentication" and pick an authenticator app
   (scan the QR code). Use SMS only if nothing else works.
3. Download the recovery codes and store them in your password manager, not
   on the same phone as the authenticator.
4. Optional: add a passkey or a security key as a second method.
5. Send the maintainer your GitHub **login** (the username, not the email).

### 2.2 crates.io

1. Open `https://crates.io` and choose "Log in with GitHub". This creates the
   crates.io account; owner invitations cannot be accepted before it exists.
2. In crates.io Account Settings, add and verify an email address. Publishing
   requires a verified email.

### 2.3 Password manager

Use a password manager you control. If the maintainer will share a vault with
you, pick one that supports shared vaults or emergency access (Bitwarden and
1Password both offer it; check which plan includes it).

## 3. Maintainer: grant access

State recorded on 8 October 2026.

| Asset | Today | Action | How to check |
| --- | --- | --- | --- |
| GitHub organization `Rullst` | Owner invitation sent on 8 October 2026, awaiting acceptance | Invite the successor as **Owner** (3.1) | The successor sees the organization Settings tab |
| Repository `Rullst/Rullst`, Pages, security advisories | Administered by organization owners | Covered by the Owner role | The successor opens repository Settings → Rules and Security → Advisories |
| `crates-io` release environment | Only the maintainer is a required reviewer | Add the successor as a required reviewer (3.2) | Both names appear under required reviewers |
| crates.io ownership | The maintainer is the only owner of all 16 published crates | Add the successor as an owner of each crate (3.3) | `cargo owner --list <crate>` shows both owners |
| crates.io Trusted Publishing | Releases publish from `release.yml` through Trusted Publishing; no registry token is stored in the repository | Nothing extra: crate owners can view and change this setting | The successor sees the Trusted Publishing settings of a crate |
| Private vulnerability reports on GitHub | Enabled | Covered by the Owner role | The successor sees Security → Advisories |
| Security inbox `officialrullst@gmail.com` (named in SECURITY.md) | Only the maintainer can sign in | Set up recovery (3.4) | Depends on the option chosen |
| OpenSSF Best Practices entry | Edited by the maintainer | Grant the successor edit rights on the project page | The successor can open the edit form |
| Maintainer's personal GitHub account | No successor set | Optional (3.5) | The successor accepts the invitation |
| Domains | None; the book is served from `rullst.github.io` | Add a row here if a domain is ever registered | — |

### 3.1 Organization owner

1. Open the organization Settings → People → "Invite member".
2. Enter the successor's GitHub login and choose the **Owner** role.
3. The successor accepts from the invitation email or at
   `https://github.com/orgs/Rullst/invitation`.

### 3.2 Release approval

1. Open repository Settings → Environments → `crates-io`.
2. Under "Required reviewers", add the successor and save. An approval from
   any listed reviewer is enough.
3. During a release, GitHub notifies the reviewers when the publishing job
   waits. To approve, open that workflow run and choose "Review deployments".

### 3.3 crates.io ownership

The maintainer needs a short-lived crates.io API token:

1. crates.io → Account Settings → API Tokens → "New Token".
2. Give it a name such as "add successor owner", the shortest expiration on
   offer, and only the `change-owners` scope. If the form accepts crate name
   patterns, limit it to the Rullst crates.
3. Run `cargo login` and paste the token.
4. Run the loop below, replacing `SUCCESSOR_LOGIN` with the successor's GitHub
   login.
5. Run `cargo logout`, then revoke the token on crates.io.

```bash
for crate in rullst-macros rullst-orm-macros rullst-orm rullst-core \
  rullst-messaging rullst-connect rullst-iot rullst-security rullst-ai \
  rullst-capital rullst-mail rullst-auth rullst-nexus rullst-studio \
  rullst cargo-rullst; do
  cargo owner --add SUCCESSOR_LOGIN "$crate"
  sleep 2
done
```

The successor then accepts each invitation at
`https://crates.io/me/pending-invites`. Invitations expire, so accept them
promptly.

Repeat this for every crate published later, such as `rullst-privacy`,
`rullst-supervision`, `rullst-media` and `rullst-labs` after their first
release. `.github/release-order.json` lists the publishable packages.

Add the successor as an individual owner. A GitHub team owner can publish but
cannot manage owners, which defeats the purpose.

### 3.4 Security inbox recovery

Security reports arrive through two channels. GitHub private vulnerability
reports are already covered by the Owner role. The email inbox needs its own
recovery. Recommended combination:

1. Store the account password and the Google backup codes (Google Account →
   Security → 2-Step Verification → Backup codes) in a vault shared with the
   successor, or in one with emergency access.
2. Set an address the successor controls as the account's recovery email
   (Google Account → Security → Recovery email).

Google's Inactive Account Manager is not enough on its own. It waits at least
three months, then lets trusted contacts download data rather than keep using
the inbox. That is too slow for the one-week goal; use it only as an extra
safety net.

### 3.5 Personal account successor (optional)

On the maintainer's personal GitHub account: Settings → Account → "Successor
settings". This only covers repositories owned by the personal account, not
the `Rullst` organization, which section 3.1 covers.

## 4. Optional hardening: require 2FA in the organization

Once every member has 2FA:

1. Check organization Settings → People and filter by 2FA status.
2. Open Settings → Authentication security and enable "Require two-factor
   authentication".

GitHub removes members and outside collaborators who do not have 2FA when this
is switched on, so do step 1 first.

## 5. If the maintainer becomes unavailable

A short playbook for the successor. It contains no secrets.

1. Confirm the situation with the family before acting.
2. Open the private recovery document (section 6).
3. Pin an issue in `Rullst/Rullst` saying who is looking after the project and
   what to expect.
4. Read the private vulnerability reports (Security → Advisories) and the
   security inbox, and follow [SECURITY.md](../SECURITY.md).
5. Review open pull requests and Dependabot alerts. Merge only changes whose
   checks pass.
6. If a fix must ship, run the release workflow and approve it in the
   `crates-io` environment. The [release recovery guide](src/release-recovery.md)
   covers a release that stops halfway.
7. Within a few weeks, decide: continue, look for more maintainers, or declare
   maintenance mode. Archiving the repository keeps everything readable.

## 6. Private recovery document

This is the "what to do if I disappear" note. It lives **outside** the
repository, in one of two places:

- a vault in a password manager shared with the successor, or set up with
  emergency access;
- a printed copy in a sealed envelope that the successor can reach.

Never commit it, paste it into an issue or chat, or give it to an AI tool.
Update it whenever an account, password or recovery method changes.

Suggested outline:

1. **Accounts:** GitHub, crates.io, the security inbox and the Best Practices
   entry, with where each password and recovery code is kept.
2. **Two-factor recovery:** how to get back into each account if a phone is
   lost.
3. **Releasing:** the release workflow, `.github/release-order.json`, and the
   approval step in the `crates-io` environment.
4. **First steps:** the playbook in section 5.
5. **Support contacts:** GitHub Support, crates.io help (`help@crates.io`),
   Google account recovery.
6. **Last updated:** the date.

## 7. What to tell the successor

Pass on this file, the private recovery document, and the location of the
vault or envelope. Explain the role (section 1), and agree on the date of the
first drill.

## 8. Verification drill

Run this once after granting access, then yearly and after any account change.
It publishes nothing.

1. The successor signs in to GitHub with 2FA and opens the organization
   Settings.
2. They see themselves as a required reviewer of the `crates-io` environment.
3. `cargo owner --list` lists them for every published crate.
4. They open a test issue in `Rullst/Rullst`, label it and close it.
5. They confirm the security inbox recovery works, for example by opening the
   shared vault entry and seeing their address as the recovery email.
6. They read the private recovery document and confirm they could run a
   release from it.
7. The maintainer writes down the date.

## 9. Public record

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

## 10. What it changes, honestly

- **OpenSSF Best Practices Silver:** this resolves the documented blocker
  (access continuity). Silver has other criteria still to check, such as test
  coverage, an assurance case and signed releases.
- **OpenSSF Scorecard:** CII-Best-Practices moves from 5 to 7 only if Silver is
  granted. Code-Review rises only when the successor really reviews and
  approves pull requests. Making that approval mandatory would block routine
  maintenance, so it stays optional.
- **Bus factor:** access alone does not transfer knowledge. Report the bus
  factor honestly until the successor actually contributes.
