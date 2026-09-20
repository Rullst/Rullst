"""Bounded HTTP/WebSocket assertions for the owned deployment fixture only."""
import base64
import hashlib
import http.client
import json
import os
import socket
import struct
import time


def request(port, path="/identity", method="GET", headers=None, body=None):
    connection = http.client.HTTPConnection("127.0.0.1", port, timeout=3)
    try:
        connection.request(method, path, body=body, headers=headers or {})
        response = connection.getresponse()
        payload = response.read(16_385)
        assert len(payload) <= 16_384
        return response.status, dict(response.getheaders()), payload
    finally:
        connection.close()


def wait_for(predicate, label, seconds=5):
    end = time.monotonic() + seconds
    while time.monotonic() < end:
        try:
            if predicate():
                return
        except (OSError, http.client.HTTPException):
            pass
        time.sleep(0.05)
    raise AssertionError("deployment condition timed out: " + label)


def replicas(port, count=12):
    values = set()
    for _ in range(count):
        status, _, payload = request(port)
        if status != 200:
            return set()
        values.add(json.loads(payload)["replica"])
    return values


class WebSocket:
    """Tiny fixture client: bounded unfragmented text/close frames, never a library."""
    def __init__(self, port):
        self.socket = socket.create_connection(("127.0.0.1", port), timeout=3)
        self.file = self.socket.makefile("rb")
        key = base64.b64encode(os.urandom(16)).decode()
        self.socket.sendall((
            f"GET /socket HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\n"
            f"Origin: http://127.0.0.1:{port}\r\nUpgrade: websocket\r\n"
            f"Connection: Upgrade\r\nSec-WebSocket-Version: 13\r\n"
            f"Sec-WebSocket-Key: {key}\r\n\r\n"
        ).encode())
        assert self.file.readline(1024).startswith(b"HTTP/1.1 101")
        headers = {}
        for _ in range(64):
            line = self.file.readline(4096)
            if line == b"\r\n":
                break
            name, value = line.decode().split(":", 1)
            headers[name.lower()] = value.strip()
        else:
            raise AssertionError("unbounded upgrade headers")
        # SHA-1 is mandated by the WebSocket handshake, not used for authentication.
        digest = hashlib.sha1((key + "258EAFA5-E914-47DA-95CA-C5AB0DC85B11").encode(), usedforsecurity=False)
        assert headers["sec-websocket-accept"] == base64.b64encode(digest.digest()).decode()

    def send(self, payload, opcode=1):
        assert len(payload) < 126
        mask = os.urandom(4)
        self.socket.sendall(bytes([0x80 | opcode, 0x80 | len(payload)]) + mask +
                            bytes(value ^ mask[i % 4] for i, value in enumerate(payload)))

    def receive(self):
        head = self.file.read(2)
        assert len(head) == 2 and head[0] & 0x80 and not head[1] & 0x80
        size = head[1] & 127
        if size == 126:
            size = struct.unpack("!H", self.file.read(2))[0]
        assert size <= 1024 and size != 127
        payload = self.file.read(size)
        assert len(payload) == size
        return head[0] & 15, payload

    def close(self):
        try:
            self.send(b"\x03\xe8", 8)
        except OSError:
            pass
        self.file.close()
        self.socket.close()


def exercise(proxy, apps, stop_redis):
    wait_for(lambda: request(proxy)[0] == 503, "both replicas unready")
    apps["a"].command("ready")
    wait_for(lambda: replicas(proxy) == {"a"}, "startup excludes unready b")
    apps["b"].command("ready")
    wait_for(lambda: replicas(proxy) == {"a", "b"}, "both replicas serve")
    apps["a"].command("unready")
    wait_for(lambda: replicas(proxy) == {"b"}, "unhealthy replica removed")
    apps["a"].command("ready")
    wait_for(lambda: replicas(proxy) == {"a", "b"}, "readiness recovery")

    forged = {"X-Forwarded-For": "198.51.100.66", "Forwarded": "for=198.51.100.66"}
    for port in [proxy, apps["a"].port, apps["b"].port]:
        status, headers, payload = request(port, headers=forged)
        assert status == 200
        data = json.loads(payload)
        assert data["peer"] == "127.0.0.1", data
        if port == proxy:
            assert data["xff"] == "127.0.0.1" and data["forwarded"] is None, data
        headers = {key.lower(): value for key, value in headers.items()}
        assert headers["x-content-type-options"] == "nosniff"
        assert "content-security-policy" in headers

    seen = set()
    for remaining in range(5, -1, -1):
        status, _, payload = request(proxy, "/limited", headers=forged)
        assert status == 200
        data = json.loads(payload)
        assert data["remaining"] == remaining
        seen.add(data["replica"])
    assert seen == {"a", "b"}
    for port in [proxy, apps["a"].port, apps["b"].port]:
        assert request(port, "/limited", headers={"X-Forwarded-For": "203.0.113.99"})[0] == 429

    assert request(proxy, "/bounded", method="POST", body=b"x")[0] == 403
    csrf = "ab" * 32
    headers = {"Cookie": "rullst_csrf=" + csrf, "X-CSRF-Token": csrf,
               "Content-Type": "application/octet-stream"}
    assert request(proxy, "/bounded", method="POST", headers=headers, body=b"x" * 1024)[0] == 200
    assert request(proxy, "/bounded", method="POST", headers=headers, body=b"x" * 1025)[0] == 413
    upgrade = {"Upgrade": "websocket", "Connection": "Upgrade", "Sec-WebSocket-Version": "13",
               "Sec-WebSocket-Key": base64.b64encode(os.urandom(16)).decode()}
    assert request(proxy, "/socket", headers=upgrade)[0] == 403
    assert request(proxy, "/socket", headers={**upgrade, "Origin": "https://attacker.invalid"})[0] == 403

    connection = http.client.HTTPConnection("127.0.0.1", proxy, timeout=3)
    connection.request("GET", "/stream")
    response = connection.getresponse()
    assert response.status == 200
    first = response.readline(128).decode()
    selected = first.split(":")[0]
    assert first == selected + ":begin\n" and selected in apps
    websocket = None
    try:
        # Round robin can select either replica; keep only the matching upgrade.
        for _ in range(8):
            candidate = WebSocket(proxy)
            candidate.send(b"identify")
            opcode, echoed = candidate.receive()
            assert opcode == 1
            if echoed == (selected + ":identify").encode():
                websocket = candidate
                break
            candidate.close()
        assert websocket is not None
        wait_for(lambda: apps[selected].command("snapshot")["websockets"] == 1,
                 "unused upgrades finish closing")
        snapshot = apps[selected].command("snapshot")
        assert snapshot["lifecycle"]["in_flight_requests"] == 1
        assert snapshot["websockets"] == 1
        apps[selected].command("drain")
        opcode, payload = websocket.receive()
        assert opcode == 8 and struct.unpack("!H", payload[:2])[0] == 1012
        other = "b" if selected == "a" else "a"
        wait_for(lambda: replicas(proxy) == {other}, "draining replica excluded")
        assert apps[selected].command("snapshot")["lifecycle"]["in_flight_requests"] == 1
        apps[selected].command("release")
        assert response.read(128) == (selected + ":end\n").encode()
        wait_for(lambda: apps[selected].command("snapshot")["lifecycle"]["in_flight_requests"] == 0,
                 "stream body completes before process stop")
        apps[selected].stop()
        stop_redis()
        assert request(proxy, "/limited")[0] == 503
        apps[other].kill()
        wait_for(lambda: request(proxy)[0] == 503, "abrupt peer loss leaves no healthy upstream")
    finally:
        if websocket is not None:
            websocket.close()
        connection.close()
    print("Deployment: two processes, readiness/failure/drain, streaming, shared Redis budget, forged headers, CSRF/body limits and WebSocket origin/close contracts passed")
