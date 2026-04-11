#!/usr/bin/env bash
# CLI contract tests for sinkd (subcommand_required, globals, domain flags, server/client ls).
# Usage: cli_smoke.sh HARNESS_DIR — cwd must be repo root (see justfile).
set -euo pipefail

HARNESS="${1:?usage: cli_smoke.sh HARNESS_DIR}"

REPO_ROOT="$(git rev-parse --show-toplevel 2>/dev/null || true)"
if [[ -z "${REPO_ROOT}" ]]; then
  echo "error: need git repo root (git rev-parse failed)" >&2
  exit 2
fi

cd "$REPO_ROOT"
cargo build -q -p sinkd
TARGET_DIR="${CARGO_TARGET_DIR:-$REPO_ROOT/target}"
SINKD="$TARGET_DIR/debug/sinkd"

if [[ ! -f "$SINKD" ]]; then
  echo "error: sinkd binary not found at $SINKD" >&2
  exit 2
fi

fail() {
  echo "error: $*" >&2
  exit 1
}

# No subcommand: must fail (clap subcommand_required)
if "$SINKD" 2>/dev/null; then
  fail "bare sinkd should exit non-zero"
fi

# Root help / version
"$SINKD" --help | grep -q "Sync daemon" || fail "root --help missing about text"
"$SINKD" --version | grep -qE '[0-9]+\.[0-9]' || fail "--version missing semver-like text"

# Subcommand help
"$SINKD" client --help | grep -q "Client side" || fail "client --help"
"$SINKD" server --help | grep -q "Server side" || fail "server --help"

# Server ls (no long-running daemon): prints sync root
OUT=$("$SINKD" -d server ls 2>&1) || fail "sinkd -d server ls failed"
echo "$OUT" | grep -q "server sync root" || fail "server ls missing sync root line: $OUT"

# Global -d before subcommand
OUT2=$("$SINKD" -d server ls 2>&1) || fail "sinkd -d server ls (global -d) failed"
echo "$OUT2" | grep -q "server sync root" || fail "server ls with global -d: $OUT2"

# Client: minimal config + --client-state-dir after `client`
mkdir -p "$HARNESS/cfg" "$HARNESS/watch" "$HARNESS/state"
STATE="$(cd "$HARNESS/state" && pwd)"
CFG="$(cd "$HARNESS/cfg" && pwd)"
WATCH="$(cd "$HARNESS/watch" && pwd)"
U="$(id -un)"

cat > "$CFG/system.toml" <<EOF
server_addr = "localhost"
users = ["${U}"]
EOF

cat > "$CFG/user.toml" <<EOF
[[anchors]]
path = "${WATCH}"
interval = 1
excludes = []
EOF

SYS="$CFG/system.toml"
USER="$CFG/user.toml"

"$SINKD" -d client --client-state-dir "$STATE" -s "$SYS" -u "$USER" ls >/dev/null \
  || fail "client ls with client-state-dir after client"

echo "cli_smoke OK"
