#!/usr/bin/env bash
# End-to-end: victim establishes gen@1, pusher advances to gen@2, victim returns with
# stale ack, local edit marks dirty → behind pull uses rsync --backup-dir (see client logs).
# Asserts client state layout: behind_backups/<N>/ and log line naming the backup path for STATE_V.
# Expects cwd = repo root; uses -d (debug) and distinct --client-state-dir trees.
set -euo pipefail

HARNESS="${1:?usage: behind_pull_conflict_backup.sh HARNESS_DIR}"

REPO_ROOT="$(git rev-parse --show-toplevel 2>/dev/null || true)"
if [[ -z "${REPO_ROOT}" ]]; then
  echo "error: need git repo root (git rev-parse failed)" >&2
  exit 2
fi

mkdir -p "$HARNESS/watch" "$HARNESS/state_v" "$HARNESS/state_p" "$HARNESS/cfg"
rm -rf /tmp/sinkd

STATE_V="$(cd "$HARNESS/state_v" && pwd)"
STATE_P="$(cd "$HARNESS/state_p" && pwd)"
CFG_ABS="$(cd "$HARNESS/cfg" && pwd)"
SYS_TOML="$CFG_ABS/system.toml"
USER_TOML="$CFG_ABS/user.toml"

WATCH_ABS="$(cd "$HARNESS/watch" && pwd)"
U="$(id -un)"

cat > "$SYS_TOML" <<EOF
server_addr = "localhost"
users = ["${U}"]
EOF

cat > "$USER_TOML" <<EOF
[[anchors]]
path = "${WATCH_ABS}"
interval = 1
excludes = []
EOF

TARGET_DIR="${CARGO_TARGET_DIR:-$REPO_ROOT/target}"
(cd "$REPO_ROOT" && cargo build -q -p sinkd)
SINKD="$TARGET_DIR/debug/sinkd"

wait_gen_at_least() {
  local want="$1"
  local GEN="/tmp/sinkd/srv/generation_state.toml"
  local cur="" ok=0
  for _ in $(seq 1 180); do
    if [[ -f "$GEN" ]]; then
      cur=$(grep -E '^current_generation[[:space:]]*=' "$GEN" | head -1 | sed -E 's/^[^=]*=[[:space:]]*//;s/[[:space:]]*$//')
      if [[ -n "${cur:-}" && "${cur}" =~ ^[0-9]+$ && "${cur}" -ge "$want" ]]; then
        ok=1
        break
      fi
    fi
    sleep 1
  done
  if [[ "$ok" != 1 ]]; then
    echo "error: timed out waiting for current_generation >= $want (got ${cur:-<empty>})" >&2
    [[ -f "$GEN" ]] && cat "$GEN" >&2 || true
    return 1
  fi
  return 0
}

cleanup() {
  "$SINKD" -d client --client-state-dir "$STATE_V" -s "$SYS_TOML" -u "$USER_TOML" stop 2>/dev/null || true
  "$SINKD" -d client --client-state-dir "$STATE_P" -s "$SYS_TOML" -u "$USER_TOML" stop 2>/dev/null || true
  "$SINKD" -d server stop 2>/dev/null || true
}
trap cleanup EXIT

echo "phase1: server + victim → generation >= 1"
"$SINKD" -d server start
sleep 2
"$SINKD" -d client --client-state-dir "$STATE_V" -s "$SYS_TOML" -u "$USER_TOML" start
sleep 6
echo "victim boot $(date +%s)" > "$HARNESS/watch/conflict.txt"
wait_gen_at_least 1

"$SINKD" -d client --client-state-dir "$STATE_V" -s "$SYS_TOML" -u "$USER_TOML" stop 2>/dev/null || true
sleep 2

if [[ ! -f "$STATE_V/acked_generation" ]]; then
  echo "error: expected $STATE_V/acked_generation after victim sync" >&2
  exit 1
fi
echo "victim acked_generation=$(cat "$STATE_V/acked_generation")"

echo "phase2: pusher advances server (Behind pull then push → gen >= 2)"
"$SINKD" -d client --client-state-dir "$STATE_P" -s "$SYS_TOML" -u "$USER_TOML" start
sleep 10
echo "pusher line $(date +%s)" >> "$HARNESS/watch/conflict.txt"
sleep 6
echo "pusher line2 $(date +%s)" >> "$HARNESS/watch/conflict.txt"
sleep 18
wait_gen_at_least 2

"$SINKD" -d client --client-state-dir "$STATE_P" -s "$SYS_TOML" -u "$USER_TOML" stop 2>/dev/null || true
sleep 2

echo "phase3: victim returns (stale basis), local append → behind pull with backup dir"
: > /tmp/sinkd/client.log
"$SINKD" -d client --client-state-dir "$STATE_V" -s "$SYS_TOML" -u "$USER_TOML" start
sleep 8
echo "local dirty $(date +%s)" >> "$HARNESS/watch/conflict.txt"

LOG=/tmp/sinkd/client.log
MARKER="behind pull with local edits pending; rsync backups will use ${STATE_V}"
ok=0
for _ in $(seq 1 150); do
  if [[ -f "$LOG" ]] && grep -qF "$MARKER" "$LOG" 2>/dev/null; then
    ok=1
    break
  fi
  sleep 1
done

if [[ "$ok" != 1 ]]; then
  echo "error: timed out waiting for victim behind-pull backup log line (STATE_V=$STATE_V)" >&2
  [[ -f "$LOG" ]] && tail -80 "$LOG" >&2 || echo "(no $LOG)" >&2
  exit 1
fi

if [[ ! -d "$STATE_V/behind_backups" ]]; then
  echo "error: expected directory $STATE_V/behind_backups" >&2
  exit 1
fi
shopt -s nullglob
dirs=("$STATE_V"/behind_backups/[0-9]*)
shopt -u nullglob
if [[ ${#dirs[@]} -eq 0 ]]; then
  echo "error: expected behind_backups/<N> under $STATE_V" >&2
  ls -la "$STATE_V" >&2 || true
  exit 1
fi

if grep -q "create backup run.*File exists" "$LOG" 2>/dev/null; then
  echo "error: backup dir allocation should not log EEXIST after retry (see $LOG)" >&2
  exit 1
fi

echo "backup log + layout OK (${#dirs[@]} run dir(s) under behind_backups)"
find "$STATE_V/behind_backups" -type f 2>/dev/null | head -5 || true

trap - EXIT
cleanup

echo "behind_pull_conflict_backup scenario passed"
