#!/usr/bin/env bash
# The runtime override must precede Rust's profiler archive so the linker does
# not pull its automatic initializer. This wrapper is used only by CI coverage.
set -euo pipefail
exec /usr/bin/cc "$(dirname "$0")/profile-runtime.o" "$@"
