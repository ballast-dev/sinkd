#!/usr/bin/env bash
# Integrates with scenario_runner: asserts server generation_state advances after a client file change.
# Usage: sinkd_generation_smoke.sh HARNESS_DIR
# Expects cwd to be the sinkd repo root (see justfile / cargo run -p scenario).
set -euo pipefail

HARNESS="${1:?usage: sinkd_generation_smoke.sh HARNESS_DIR}"

REPO_ROOT="$(git rev-parse --show-toplevel 2>/dev/null || true)"
if [[ -z "${REPO_ROOT}" ]]; then
  echo "error: need git repo root (git rev-parse failed)" >&2
  exit 2
fi

mkdir -p "$HARNESS/watch" "$HARNESS/cfg"
U="$(id -un)"

cat > "$HARNESS/cfg/system.toml" <<EOF
server_addr = "localhost"
users = ["${U}"]
EOF

WATCH_ABS="$(cd "$HARNESS/watch" && pwd)"
cat > "$HARNESS/cfg/user.toml" <<EOF
[[anchors]]
path = "${WATCH_ABS}"
interval = 1
excludes = []
EOF

rm -rf /tmp/sinkd

CFG_ABS="$(cd "$HARNESS/cfg" && pwd)"
SYS_TOML="$CFG_ABS/system.toml"
USER_TOML="$CFG_ABS/user.toml"

TARGET_DIR="${CARGO_TARGET_DIR:-$REPO_ROOT/target}"
(cd "$REPO_ROOT" && cargo build -q -p sinkd)
SINKD="$TARGET_DIR/debug/sinkd"

"$SINKD" -d server start
"$SINKD" -d client -s "$SYS_TOML" -u "$USER_TOML" start

sleep 5
echo "harness $(date +%s)" > "$HARNESS/watch/smoke.txt"

GEN="/tmp/sinkd/srv/generation_state.toml"
ok=0
cur=""
for _ in $(seq 1 120); do
  if [[ -f "$GEN" ]]; then
    cur=$(grep -E '^current_generation[[:space:]]*=' "$GEN" | head -1 | sed -E 's/^[^=]*=[[:space:]]*//;s/[[:space:]]*$//')
    if [[ -n "${cur:-}" && "${cur}" =~ ^[0-9]+$ && "${cur}" -ge 1 ]]; then
      ok=1
      break
    fi
  fi
  sleep 1
done

"$SINKD" -d client -s "$SYS_TOML" -u "$USER_TOML" stop 2>/dev/null || true
"$SINKD" -d server stop 2>/dev/null || true

if [[ "$ok" != 1 ]]; then
  echo "error: timed out waiting for current_generation >= 1 in $GEN" >&2
  if [[ -f "$GEN" ]]; then
    cat "$GEN" >&2
  else
    echo "(file missing; listing /tmp/sinkd/srv)" >&2
    ls -la /tmp/sinkd/srv 2>/dev/null || true
  fi
  exit 1
fi

echo "sinkd generation smoke OK (current_generation=$cur)"
