#!/usr/bin/env python3
"""Report on the health of Rullst's public dependencies.

The crates in `.github/public-dependencies.json` are part of Rullst's public
API for the whole 13.x line (docs/src/compatibility-policy.md, "Public
dependencies"), so an abandoned one forces a breaking change, as chrono did.
For each crate the report reads:

- crates.io: the newest release of Rullst's line, the latest stable release
  and their dates;
- the rustsec/advisory-db repository: advisories, including informational
  `unmaintained`, `unsound` and `notice` ones, that the newest release of
  Rullst's line does not escape (withdrawn advisories are skipped);
- the GitHub API: whether the crate's repository is archived.

It flags a crate that has had no release for more than `--stale-days` (365),
that has a newer incompatible line (a newer major, or a newer minor of a 0.x
crate), whose newest release in Rullst's line an advisory affects, or whose
GitHub repository is archived or gone.

A flag the maintainer reviewed and decided to keep, listed in
`.github/public-dependency-reviews.json`, is reported as "reviewed until
<date>" and does not count as flagged. A review applies only while it covers
every flag of its crate and has not expired; advisories can never be
reviewed, and a review lasts at most 12 months.

    python3 .github/public-dependency-health.py [--stale-days N] > report.md

The Markdown report goes to stdout, problems to stderr. Its last line is an
HTML comment listing the flags without their dates, which
dependency-health.yml compares between runs. Exit status: 0 when nothing is
flagged, 1 when something is, 2 when a source could not be read. Requests send
the User-Agent `rullst-ci` and nothing else about the caller; GitHub API
requests use GITHUB_TOKEN when it is set; crates.io requests are one per
second. Standard library only. The offline tests are in
`.github/test-public-dependency-health.py`.
"""

from __future__ import annotations

import argparse
import datetime as dt
import io
import json
import os
import re
import sys
import tarfile
import time
import tomllib
import urllib.error
import urllib.parse
import urllib.request
from dataclasses import dataclass, field
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
POLICY_FILE = ROOT / ".github/public-dependencies.json"
REVIEWS_FILE = ROOT / ".github/public-dependency-reviews.json"
USER_AGENT = "rullst-ci"
CRATES_API = "https://crates.io/api/v1/crates/"
GITHUB_API = "https://api.github.com"
ADVISORY_DB = "rustsec/advisory-db"
ADVISORY_URL = "https://rustsec.org/advisories/{}.html"
STALE_DAYS = 365
# Flag kinds a review may cover. Advisories ("advisory") never can.
REVIEWABLE_KINDS = frozenset(
    {"stale", "no-release", "no-line-release", "newer-line", "archived", "no-repository"}
)
CRATES_IO_INTERVAL = 1.0

VERSION = re.compile(r"(\d+)\.(\d+)\.(\d+)(?:-[0-9A-Za-z.-]+)?(?:\+[0-9A-Za-z.-]+)?")
COMPARATOR = re.compile(
    r"(>=|<=|>|<|=|\^|~)?\s*(\d+)(?:\.(\d+|\*|x|X))?(?:\.(\d+|\*|x|X))?"
    r"(?:-[0-9A-Za-z.-]+)?(?:\+[0-9A-Za-z.-]+)?"
)
GITHUB_REPOSITORY = re.compile(
    r"https?://(?:www\.)?github\.com/([A-Za-z0-9_.-]+)/([A-Za-z0-9_.-]+)(?:[/#?].*)?"
)
FRONT_MATTER = re.compile(r"\A\s*```toml[ \t]*\n(.*?)\n```[ \t]*(?:\n(.*))?\Z", re.DOTALL)
ADVISORY_PATH = re.compile(r"[^/]+/crates/([^/]+)/(RUSTSEC-\d{4}-\d{4})\.md")

Version = tuple[int, int, int]


class SourceError(RuntimeError):
    """A source (crates.io, the advisory database or GitHub) could not be read."""


@dataclass(frozen=True)
class Release:
    version: str
    date: str  # YYYY-MM-DD

    @property
    def key(self) -> Version:
        return parse_version(self.version) or (0, 0, 0)


@dataclass(frozen=True)
class Advisory:
    id: str
    package: str
    kind: str  # vulnerability, unmaintained, unsound or notice
    title: str
    patched: tuple[str, ...]
    unaffected: tuple[str, ...]
    withdrawn: bool


@dataclass
class Repository:
    slug: str | None  # owner/name on GitHub, or None
    url: str | None
    archived: bool | None = None
    missing: bool = False
    pushed: str | None = None


@dataclass(frozen=True)
class Flag:
    key: str  # stable between runs, for the fingerprint
    text: str


@dataclass(frozen=True)
class Review:
    """A maintainer's decision to keep a crate despite some kinds of flag."""

    crate: str
    kinds: frozenset[str]
    reason: str
    reviewer: str
    reviewed: dt.date
    expires: dt.date


@dataclass
class Health:
    name: str
    line: str
    newest_in_line: Release | None
    latest: Release | None
    last_released: str | None
    advisories: list[Advisory]
    repository: Repository
    flags: list[Flag] = field(default_factory=list)
    # The review that covers every flag, or why the crate's review does not.
    review: Review | None = None
    review_lapsed: str | None = None

    @property
    def counted(self) -> bool:
        """Whether the crate's flags count toward the health issue."""
        return bool(self.flags) and self.review is None


# --- Versions and requirements -------------------------------------------------


def parse_version(text: str) -> Version | None:
    """A stable `major.minor.patch` version; pre-releases return None."""
    match = VERSION.fullmatch(text.strip())
    if match is None or "-" in text.split("+", 1)[0]:
        return None
    return int(match[1]), int(match[2]), int(match[3])


def parse_line(text: str) -> tuple[int, ...]:
    """A policy line such as "0.4" or "1"."""
    parts = text.strip().split(".")
    if not parts or not all(part.isdigit() for part in parts) or len(parts) > 3:
        raise ValueError(f"not a version line: {text!r}")
    return tuple(int(part) for part in parts)


def line_of(version: Version) -> tuple[int, ...]:
    """The caret line of a version: its components up to the first non-zero one."""
    for index, part in enumerate(version):
        if part != 0:
            return version[: index + 1]
    return version


def show_line(line: tuple[int, ...]) -> str:
    return ".".join(map(str, line))


def bump(parts: list[int]) -> Version:
    """The first version past the given components: 1 -> 2.0.0, 1.2 -> 1.3.0."""
    padded = [*parts, 0, 0][:3]
    padded[len(parts) - 1] += 1
    return padded[0], padded[1], padded[2]


def comparator_bounds(op: str, parts: list[int]) -> tuple[Version | None, Version | None]:
    """The inclusive lower and exclusive upper bound of one comparator."""
    exact = (*parts, 0, 0)[:3]
    if op == ">":
        return bump(parts), None
    if op == ">=":
        return exact, None
    if op == "<":
        return None, exact
    if op == "<=":
        return None, bump(parts)
    if op == "=":
        return exact, bump(parts)
    if op == "~":
        return exact, bump(parts[:2])
    # Caret: compatible within the leftmost non-zero component given.
    significant = next((index for index, part in enumerate(parts) if part), len(parts) - 1)
    return exact, bump(parts[: significant + 1])


def comparator_matches(comparator: str, version: Version) -> bool:
    """Whether a version satisfies one semver comparator (Cargo's rules)."""
    text = comparator.strip()
    if text in {"*", "x", "X"}:
        return True
    match = COMPARATOR.fullmatch(text)
    if match is None:
        raise ValueError(f"not a semver comparator: {comparator!r}")
    parts = [int(part) for part in match.groups()[1:] if part is not None and part.isdigit()]
    lower, upper = comparator_bounds(match[1] or "^", parts)
    return (lower is None or version >= lower) and (upper is None or version < upper)


def requirement_matches(requirement: str, version: Version) -> bool:
    """Whether a version satisfies a comma-separated semver requirement."""
    return all(comparator_matches(part, version) for part in requirement.split(","))


# --- Sources -------------------------------------------------------------------


def releases(crate: dict) -> list[Release]:
    """Stable, non-yanked releases from a crates.io crate response, newest first."""
    found = []
    for entry in crate.get("versions") or []:
        number = str(entry.get("num", ""))
        if entry.get("yanked") or parse_version(number) is None:
            continue
        found.append(Release(number, str(entry.get("created_at", ""))[:10]))
    return sorted(found, key=lambda release: release.key, reverse=True)


def parse_advisory(text: str) -> Advisory:
    """One advisory-db Markdown file: TOML front matter, then a `# Title`."""
    match = FRONT_MATTER.match(text)
    if match is None:
        raise ValueError("no TOML front matter")
    front = tomllib.loads(match[1])
    advisory = front.get("advisory", {})
    versions = front.get("versions", {})
    title = next(
        (line[2:].strip() for line in (match[2] or "").splitlines() if line.startswith("# ")),
        "",
    )
    return Advisory(
        id=str(advisory["id"]),
        package=str(advisory["package"]),
        kind=str(advisory.get("informational") or "vulnerability"),
        title=title,
        patched=tuple(str(item) for item in versions.get("patched", [])),
        unaffected=tuple(str(item) for item in versions.get("unaffected", [])),
        withdrawn="withdrawn" in advisory,
    )


def affects(advisory: Advisory, version: Version) -> bool:
    """RustSec's rule: affected unless patched or unaffected. Unreadable
    requirements count as affected, so they are reported, not hidden."""
    try:
        return not any(
            requirement_matches(requirement, version)
            for requirement in (*advisory.patched, *advisory.unaffected)
        )
    except ValueError:
        return True


def advisories_from_tarball(
    data: bytes, crates: set[str]
) -> tuple[str, dict[str, list[Advisory]], list[str]]:
    """The advisories of `crates` in an advisory-db tarball (read in memory,
    never extracted), its commit and the advisories that could not be read."""
    found: dict[str, list[Advisory]] = {name: [] for name in crates}
    problems: list[str] = []
    with tarfile.open(fileobj=io.BytesIO(data), mode="r:gz") as archive:
        for member in archive:
            match = ADVISORY_PATH.fullmatch(member.name)
            if match is None or match[1] not in crates or not member.isfile():
                continue
            handle = archive.extractfile(member)
            try:
                if handle is None:
                    raise ValueError("not a regular file")
                found[match[1]].append(parse_advisory(handle.read().decode("utf-8")))
            except (ValueError, KeyError) as error:  # UnicodeDecodeError is a ValueError
                problems.append(f"RustSec advisory {match[2]}: {error}")
        # GitHub records the commit in the archive's global pax header.
        revision = str(archive.pax_headers.get("comment", ""))[:12]
    for advisories in found.values():
        advisories.sort(key=lambda advisory: advisory.id)
    return revision, found, problems


def github_slug(url: str | None) -> str | None:
    """`owner/name` of a GitHub repository URL."""
    match = GITHUB_REPOSITORY.fullmatch((url or "").strip())
    if match is None:
        return None
    name = match[2].removesuffix(".git")
    return f"{match[1]}/{name}" if name else None


# --- Evaluation and report -----------------------------------------------------


def evaluate(
    name: str,
    line: str,
    crate: dict,
    advisories: list[Advisory],
    repository: Repository,
    today: dt.date,
    stale_days: int = STALE_DAYS,
) -> Health:
    """The health of one public dependency and the reasons to flag it."""
    policy_line = parse_line(line)
    available = releases(crate)
    in_line = [release for release in available if line_of(release.key) == policy_line]
    newest_in_line = in_line[0] if in_line else None
    latest = available[0] if available else None
    # A fix to an older line can be the most recent release.
    last_released = max((release.date for release in available), default=None)
    relevant = [
        advisory
        for advisory in advisories
        if not advisory.withdrawn
        and (newest_in_line is None or affects(advisory, newest_in_line.key))
    ]
    health = Health(name, line, newest_in_line, latest, last_released, relevant, repository)

    health.flags += release_flags(health, policy_line, today, stale_days)
    for advisory in relevant:
        version = newest_in_line.version if newest_in_line else "every release"
        health.flags.append(
            Flag(
                advisory.id,
                f"[{advisory.id}]({ADVISORY_URL.format(advisory.id)}) ({advisory.kind}) "
                f"affects {version}: {advisory.title}",
            )
        )
    if repository.archived:
        health.flags.append(Flag("archived", f"repository {repository.slug} is archived"))
    if repository.missing:
        health.flags.append(
            Flag("no-repository", f"repository {repository.slug} was not found on GitHub")
        )
    return health


def release_flags(
    health: Health, policy_line: tuple[int, ...], today: dt.date, stale_days: int
) -> list[Flag]:
    """Flags about releases: none at all, none for a year, none in Rullst's
    line, or a newer incompatible line."""
    flags = []
    newest, latest = health.newest_in_line, health.latest
    if health.last_released is None:
        flags.append(Flag("no-release", "no stable release on crates.io"))
    else:
        age = (today - dt.date.fromisoformat(health.last_released)).days
        if age > stale_days:
            flags.append(
                Flag("stale", f"no release for {age // 30} months (last on {health.last_released})")
            )
    if newest is None:
        flags.append(Flag("no-line-release", f"no stable release in line {health.line}"))
    elif latest is not None and line_of(latest.key) != policy_line and latest.key > newest.key:
        newer = show_line(line_of(latest.key))
        flags.append(
            Flag(
                f"line-{newer}",
                f"newer incompatible line {newer} ({latest.version}, {latest.date}); "
                f"Rullst uses {health.line}",
            )
        )
    return flags


def flag_kind(flag: Flag) -> str:
    """The kind a review names: `newer-line` for any `line-…` key, `advisory`
    for a RustSec ID, else the key itself."""
    if flag.key.startswith("RUSTSEC-"):
        return "advisory"
    if flag.key.startswith("line-"):
        return "newer-line"
    return flag.key


def one_year_after(date: dt.date) -> dt.date:
    try:
        return date.replace(year=date.year + 1)
    except ValueError:  # 29 February
        return date.replace(year=date.year + 1, day=28)


def review_field(entry: dict, name: str, crate: str) -> str:
    value = entry.get(name)
    if not isinstance(value, str) or not value.strip():
        raise ValueError(f"review of `{crate}`: `{name}` must be a non-empty string")
    return value.strip()


def review_date(entry: dict, name: str, crate: str) -> dt.date:
    try:
        return dt.date.fromisoformat(review_field(entry, name, crate))
    except ValueError as error:
        raise ValueError(f"review of `{crate}`: `{name}` must be a YYYY-MM-DD date") from error


def parse_reviews(data: dict, dependencies: set[str]) -> dict[str, Review]:
    """The reviews file, validated: known crates, reviewable kinds only, a
    reason and reviewer, and an expiry within 12 months of the review."""
    if data.get("schema_version") != 1 or not isinstance(data.get("reviews"), list):
        raise ValueError("reviews file needs schema_version 1 and a `reviews` list")
    reviews: dict[str, Review] = {}
    for entry in data["reviews"]:
        if not isinstance(entry, dict):
            raise ValueError("each review must be an object")
        crate = review_field(entry, "crate", "?")
        if crate not in dependencies:
            raise ValueError(f"review of `{crate}`: not a public dependency")
        if crate in reviews:
            raise ValueError(f"review of `{crate}`: listed twice")
        kinds = entry.get("flags")
        if not isinstance(kinds, list) or not kinds or not all(isinstance(kind, str) for kind in kinds):
            raise ValueError(f"review of `{crate}`: `flags` must list flag kinds")
        if "advisory" in kinds or any(kind.startswith("RUSTSEC-") for kind in kinds):
            raise ValueError(f"review of `{crate}`: an advisory can never be marked reviewed")
        unknown = sorted(set(kinds) - REVIEWABLE_KINDS)
        if unknown:
            raise ValueError(f"review of `{crate}`: unknown flag kinds {', '.join(unknown)}")
        reviewed = review_date(entry, "reviewed", crate)
        expires = review_date(entry, "expires", crate)
        if not reviewed < expires <= one_year_after(reviewed):
            raise ValueError(
                f"review of `{crate}`: the expiry must follow the review by at most 12 months"
            )
        reviews[crate] = Review(
            crate,
            frozenset(kinds),
            review_field(entry, "reason", crate),
            review_field(entry, "reviewer", crate),
            reviewed,
            expires,
        )
    return reviews


def apply_review(health: Health, review: Review | None, today: dt.date) -> None:
    """Marks the crate reviewed when its review is current and covers every
    flag; otherwise records why the review no longer applies."""
    if review is None or not health.flags:
        return
    uncovered = sorted({flag_kind(flag) for flag in health.flags} - review.kinds)
    if today > review.expires:
        health.review_lapsed = f"the review of {review.expires.isoformat()} has expired"
    elif uncovered:
        health.review_lapsed = (
            f"the review until {review.expires.isoformat()} does not cover "
            + ", ".join(uncovered)
        )
    else:
        health.review = review


def fingerprint(results: list[Health]) -> str:
    """The counted flags without their changing details (ages, dates), so a
    workflow can tell a new finding from the same one a month older."""
    keys = sorted(
        f"{health.name}:{flag.key}" for health in results if health.counted for flag in health.flags
    )
    return "<!-- flags: " + (" ".join(keys) or "none") + " -->"


def cell(text: str) -> str:
    return text.replace("|", "\\|").replace("\n", " ")


def release_cell(release: Release | None) -> str:
    return f"{release.version} ({release.date})" if release else "none"


def repository_cell(repository: Repository) -> str:
    if repository.slug is None:
        return "not on GitHub" if repository.url else "not listed"
    if repository.missing:
        return f"{repository.slug} (not found)"
    if repository.archived is None:
        return f"{repository.slug} (not checked)"
    if repository.archived:
        return f"{repository.slug} (archived)"
    return f"{repository.slug} (pushed {repository.pushed or 'unknown'})"


def render(results: list[Health], today: dt.date, revision: str, errors: list[str]) -> str:
    flagged = [health for health in results if health.counted]
    reviewed = [health for health in results if health.flags and health.review is not None]
    lines = [
        "# Public dependency health",
        "",
        f"{len(results)} public dependencies from `.github/public-dependencies.json`, "
        f"checked on {today.isoformat()} against crates.io, the RustSec advisory "
        f"database (`{ADVISORY_DB}` at `{revision or 'unknown'}`) and GitHub.",
        "",
    ]
    if errors:
        lines += ["## Not checked", ""]
        lines += [f"- {cell(error)}" for error in errors]
        lines.append("")
    if flagged:
        lines += [f"## Flagged ({len(flagged)})", ""]
        for health in flagged:
            lines.append(f"- `{health.name}` (Rullst line `{health.line}`):")
            lines += [f"  - {cell(flag.text)}" for flag in health.flags]
            if health.review_lapsed:
                lines.append(f"  - counted again: {health.review_lapsed}")
        lines += [
            "",
            "A flag is a reason to review the dependency, not a defect: see "
            "docs/src/compatibility-policy.md#public-dependencies before "
            "replacing or moving a public dependency.",
            "",
        ]
    else:
        lines += ["Nothing is flagged.", ""]
    if reviewed:
        lines += [f"## Reviewed, staying ({len(reviewed)})", ""]
        for health in reviewed:
            review = health.review
            if review is None:
                continue
            lines.append(
                f"- `{health.name}` (Rullst line `{health.line}`), reviewed until "
                f"{review.expires.isoformat()} by {cell(review.reviewer)}: {cell(review.reason)}"
            )
            lines += [f"  - {cell(flag.text)}" for flag in health.flags]
        lines.append("")
    lines += [
        "## All public dependencies",
        "",
        "| Crate | Line | Newest in line | Latest release | Advisories | Repository |",
        "| :--- | :--- | :--- | :--- | :--- | :--- |",
    ]
    for health in results:
        advisories = ", ".join(advisory.id for advisory in health.advisories) or "none"
        lines.append(
            f"| `{health.name}` | `{health.line}` | {release_cell(health.newest_in_line)} | "
            f"{release_cell(health.latest)} | {advisories} | "
            f"{cell(repository_cell(health.repository))} |"
        )
    lines += ["", fingerprint(results)]
    return "\n".join(lines) + "\n"


# --- Network -------------------------------------------------------------------


class _SameHostAuthorization(urllib.request.HTTPRedirectHandler):
    """Follow redirects, but send the GitHub token only to the host it was meant for."""

    def redirect_request(self, req, fp, code, msg, headers, newurl):  # noqa: N802 (urllib API)
        request = super().redirect_request(req, fp, code, msg, headers, newurl)
        if request is not None and (
            urllib.parse.urlsplit(newurl).hostname != urllib.parse.urlsplit(req.full_url).hostname
        ):
            request.remove_header("Authorization")
        return request


OPENER = urllib.request.build_opener(_SameHostAuthorization)


def fetch(url: str, *, token: str | None = None, accept: str = "application/json") -> bytes:
    headers = {"User-Agent": USER_AGENT, "Accept": accept}
    if token:
        headers["Authorization"] = f"Bearer {token}"
        headers["X-GitHub-Api-Version"] = "2022-11-28"
    for attempt in range(3):
        try:
            with OPENER.open(urllib.request.Request(url, headers=headers), timeout=60) as response:
                return response.read()
        except urllib.error.HTTPError as error:
            if error.code not in {429, 500, 502, 503, 504} or attempt == 2:
                raise
        except (urllib.error.URLError, TimeoutError):
            if attempt == 2:
                raise
        time.sleep(5 * (attempt + 1))
    raise AssertionError("unreachable")


def fetch_json(url: str, **kwargs) -> dict:
    return json.loads(fetch(url, **kwargs))


def read_repository(url: str | None, token: str | None, cache: dict[str, Repository]) -> Repository:
    slug = github_slug(url)
    if slug is None:
        return Repository(None, url)
    if slug.lower() in cache:
        return cache[slug.lower()]
    repository = Repository(slug, url)
    try:
        data = fetch_json(f"{GITHUB_API}/repos/{slug}", token=token, accept="application/vnd.github+json")
        repository.archived = bool(data.get("archived"))
        repository.pushed = str(data.get("pushed_at") or "")[:10] or None
    except urllib.error.HTTPError as error:
        if error.code != 404:
            raise SourceError(f"GitHub repository {slug}: HTTP {error.code}") from error
        repository.missing = True
    cache[slug.lower()] = repository
    return repository


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n", 1)[0])
    parser.add_argument("--stale-days", type=int, default=STALE_DAYS)
    args = parser.parse_args(argv)

    policy = json.loads(POLICY_FILE.read_text(encoding="utf-8"))
    dependencies: dict[str, str] = policy["dependencies"]
    try:
        reviews = parse_reviews(
            json.loads(REVIEWS_FILE.read_text(encoding="utf-8")), set(dependencies)
        )
    except (OSError, ValueError) as error:
        print(f"{REVIEWS_FILE.name}: {error}", file=sys.stderr)
        return 2
    token = os.environ.get("GITHUB_TOKEN") or None
    today = dt.datetime.now(dt.timezone.utc).date()
    errors: list[str] = []

    try:
        tarball = fetch(
            f"{GITHUB_API}/repos/{ADVISORY_DB}/tarball/main",
            token=token,
            accept="application/vnd.github+json",
        )
        revision, advisories, problems = advisories_from_tarball(tarball, set(dependencies))
        errors += problems
    except (OSError, tarfile.TarError) as error:  # URLError and TimeoutError are OSErrors
        errors.append(f"RustSec advisory database: {error}")
        revision, advisories = "", {name: [] for name in dependencies}

    results: list[Health] = []
    repositories: dict[str, Repository] = {}
    for index, (name, line) in enumerate(sorted(dependencies.items())):
        if index:
            time.sleep(CRATES_IO_INTERVAL)
        try:
            crate = fetch_json(CRATES_API + urllib.parse.quote(name))
        except (urllib.error.URLError, TimeoutError, ValueError) as error:
            errors.append(f"crates.io `{name}`: {error}")
            continue
        url = (crate.get("crate") or {}).get("repository")
        try:
            repository = read_repository(url, token, repositories)
        except (SourceError, urllib.error.URLError, TimeoutError, ValueError) as error:
            errors.append(str(error))
            repository = Repository(github_slug(url), url)
        health = evaluate(
            name, line, crate, advisories.get(name, []), repository, today, args.stale_days
        )
        apply_review(health, reviews.get(name), today)
        results.append(health)

    sys.stdout.write(render(results, today, revision, errors))
    if errors:
        print("\n".join(errors), file=sys.stderr)
        return 2
    return 1 if any(health.counted for health in results) else 0


if __name__ == "__main__":
    raise SystemExit(main())
