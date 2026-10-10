# Rullst governance

## Roles and decisions

As of 8 October 2026, [venelouis](https://github.com/venelouis) is the
maintainer and release decision maker. The maintainer sets compatibility and
maintenance scope, accepts contributions, administers repository protections,
coordinates vulnerability reports and authorizes releases.

[stevi001](https://github.com/stevi001) is the successor maintainer. They hold
organization ownership, release approval and registry ownership so the project
can continue if the maintainer becomes unavailable (see [Continuity](#continuity)).
They are not a day-to-day maintainer, and they count as an independent reviewer
only for changes they actually review and approve.

Contributors propose changes through issues and pull requests under
[CONTRIBUTING.md](CONTRIBUTING.md) and the
[Code of Conduct](CODE_OF_CONDUCT.md). Architectural proposals must identify
the applicable specification, compatibility effects, trust boundaries, testing
and ongoing maintenance cost. The maintainer records acceptance, rejection or
deferral in the pull request or maintained roadmap. Unresolved concerns may be
raised in the same discussion; private security reports follow
[SECURITY.md](SECURITY.md).

AI assistants can prepare changes, investigate findings and run verification
within the maintainer's authorized scope. They do not constitute an independent
human reviewer or assume ownership, emergency access or release authority.
Automated checks do not establish independent review.

## Change and release controls

`v12` maintains the compatible stable line; `main` develops v13. Both require
pull requests and the named, application-bound checks in their branch policy.
The public [maintained-branch ruleset](https://github.com/Rullst/Rullst/rules/24080901)
mirrors their classic protections, including all 46 required checks, an up-to-date
base, resolved review conversations, and prevention of force pushes and branch
deletion. It has no bypass actors. The classic protections remain enabled;
future policy changes must keep both representations synchronized until a
separately reviewed migration removes the duplication.

The approval count is explicitly zero while there is only one active maintainer.
Introducing a mandatory independent approval requires an available, authorized
reviewer and a documented transition; it must not silently block maintenance.

Passing PR checks permits source integration. Publication additionally requires
the exact-source release admission, package/native artifact verification,
attestations, ownership checks and protected registry approval. No score or
badge substitutes for those controls. Security handling and support obligations
remain defined in SECURITY.md; workflow and package records retain release
evidence.

## Continuity

A succession arrangement was established on 8 October 2026 with
[stevi001](https://github.com/stevi001) as successor maintainer. Its scope:

- owner of the `Rullst` GitHub organization, with two-factor authentication,
  covering issue and pull-request administration, repository settings,
  GitHub Pages and private vulnerability reports;
- required reviewer of the `crates-io` release environment, so either person
  can approve a publication;
- owner of every published crate on crates.io;
- recovery access to the security inbox named in SECURITY.md and to a private
  recovery document kept outside this repository (confirmed by the maintainer).

Last verified: 8 October 2026. GitHub and crates.io access was checked through
their APIs, and the successor opened and closed a test issue
([#440](https://github.com/Rullst/Rullst/issues/440)). The arrangement is
re-verified every year and after any account change, following the
[successor maintainer checklist](docs/successor.md). The book is served from
`rullst.github.io`. The maintainer holds the project's domains (for example
`rullst.win`, which serves the demo applications); the successor received
recovery access to the domain registrar and the demo hosting on 9 October 2026
(confirmed by the maintainer).

Keep credentials and recovery instructions outside the public repository.
Additional maintainers and independent review are welcome, but no role or
credential access is granted by contributing a pull request.
