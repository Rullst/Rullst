#!/usr/bin/env python3
"""Run two owned Rullst processes with a pinned Caddy/Redis pair on loopback (Linux)."""
import collections
import json
import os
from pathlib import Path
import queue
import secrets
import socket
import subprocess
import sys
import tempfile
import threading
import uuid

from deployment_proxy_contract import exercise, request, wait_for

ROOT = Path(__file__).resolve().parent.parent
CADDY = "caddy@sha256:de23def33b17fb5d1290b0f6c2add1d70780e52341896c00a4c8a2a2fe9d355e"
REDIS = "redis:7.4-alpine@sha256:ff02b58f971e7d7d156a1267e283fcbbeee91773b6aa36c49dac28ecfe28eadf"


def docker(*args):
    return subprocess.check_output(["docker", *args], text=True, timeout=120).strip()


def executable():
    build = subprocess.run([
        "cargo", "test", "--locked", "-p", "rullst-security", "--no-default-features",
        "--features", "redis-rate-limit", "--test", "deployment_acceptance", "--no-run",
        "--message-format=json",
    ], cwd=ROOT, check=True, capture_output=True, text=True, timeout=1200)
    matches = []
    for line in build.stdout.splitlines():
        message = json.loads(line)
        if message.get("reason") == "compiler-artifact" and message.get("executable"):
            if message["target"]["name"] == "deployment_acceptance":
                matches.append(message["executable"])
    if len(matches) != 1:
        raise RuntimeError("expected exactly one owned deployment test executable")
    return matches[0]


def ports():
    sockets = [socket.socket() for _ in range(3)]
    try:
        for handle in sockets:
            handle.bind(("127.0.0.1", 0))
        return [handle.getsockname()[1] for handle in sockets]
    finally:
        for handle in sockets:
            handle.close()


class Application:
    def __init__(self, binary, root, name, port, proxy, redis):
        self.port = port
        self.events = queue.Queue()
        self.log = collections.deque(maxlen=40)
        root.mkdir()
        env = dict(os.environ)
        # A fresh cwd supplies no .env/rullst.toml/application database. Override
        # relevant runtime configuration instead of borrowing a user's app.
        env.update({
            "RULLST_DEPLOYMENT_DISPOSABLE": "1", "RULLST_HOST": "127.0.0.1",
            "HOST": "127.0.0.1", "PORT": str(port), "RULLST_ENV": "production",
            "APP_KEY": secrets.token_hex(32), "RULLST_DISABLE_UPDATE_CHECK": "true",
            "RULLST_DEPLOYMENT_REPLICA": name, "RULLST_DEPLOYMENT_PORT": str(port),
            "RULLST_DEPLOYMENT_ORIGIN": f"http://127.0.0.1:{proxy}",
            "RULLST_DEPLOYMENT_REDIS": redis,
        })
        self.process = subprocess.Popen([
            binary, "--ignored", "--exact", "owned_application_process", "--nocapture",
        ], cwd=root, env=env, stdin=subprocess.PIPE, stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT, text=True, bufsize=1)
        self.reader = threading.Thread(target=self.read, daemon=True)
        self.reader.start()
        try:
            wait_for(lambda: request(port, "/health")[0] == 200 and
                     self.command("snapshot")["lifecycle"]["phase"] == "ready", "owned app listener")
        except Exception:
            self.kill()
            print("Owned app startup output:\n" + "\n".join(self.log), file=sys.stderr)
            raise

    def read(self):
        for line in self.process.stdout:
            self.log.append(line.rstrip())
            if line.startswith("FIXTURE "):
                self.events.put(json.loads(line.removeprefix("FIXTURE ")))

    def command(self, value):
        assert value in {"ready", "unready", "release", "drain", "snapshot"}
        self.process.stdin.write(value + "\n")
        self.process.stdin.flush()
        event = self.events.get(timeout=5)
        assert event["command"] == value
        return event

    def stop(self):
        self.process.stdin.write("stop\n")
        self.process.stdin.close()
        assert self.process.wait(timeout=8) == 0, "owned app did not stop gracefully"

    def kill(self):
        if self.process.poll() is None:
            self.process.kill()
            self.process.wait(timeout=5)


def main():
    if not sys.platform.startswith("linux"):
        raise SystemExit("Owned proxy contract currently requires Linux host networking")
    binary = executable()
    names = {kind: "rullst-deployment-" + kind + "-" + uuid.uuid4().hex
             for kind in ["proxy", "redis"]}
    started, apps = [], {}
    with tempfile.TemporaryDirectory(prefix="rullst-deployment-") as temporary:
        root = Path(temporary)
        try:
            docker("run", "--detach", "--rm", "--name", names["redis"],
                   "--publish", "127.0.0.1::6379", REDIS,
                   "redis-server", "--save", "", "--appendonly", "no", "--maxmemory", "16mb")
            started.append(names["redis"])
            binding = docker("port", names["redis"], "6379/tcp")
            assert binding.startswith("127.0.0.1:") and binding.removeprefix("127.0.0.1:").isdigit()
            wait_for(lambda: docker("exec", names["redis"], "redis-cli", "ping") == "PONG", "owned Redis ready")
            redis = "redis://" + binding
            a, b, proxy = ports()
            for name, port in [("a", a), ("b", b)]:
                apps[name] = Application(binary, root / name, name, port, proxy, redis)
            assert apps["a"].process.pid != apps["b"].process.pid
            configuration = (ROOT / ".github/fixtures/deployment.Caddyfile").read_text()
            for key, value in [("__APP_A__", a), ("__APP_B__", b), ("__PROXY_PORT__", proxy)]:
                configuration = configuration.replace(key, str(value))
            config = root / "Caddyfile"
            config.write_text(configuration)
            config.chmod(0o644)
            # The official binary carries cap_net_bind_service; retain only that
            # capability so Linux can execute it even though fixture ports are high.
            docker("run", "--detach", "--name", names["proxy"], "--network", "host",
                   "--read-only", "--cap-drop", "ALL", "--cap-add", "NET_BIND_SERVICE",
                   "--security-opt", "no-new-privileges",
                   "--user", "65532:65532", "--tmpfs", "/data:rw,size=16m,mode=1777",
                   "--tmpfs", "/config:rw,size=16m,mode=1777",
                   "--mount", f"type=bind,source={config},target=/etc/caddy/Caddyfile,readonly",
                   CADDY, "caddy", "run", "--config", "/etc/caddy/Caddyfile")
            started.append(names["proxy"])

            def stop_redis():
                docker("stop", "--time", "1", names["redis"])
                started.remove(names["redis"])

            exercise(proxy, apps, stop_redis)
        except Exception:
            for name, app in apps.items():
                print("Owned app " + name + " recent output:\n" + "\n".join(app.log), file=sys.stderr)
            for name in started:
                subprocess.run(["docker", "logs", "--tail", "30", name], check=False, timeout=10)
            raise
        finally:
            for app in apps.values():
                app.kill()
            for name in reversed(started):
                subprocess.run(["docker", "stop", "--time", "1", name],
                               check=False, timeout=15, stdout=subprocess.DEVNULL)
                if name == names["proxy"]:
                    subprocess.run(["docker", "rm", name], check=False, timeout=15,
                                   stdout=subprocess.DEVNULL)


if __name__ == "__main__":
    main()
