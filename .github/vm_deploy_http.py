"""HTTPS requests through the generated Caddy for .github/check-vm-deploy.py."""

from __future__ import annotations

import http.cookiejar
import json
from pathlib import Path
import ssl
import urllib.error
import urllib.parse
import urllib.request

ACCOUNT_NAME = "Deploy Smoke"


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
        try:
            response = self.opener.open(request, timeout=30)
        except urllib.error.HTTPError as error:
            response = error
        except (urllib.error.URLError, OSError) as error:
            raise Failure(f"{request.get_method()} {url} failed: {error}") from error
        with response:
            body = response.read(2 * 1024 * 1024).decode(errors="replace")
            return response.status, {k.lower(): v for k, v in response.headers.items()}, body


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
