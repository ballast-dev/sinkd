#!/usr/bin/env bash
# Two clients (alice / bob), interleaved writes, shared Zenoh on localhost.
# Uses `sinkd --client-state-dir` so each daemon has its own client_id (see `client::client_state_dir`).
set -euo pipefail

HARNESS="${1:?usage: multi_client_interleave.sh HARNESS_DIR}"

REPO_ROOT="$(git rev-parse --show-toplevel 2>/dev/null || true)"
if [[ -z "${REPO_ROOT}" ]]; then
  echo "error: need git repo root (git rev-parse failed)" >&2
  exit 2
fi

mkdir -p "$HARNESS/watch_a" "$HARNESS/watch_b" "$HARNESS/cfg" "$HARNESS/state_a" "$HARNESS/state_b"
rm -rf /tmp/sinkd
# Daemon chdirs to /; state dirs must be absolute.
STATE_A="$(cd "$HARNESS/state_a" && pwd)"
STATE_B="$(cd "$HARNESS/state_b" && pwd)"
CFG_ABS="$(cd "$HARNESS/cfg" && pwd)"
SYS_TOML="$CFG_ABS/system.toml"
USER_ALICE="$CFG_ABS/user_alice.toml"
USER_BOB="$CFG_ABS/user_bob.toml"

WATCH_A="$(cd "$HARNESS/watch_a" && pwd)"
WATCH_B="$(cd "$HARNESS/watch_b" && pwd)"

cat > "$SYS_TOML" <<EOF
server_addr = "localhost"
users = ["alice", "bob"]
EOF

cat > "$USER_ALICE" <<EOF
users = ["alice"]
[[anchors]]
path = "${WATCH_A}"
interval = 1
excludes = []
EOF

cat > "$USER_BOB" <<EOF
users = ["bob"]
[[anchors]]
path = "${WATCH_B}"
interval = 1
excludes = []
EOF

TARGET_DIR="${CARGO_TARGET_DIR:-$REPO_ROOT/target}"
(cd "$REPO_ROOT" && cargo build -q -p sinkd)
SINKD="$TARGET_DIR/debug/sinkd"

cleanup() {
  "$SINKD" -d --client-state-dir "$STATE_A" client -s "$SYS_TOML" -u "$USER_ALICE" stop 2>/dev/null || true
  "$SINKD" -d --client-state-dir "$STATE_B" client -s "$SYS_TOML" -u "$USER_BOB" stop 2>/dev/null || true
  "$SINKD" -d server stop 2>/dev/null || true
}
trap cleanup EXIT

"$SINKD" -d server start
"$SINKD" -d --client-state-dir "$STATE_A" client -s "$SYS_TOML" -u "$USER_ALICE" start
"$SINKD" -d --client-state-dir "$STATE_B" client -s "$SYS_TOML" -u "$USER_BOB" start

sleep 6

for i in $(seq 1 8); do
  echo "alice round $i $(date +%s)" >> "$HARNESS/watch_a/interleave.txt"
  sleep 0.35
  echo "bob round $i $(date +%s)" >> "$HARNESS/watch_b/interleave.txt"
  sleep 0.35
done

GEN="/tmp/sinkd/srv/generation_state.toml"
ok=0
cur=""
for _ in $(seq 1 150); do
  if [[ -f "$GEN" ]]; then
    cur=$(grep -E '^current_generation[[:space:]]*=' "$GEN" | head -1 | sed -E 's/^[^=]*=[[:space:]]*//;s/[[:space:]]*$//')
    if [[ -n "${cur:-}" && "${cur}" =~ ^[0-9]+$ && "${cur}" -ge 3 ]]; then
      ok=1
      break
    fi
  fi
  sleep 1
done

if [[ "$ok" != 1 ]]; then
  echo "error: timed out waiting for current_generation >= 3 in $GEN (got ${cur:-<empty>})" >&2
  [[ -f "$GEN" ]] && cat "$GEN" >&2 || ls -la /tmp/sinkd/srv 2>/dev/null >&2 || true
  exit 1
fi

if [[ ! -s "$STATE_A/client_id" ]] || [[ ! -s "$STATE_B/client_id" ]]; then
  echo "error: expected distinct client_id files under state_a / state_b" >&2
  exit 1
fi

if [[ "$(cat "$STATE_A/client_id")" == "$(cat "$STATE_B/client_id")" ]]; then
  echo "error: alice and bob must not share the same client_id" >&2
  exit 1
fi

trap - EXIT
cleanup

echo "multi-client interleave OK (current_generation=$cur)"
