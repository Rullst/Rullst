#!/usr/bin/env python3
"""Verify independent Messaging processes against an owned standard TLS collector."""
import argparse
import os
from pathlib import Path
import re
import ssl
import subprocess
import tempfile
import time
import urllib.error
import urllib.request
import uuid

IMAGE = "otel/opentelemetry-collector:0.161.0@sha256:b6d2b9a85b1029d05b5ad913150c1f014eed4ae99be81a1813ca5ade4a191913"


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--coverage", action="store_true")
    args = parser.parse_args()
    name = "rullst-tracing-" + uuid.uuid4().hex
    started = False
    with tempfile.TemporaryDirectory(prefix="rullst-trace-collector-") as directory:
        work = Path(directory)
        exposed = work / "fixture"
        exposed.mkdir(mode=0o755)
        commands = [
            ["openssl", "req", "-x509", "-newkey", "rsa:2048", "-nodes", "-keyout", "ca-key.pem", "-out", "ca.pem", "-days", "1", "-subj", "/CN=Disposable Trace Fixture CA"],
            ["openssl", "req", "-new", "-newkey", "rsa:2048", "-nodes", "-keyout", "fixture/key.pem", "-out", "request.pem", "-subj", "/CN=localhost"],
            ["openssl", "x509", "-req", "-in", "request.pem", "-CA", "ca.pem", "-CAkey", "ca-key.pem", "-CAcreateserial", "-out", "fixture/cert.pem", "-days", "1", "-extfile", "extensions.cnf"],
        ]
        (work / "extensions.cnf").write_text("basicConstraints=critical,CA:FALSE\nsubjectAltName=DNS:localhost\nextendedKeyUsage=serverAuth\n")
        for command in commands:
            subprocess.run(command, cwd=work, check=True, timeout=30, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        # The mounted server identity is public, ephemeral fixture material.
        # The CA private key stays outside the container mount.
        (exposed / "key.pem").chmod(0o644)
        (exposed / "collector.yaml").write_text("""receivers:
  otlp:
    protocols:
      http:
        endpoint: 0.0.0.0:4318
        tls:
          cert_file: /fixture/cert.pem
          key_file: /fixture/key.pem
exporters:
  debug:
    verbosity: detailed
    sampling_initial: 1000
    sampling_thereafter: 1
service:
  telemetry:
    metrics:
      level: none
  pipelines:
    traces:
      receivers: [otlp]
      exporters: [debug]
""")
        try:
            subprocess.run(["docker", "run", "--detach", "--name", name,
                            "--user", "10001:10001", "--cap-drop", "ALL", "--security-opt", "no-new-privileges",
                            "--read-only", "--memory", "192m", "--cpus", "1", "--pids-limit", "128",
                            "--log-opt", "max-size=1m", "--log-opt", "max-file=1",
                            "--mount", "type=bind,source=" + str(exposed) + ",target=/fixture,readonly",
                            "--publish", "127.0.0.1::4318", IMAGE,
                            "--config=/fixture/collector.yaml"], check=True, timeout=120)
            started = True
            binding = subprocess.check_output(["docker", "port", name, "4318/tcp"], text=True, timeout=10).strip()
            if not re.fullmatch(r"127\.0\.0\.1:\d+", binding):
                raise RuntimeError("Owned collector must bind only loopback")
            endpoint = "https://localhost:" + binding.rsplit(":", 1)[1] + "/v1/traces"
            context = ssl.create_default_context(cafile=str(work / "ca.pem"))
            for attempt in range(40):
                try:
                    request = urllib.request.Request(endpoint, data=b"", headers={"Content-Type": "application/x-protobuf"})
                    with urllib.request.urlopen(request, context=context, timeout=1) as response:
                        assert response.status == 200
                    break
                except (urllib.error.URLError, TimeoutError):
                    if attempt == 39:
                        raise
                    time.sleep(0.1)
            env = dict(os.environ, RULLST_TEST_OTLP_ENDPOINT=endpoint, RULLST_TEST_OTLP_CA=str(work / "ca.pem"))
            base = ["cargo", "llvm-cov", "--no-report"] if args.coverage else ["cargo", "test", "--locked"]
            subprocess.run(base + ["-p", "rullst-core", "--no-default-features", "--features", "telemetry",
                                  "--test", "distributed_tracing", "--", "--ignored", "--exact",
                                  "protocol::verified_ca_succeeds_while_untrusted_ca_and_wrong_hostname_fail", "--nocapture"],
                           check=True, env=env, timeout=600)
            subprocess.run(base + ["-p", "rullst", "--no-default-features", "--features", "telemetry,messaging-sqlite",
                                  "--test", "distributed_tracing", "--", "--ignored", "--exact",
                                  "independent_producer_and_consumer_reach_the_standard_collector", "--nocapture"],
                           check=True, env=env, timeout=600)
            logs = subprocess.check_output(["docker", "logs", name], text=True, stderr=subprocess.STDOUT, timeout=10)
            if len(logs) > 1024 * 1024 or "private-" in logs:
                raise RuntimeError("Collector log violated metadata/size boundary")
            spans = {}
            for block in re.split(r"(?m)^\s*Span #\d+\s*$", logs)[1:]:
                fields = {}
                for field, value in re.findall(r"(?m)^[ \t]*(Trace ID|Parent ID|ID|Name)[ \t]*:[ \t]*([^\r\n]*)", block):
                    fields[field] = value.strip()
                if fields.get("Name", "").startswith("academy."):
                    if fields["Name"] in spans:
                        raise RuntimeError("Duplicate collector span")
                    spans[fields["Name"]] = fields
            assert set(spans) == {"academy.schedule", "academy.handle", "academy.persist"}, spans
            producer, consumer, effect = (spans[key] for key in ("academy.schedule", "academy.handle", "academy.persist"))
            assert re.fullmatch(r"[0-9a-f]{32}", producer["Trace ID"])
            assert producer["Parent ID"] == ""
            assert producer["Trace ID"] == consumer["Trace ID"] == effect["Trace ID"]
            assert producer["ID"] == consumer["Parent ID"] and consumer["ID"] == effect["Parent ID"]
            assert len({producer["ID"], consumer["ID"], effect["ID"]}) == 3
            print("Standard collector verified TLS, producer/consumer/persistence ancestry and minimized metadata.")
        finally:
            if started:
                result = subprocess.run(["docker", "logs", name], text=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, timeout=10)
                output = Path("target/distributed-tracing")
                output.mkdir(parents=True, exist_ok=True)
                (output / "collector.log").write_text(result.stdout[-1024 * 1024:])
                subprocess.run(["docker", "rm", "--force", name], check=True, timeout=30, stdout=subprocess.DEVNULL)


if __name__ == "__main__":
    main()
