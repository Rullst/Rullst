"""HTTPS requests through the generated Caddy for .github/check-vm-deploy.py."""

from __future__ import annotations

import http.cookiejar
import json
from pathlib import Path
import re
import ssl
import urllib.error
import urllib.parse
import urllib.request

ACCOUNT_NAME = "Deploy Smoke"
# Every starter ships the htmx client; the Blank page links its hashed name.
HTMX = "/static/htmx-2.0.11.min.js"
HASHED_HTMX = re.compile(r"/static/htmx-2\.0\.11\.min\.[0-9a-f]{16}\.js")


class Failure(Exception):
    """A deployment check failed; the message says which."""


def log(message: str) -> None:
    print(message, flush=True)


def expect(condition: bool, message: str) -> None:
    if not condition:
        raise Failure(message)
    log(f"ok: {message}")


class NoRedirect(urllib.request.HTTPRedirectHandler):
    def redirect_request(self, *_args, **_kwargs):
        return None


class Client:
    """A cookie-keeping HTTPS client that trusts only Caddy's local CA."""

    def __init__(self, ca: Path):
        self.cookies = http.cookiejar.CookieJar()
        context = ssl.create_default_context(cafile=str(ca))
        # Chain and host name are verified; RFC 5280 profile strictness (the
        # Python 3.13+ default) is not what this check is about.
        context.verify_flags &= ~ssl.VERIFY_X509_STRICT
        self.opener = urllib.request.build_opener(
            urllib.request.ProxyHandler({}), NoRedirect(),
            urllib.request.HTTPSHandler(context=context),
            urllib.request.HTTPCookieProcessor(self.cookies))

    def cookie(self, name: str) -> str:
        return next((cookie.value or "" for cookie in self.cookies if cookie.name == name), "")

    def request(self, url: str, form: dict[str, str] | None = None) -> tuple[int, dict, str]:
        headers, data = {}, None
        if form is not None:
            token = self.cookie("rullst_csrf")
            headers = {"Content-Type": "application/x-www-form-urlencoded",
                       "Origin": "https://localhost", "X-CSRF-Token": token}
            data = urllib.parse.urlencode({**form, "_token": token}).encode()
        request = urllib.request.Request(url, data=data, headers=headers)
        with self.open(request) as response:
            body = response.read(2 * 1024 * 1024).decode(errors="replace")
            return response.status, {k.lower(): v for k, v in response.headers.items()}, body

    def fetch(self, url: str, encoding: str):
        """GET `url` accepting only `encoding`: the status, the headers (an
        `email.message.Message`) and the body as served."""
        request = urllib.request.Request(url, headers={"Accept-Encoding": encoding})
        with self.open(request) as response:
            return response.status, response.headers, response.read(2 * 1024 * 1024)

    def open(self, request: urllib.request.Request):
        try:
            return self.opener.open(request, timeout=30)
        except urllib.error.HTTPError as error:
            return error
        except OSError as error:  # URLError is an OSError
            raise Failure(f"{request.get_method()} {request.full_url} failed: {error}") from error


def check_https(ca: Path) -> None:
    client = Client(ca)
    code, headers, body = client.request("https://localhost/health")
    expect(code == 200, f"GET https://localhost/health through Caddy answers {code}")
    try:
        json.loads(body)
    except json.JSONDecodeError as error:
        raise Failure(f"/health did not answer JSON: {error}") from error
    expect(headers.get("x-content-type-options") == "nosniff",
           "the app's security headers pass through Caddy")
    code, headers, _ = client.request("http://localhost/health")
    expect(code in (301, 308) and headers.get("location") == "https://localhost/health",
           f"HTTP redirects to HTTPS ({code} -> {headers.get('location')})")


def check_application(starter: str, ca: Path, password: str, registered: bool) -> None:
    """A database-backed request through Caddy; SaaS registers, then signs in."""
    client = Client(ca)
    if starter == "blank":
        code, _, body = client.request("https://localhost/")
        expect(code == 200 and "Database connected." in body,
               f"the home page reaches the database ({code})")
        return
    page = "/login" if registered else "/register"
    code, _, _ = client.request(f"https://localhost{page}")
    expect(code == 200 and client.cookie("rullst_csrf") != "", f"GET {page} sets the CSRF cookie")
    form = {"email": "deploy-smoke@example.com", "password": password}
    if not registered:
        form["name"] = ACCOUNT_NAME
    code, headers, _ = client.request(f"https://localhost{page}", form)
    expect(code in (302, 303) and headers.get("location") == "/dashboard",
           f"POST {page} over HTTPS signs the account in ({code} -> {headers.get('location')})")
    session = [cookie for cookie in client.cookies if cookie.name == "rullst_session"]
    expect(len(session) == 1 and session[0].secure, "the session cookie is Secure behind Caddy")
    code, _, body = client.request("https://localhost/dashboard")
    expect(code == 200 and ACCOUNT_NAME in body, f"the dashboard shows the account ({code})")


def varies_on_encoding(headers) -> bool:
    values = ",".join(headers.get_all("Vary") or [])
    return "accept-encoding" in [value.strip().lower() for value in values.split(",")]


def check_compression(starter: str, ca: Path) -> None:
    """Pages stay uncompressed (they carry the unmasked CSRF token: BREACH);
    static files come from the image's .br/.zst siblings."""
    client = Client(ca)
    page = "/" if starter == "blank" else "/login"
    code, headers, body = client.fetch(f"https://localhost{page}", "gzip, zstd, br")
    encoding = headers.get("Content-Encoding")
    html = body.decode(errors="replace")
    expect(code == 200 and encoding is None and "</html>" in html.lower(),
           f"GET {page} is sent uncompressed to a client that accepts compression "
           f"({code}, {encoding})")

    code, headers, plain = client.fetch(f"https://localhost{HTMX}", "identity")
    expect(code == 200 and headers.get("Content-Encoding") is None
           and headers.get("Cache-Control") == "no-cache",
           f"GET {HTMX} without compression ({code}, {headers.get('Cache-Control')})")
    paths = [HTMX]
    if starter == "blank":
        hashed = HASHED_HTMX.search(html)
        expect(hashed is not None, "the Blank page links the content-hashed htmx client")
        paths.append(hashed.group(0))
    for path in paths:
        for coding in ("br", "zstd"):
            code, headers, body = client.fetch(f"https://localhost{path}", coding)
            served = headers.get("Content-Encoding")
            kind = headers.get("Content-Type") or ""
            cache = headers.get("Cache-Control") or ""
            expect(code == 200 and served == coding and "javascript" in kind
                   and varies_on_encoding(headers) and 0 < len(body) < len(plain) // 2,
                   f"GET {path} with {coding} serves the image's sibling "
                   f"({code}, {served}, {kind}, {len(body)} of {len(plain)} bytes)")
            immutable = path != HTMX
            expect(("immutable" in cache) == immutable,
                   f"GET {path} with {coding} keeps its caching ({cache})")
