#!/usr/bin/env python3
"""Check that the acceptance client cannot hide denials or share user cookies."""
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
import threading
import unittest

from saas_journey_http import Client


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


if __name__ == "__main__":
    unittest.main()
