"""TEMP-DIAG: write and fsync large files in the temp directory to load the disk.

Removed before the final commit.
"""
import os
import sys
import tempfile
import time

seconds = float(sys.argv[1])
name = os.path.join(tempfile.gettempdir(), f"rullst-io-pressure-{os.getpid()}.bin")
chunk = os.urandom(8 * 1024 * 1024)
deadline = time.monotonic() + seconds
written = 0
while time.monotonic() < deadline:
    with open(name, "wb") as handle:
        for _ in range(64):  # 512 MiB per file
            handle.write(chunk)
            handle.flush()
            os.fsync(handle.fileno())
            written += len(chunk)
            if time.monotonic() >= deadline:
                break
    os.remove(name)
print(f"pressure {os.getpid()} wrote {written // (1024 * 1024)} MiB", flush=True)
