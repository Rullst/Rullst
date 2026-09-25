"""Real loopback HTTP acceptance; no external services or browser claims."""
import http.cookiejar
import json
import secrets
import sqlite3
import urllib.error
import urllib.parse
import urllib.request


class NoRedirect(urllib.request.HTTPRedirectHandler):
    def redirect_request(self, req, fp, code, msg, headers, newurl):
        return None


class Client:
    def __init__(self, base):
        self.base = base
        self.cookies = http.cookiejar.CookieJar()
        self.opener = urllib.request.build_opener(
            urllib.request.ProxyHandler({}), NoRedirect(),
            urllib.request.HTTPCookieProcessor(self.cookies),
        )

    def cookie(self, name):
        return next((cookie.value for cookie in self.cookies if cookie.name == name), None)

    def request(self, method, path, *, form=None, payload=None, tenant=None, csrf=True):
        headers = {}
        data = None
        if form is not None:
            headers["Content-Type"] = "application/x-www-form-urlencoded"
            data = urllib.parse.urlencode(form).encode()
        if payload is not None:
            headers["Content-Type"] = "application/json"
            data = json.dumps(payload).encode()
        if tenant is not None:
            headers["X-Tenant-ID"] = tenant
        if method != "GET" and csrf and self.cookie("rullst_csrf"):
            headers["X-CSRF-Token"] = self.cookie("rullst_csrf")
        request = urllib.request.Request(self.base + path, data=data, headers=headers, method=method)
        try:
            response = self.opener.open(request, timeout=10)
        except urllib.error.HTTPError as error:
            response = error
        with response:
            body = response.read(1024 * 1024 + 1)
            if len(body) > 1024 * 1024:
                raise RuntimeError("HTTP response exceeded the journey limit")
            return response.status, response.headers, body


def exercise(base, database, restart, report):
    def check(name, condition):
        report["cases"].append({"name": name, "passed": bool(condition)})
        if not condition:
            raise RuntimeError("Acceptance failed: " + name)

    def expect(name, response, status):
        report["cases"].append({"name": name, "passed": response[0] == status,
                                "expected_status": status, "actual_status": response[0]})
        if response[0] != status:
            raise RuntimeError(f"Acceptance failed: {name}: expected HTTP {status}, got {response[0]}")
        return response

    anonymous = Client(base)
    response = expect("anonymous-resource", anonymous.request("GET", "/journey/notes/1"), 303)
    check("anonymous-login-location", response[1].get("Location") == "/login")
    clients = {}
    password = secrets.token_urlsafe(24)
    for name in ("alice", "bob", "charlie"):
        client = Client(base)
        page = expect(name + "-registration-form", client.request("GET", "/register"), 200)
        check(name + "-csrf-form", bool(client.cookie("rullst_csrf")) and
              client.cookie("rullst_csrf").encode() in page[2])
        # Exercise the form proof on registration, and header proofs on JSON writes.
        response = expect(name + "-register", client.request("POST", "/register", form={
            "name": name.title(), "email": name + "@example.invalid", "password": password,
            "_token": client.cookie("rullst_csrf"),
        }, csrf=False), 303)
        check(name + "-session-cookie", bool(client.cookie("rullst_session")) and
              response[1].get("Location") == "/dashboard")
        expect(name + "-dashboard", client.request("GET", "/dashboard"), 200)
        clients[name] = client

    # Only the fixture operator can provision/revoke membership. HTTP never
    # accepts a user_id/tenant_id claim as authority. No real account is involved.
    with sqlite3.connect(database) as db:
        users = dict(db.execute("SELECT email, id FROM users"))
        check("three-persisted-users", len(users) == 3)
        for name, tenant in (("alice", "org-a"), ("bob", "org-b"), ("charlie", "org-a")):
            db.execute("INSERT INTO journey_memberships (user_id, tenant_id) VALUES (?, ?)",
                       (users[name + "@example.invalid"], tenant))

    denied = Client(base)
    expect("login-form", denied.request("GET", "/login"), 200)
    expect("wrong-password-response", denied.request("POST", "/login", form={
        "email": "alice@example.invalid", "password": "incorrect-fixture-password",
    }), 200)
    check("wrong-password-no-session", denied.cookie("rullst_session") is None)
    expect("wrong-password-denied", denied.request("GET", "/dashboard"), 303)
    expect("valid-login", denied.request("POST", "/login", form={
        "email": "alice@example.invalid", "password": password,
    }), 303)
    check("login-created-session", bool(denied.cookie("rullst_session")))

    alice, bob, charlie = (clients[name] for name in ("alice", "bob", "charlie"))
    response = expect("owner-create", alice.request("POST", "/journey/notes",
                      payload={"body": "first note"}, tenant="org-a"), 201)
    note = json.loads(response[2])
    check("server-bound-ownership", note["owner_id"] == users["alice@example.invalid"] and
          note["tenant_id"] == "org-a" and note["body"] == "first note")
    path = "/journey/notes/" + str(note["id"])
    response = expect("owner-read", alice.request("GET", path, tenant="org-a"), 200)
    check("security-headers", response[1].get("X-Content-Type-Options") == "nosniff" and
          bool(response[1].get("Content-Security-Policy")))
    expect("trusted-single-membership-default", alice.request("GET", path), 200)
    for client, tenant, prefix, status in (
        (bob, "org-b", "cross-tenant", 404),
        (charlie, "org-a", "same-tenant-wrong-owner", 403),
        (bob, "org-a", "forged-tenant", 403),
    ):
        expect(prefix + "-read", client.request("GET", path, tenant=tenant), status)
        expect(prefix + "-write", client.request("PUT", path, tenant=tenant,
               payload={"body": "unauthorized"}), status)
    expect("missing-csrf", alice.request("PUT", path, tenant="org-a", csrf=False,
           payload={"body": "csrf-unauthorized"}), 403)
    expect("reject-owner-mass-assignment", alice.request("PUT", path, tenant="org-a",
           payload={"body": "spoofed", "owner_id": users["bob@example.invalid"]}), 422)
    expect("reject-tenant-mass-assignment", alice.request("POST", "/journey/notes", tenant="org-a",
           payload={"body": "spoofed", "tenant_id": "org-b"}), 422)
    expect("bounded-resource", alice.request("PUT", path, tenant="org-a",
           payload={"body": "a" * 257}), 422)
    with sqlite3.connect(database) as db:
        check("denials-left-state-unchanged", db.execute(
            "SELECT tenant_id, owner_id, body FROM journey_notes").fetchall() ==
            [("org-a", users["alice@example.invalid"], "first note")])
    expect("owner-update", alice.request("PUT", path, tenant="org-a",
           payload={"body": "persist across restart"}), 200)
    restart()
    response = expect("authorized-after-restart", alice.request("GET", path, tenant="org-a"), 200)
    check("persisted-content-after-restart", json.loads(response[2])["body"] == "persist across restart")
    expect("cross-tenant-after-restart", bob.request("GET", path, tenant="org-b"), 404)
    with sqlite3.connect(database) as db:
        db.execute("DELETE FROM journey_memberships WHERE user_id = ?",
                   (users["alice@example.invalid"],))
    expect("fresh-membership-after-revocation", alice.request("GET", path, tenant="org-a"), 403)
    expect("revoked-membership-cannot-write", alice.request("PUT", path, tenant="org-a",
           payload={"body": "revoked write"}), 403)
    with sqlite3.connect(database) as db:
        check("revocation-left-state-unchanged", db.execute(
            "SELECT body FROM journey_notes WHERE id = ?", (note["id"],)).fetchone() ==
            ("persist across restart",))
    expect("logout", denied.request("POST", "/logout"), 303)
    check("logout-cleared-client-cookie", denied.cookie("rullst_session") is None)
    expect("logout-denied", denied.request("GET", "/dashboard"), 303)
