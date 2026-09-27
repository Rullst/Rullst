# Rullst governance

## Roles and decisions

As of 27 September 2026, [venelouis](https://github.com/venelouis) is the sole
maintainer and release decision maker. The maintainer sets compatibility and
maintenance scope, accepts contributions, administers repository protections,
coordinates vulnerability reports and authorizes releases. No independent
reviewer or succession maintainer is currently appointed.

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

The approval count is explicitly zero while there is only one maintainer.
Introducing a mandatory independent approval requires an available, authorized
reviewer and a documented transition; it must not silently block maintenance.

Passing PR checks permits source integration. Publication additionally requires
the exact-source release admission, package/native artifact verification,
attestations, ownership checks and protected registry approval. No score or
badge substitutes for those controls. Security handling and support obligations
remain defined in SECURITY.md; workflow and package records retain release
evidence.

## Continuity: outstanding operational requirement

The project does not yet claim continuity if the sole maintainer becomes
unavailable. Establishing continuity needs an actual arrangement approved by
the maintainer. A trusted successor or recovery custodian needs appropriate
access and authority, plus verified recovery of issue/PR administration, releases,
registry ownership and required domains. A Silver badge application is deferred
until this operational dependency is resolved.
Keep credentials and recovery instructions outside the public repository.

Record only the existence, scope and last verification date of an established
arrangement publicly. Until it exists and is verified, continuity remains an
open requirement. Additional maintainers and independent review are welcome,
but no role or credential access is granted by contributing a pull request.
