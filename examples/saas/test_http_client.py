#!/usr/bin/env python3
"""Check client isolation and the local example operator's membership boundary."""
from contextlib import closing
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
import sqlite3
import tempfile
import threading
import unittest

from http_acceptance import Client
from membership import grant_membership


class Endpoint(BaseHTTPRequestHandler):
    def log_message(self, *_args):
        pass

    def do_GET(self):
        if self.path == "/login":
            self.server.followed = True
        status = 303 if self.path == "/denied" else 200
        self.send_response(status)
        if status == 303:
            self.send_header("Location", "/login")
            self.send_header("Set-Cookie", "rullst_session=synthetic-user-a; Path=/; HttpOnly")
            self.send_header("Set-Cookie", "rullst_csrf=synthetic-csrf; Path=/")
        if self.path == "/secure":
            self.send_header("Set-Cookie", "secure_session=synthetic-secure; Secure; Path=/")
        self.end_headers()
        self.wfile.write(self.headers.get("Cookie", "").encode())

    def do_PUT(self):
        self.rfile.read(int(self.headers.get("Content-Length", "0")))
        valid = self.headers.get("X-CSRF-Token") == "synthetic-csrf"
        self.send_response(200 if valid else 403)
        self.end_headers()


class ClientContract(unittest.TestCase):
    def setUp(self):
        self.server = ThreadingHTTPServer(("127.0.0.1", 0), Endpoint)
        self.server.followed = False
        self.thread = threading.Thread(target=self.server.serve_forever, daemon=True)
        self.thread.start()
        self.base = "http://127.0.0.1:" + str(self.server.server_port)
        self.addCleanup(self.shutdown)

    def shutdown(self):
        self.server.shutdown()
        self.server.server_close()
        self.thread.join(timeout=5)
        self.assertFalse(self.thread.is_alive())

    def test_redirect_denial_is_visible_and_separate_clients_never_share_cookies(self):
        alice, bob = Client(self.base), Client(self.base)
        status, headers, _ = alice.request("GET", "/denied")
        self.assertEqual(status, 303)
        self.assertEqual(headers["Location"], "/login")
        self.assertFalse(self.server.followed)
        self.assertEqual(alice.cookie("rullst_session"), "synthetic-user-a")
        self.assertIn(b"synthetic-user-a", alice.request("GET", "/echo")[2])
        self.assertEqual(bob.request("GET", "/echo")[2], b"")
        self.assertIsNone(bob.cookie("rullst_session"))
        self.assertEqual(alice.request("PUT", "/note", payload={"body": "a"})[0], 200)
        self.assertEqual(alice.request("PUT", "/note", payload={"body": "a"}, csrf=False)[0], 403)

    def test_secure_cookie_is_not_silently_sent_over_the_development_http_profile(self):
        client = Client(self.base)
        client.request("GET", "/secure")
        self.assertEqual(client.cookie("secure_session"), "synthetic-secure")
        self.assertNotIn(b"synthetic-secure", client.request("GET", "/echo")[2])


class MembershipContract(unittest.TestCase):
    def test_only_existing_accounts_receive_idempotent_parameterized_membership(self):
        with tempfile.TemporaryDirectory() as directory:
            database = Path(directory) / "example.sqlite"
            with closing(sqlite3.connect(database)) as db, db:
                db.execute("CREATE TABLE users (id INTEGER PRIMARY KEY, email TEXT UNIQUE)")
                db.execute("CREATE TABLE journey_memberships (user_id INTEGER REFERENCES users(id), "
                           "tenant_id TEXT, PRIMARY KEY (user_id, tenant_id))")
                db.execute("INSERT INTO users VALUES (1, ?)", ("alice@example.invalid",))
            grant_membership(database, " Alice@example.invalid ", "org-a")
            grant_membership(database, "alice@example.invalid", "org-a")
            for email in ("missing@example.invalid", "' OR 1=1 --"):
                with self.assertRaises(ValueError):
                    grant_membership(database, email, "org-b")
            with self.assertRaises(ValueError):
                grant_membership(database, "alice@example.invalid", "org a")
            with closing(sqlite3.connect(database)) as db:
                self.assertEqual(db.execute("SELECT * FROM journey_memberships").fetchall(), [(1, "org-a")])

    def test_mistyped_database_path_does_not_create_an_empty_database(self):
        with tempfile.TemporaryDirectory() as directory:
            database = Path(directory) / "absent.sqlite"
            with self.assertRaises(sqlite3.OperationalError):
                grant_membership(database, "alice@example.invalid", "org-a")
            self.assertFalse(database.exists())


if __name__ == "__main__":
    unittest.main()
