#!/usr/bin/env bash
# Driver for the multi-client / single-server compose scenario.
#
# Usage:  compose.sh   (run from repo root; logs/manifests -> test/scenario/compose/)
#
# Steps:
#   1. `docker compose build` the scenario image.
#   2. Bring up `zenoh` + `server`, give them a moment to settle.
#   3. Bring up `client-alice` + `client-bob`.
#   4. Wait for each client to drop /<watch>/.events_done.
#   5. Wait for `current_generation` on the server to advance past the burst.
#   6. Checkpoint A: snapshot-diff each client's watch tree against
#      `/srv/sinkd<watch>` on the server (rsync `-R` preserves absolute paths).
#   7. Fire a second wave of events via `docker compose exec`.
#   8. Checkpoint B: snapshot-diff again.
#   9. `docker compose down -v` (in EXIT trap) tears the world down.
#
# Manifests + logs are written under test/scenario/compose/.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
ARTIFACTS="${REPO_ROOT}/test/scenario/compose"
mkdir -p "${ARTIFACTS}"

COMPOSE_FILE="${REPO_ROOT}/scenario/compose/compose.yml"
PROJECT_NAME="sinkd_compose"
COMPOSE=(docker compose -f "${COMPOSE_FILE}" -p "${PROJECT_NAME}")

log() { printf '[compose] %s\n' "$*"; }

cleanup() {
    local rc=$?
    "${COMPOSE[@]}" logs --no-color > "${ARTIFACTS}/compose.log" 2>&1 || true
    for svc in server client-alice client-bob; do
        "${COMPOSE[@]}" exec -T "${svc}" \
            bash -c 'for f in /var/log/sinkd/*.log; do echo ===== "$f" =====; cat "$f"; done' \
            > "${ARTIFACTS}/${svc}.sinkd.log" 2>&1 || true
    done
    "${COMPOSE[@]}" exec -T server \
        bash -c 'cat /srv/sinkd/generation_state.toml 2>/dev/null' \
        > "${ARTIFACTS}/server.generation_state.toml" 2>&1 || true
    if [[ "${COMPOSE_KEEP:-0}" == "1" && $rc -ne 0 ]]; then
        log "COMPOSE_KEEP=1 and rc=$rc; leaving stack running for inspection"
        return
    fi
    log "tearing down compose stack"
    "${COMPOSE[@]}" down -v --remove-orphans >/dev/null 2>&1 || true
}
trap cleanup EXIT

require() {
    if ! command -v "$1" >/dev/null 2>&1; then
        echo "compose.sh: required tool not found: $1" >&2
        exit 127
    fi
}
require docker
if ! docker compose version >/dev/null 2>&1; then
    echo "compose.sh: 'docker compose' plugin is required" >&2
    exit 127
fi

log "building image (first run can take several minutes; progress follows)"
"${COMPOSE[@]}" build --pull

log "starting zenoh + server"
"${COMPOSE[@]}" up -d zenoh server
sleep 5

# Start one client at a time. If both run concurrently, a NotReady(Behind) pull
# against an empty server mirror for the other user runs `rsync --delete` and
# wipes the watch tree before phase A finishes (rename then fails).
log "starting client-alice"
"${COMPOSE[@]}" up -d client-alice

# wait_for_path <service> <path> [timeout-sec]
wait_for_path() {
    local svc="$1" path="$2" timeout="${3:-180}"
    local elapsed=0
    while (( elapsed < timeout )); do
        if "${COMPOSE[@]}" exec -T "${svc}" test -f "${path}" >/dev/null 2>&1; then
            return 0
        fi
        sleep 2
        elapsed=$(( elapsed + 2 ))
    done
    echo "compose.sh: timed out waiting ${timeout}s for ${svc}:${path}" >&2
    "${COMPOSE[@]}" logs --tail 200 "${svc}" >&2 || true
    return 1
}

log "waiting for client-alice events_done"
wait_for_path client-alice /watch_alice/.events_done 300

log "starting client-bob"
"${COMPOSE[@]}" up -d client-bob
log "waiting for client-bob events_done"
wait_for_path client-bob /watch_bob/.events_done 300

# wait_for_generation <min-generation> [timeout-sec]
wait_for_generation() {
    local minimum="$1" timeout="${2:-120}"
    local elapsed=0 cur=0
    while (( elapsed < timeout )); do
        cur=$("${COMPOSE[@]}" exec -T server sh -c \
            "awk -F'=' '/^current_generation/ {gsub(/[ \"]/,\"\",\$2); print \$2}' /srv/sinkd/generation_state.toml 2>/dev/null" \
            | tr -d '\r' || true)
        if [[ -n "${cur}" && "${cur}" -ge "${minimum}" ]]; then
            log "server current_generation=${cur} (>= ${minimum})"
            return 0
        fi
        sleep 2
        elapsed=$(( elapsed + 2 ))
    done
    echo "compose.sh: server generation never reached ${minimum} (last=${cur:-?})" >&2
    "${COMPOSE[@]}" logs --tail 200 server >&2 || true
    return 1
}

log "waiting for server generation to catch up to phase A"
wait_for_generation 4 180

# Clients may enter NotReady(Behind) and run pull_behind against the server mirror.
# Wait until generation stops moving so the last server-side rsync has finished
# before we snapshot-compare trees.
log "waiting for server generation to settle before checkpoint A"
settled=0
prev=""
for _ in $(seq 1 45); do
    cur=$("${COMPOSE[@]}" exec -T server sh -c \
        "awk -F'=' '/^current_generation/ {gsub(/[ \"]/,\"\",\$2); print \$2}' /srv/sinkd/generation_state.toml 2>/dev/null" \
        | tr -d '\r' || true)
    if [[ -n "${cur}" && "${cur}" == "${prev}" ]]; then
        settled=$(( settled + 1 ))
        if (( settled >= 4 )); then
            log "server generation stable at ${cur}"
            break
        fi
    else
        settled=0
        prev="${cur}"
    fi
    sleep 2
done
sleep 6

# Ensure the server has applied at least one rsync per client tree (`-R` layout).
wait_for_server_mirror_dir() {
    local path="$1" timeout="${2:-180}"
    local elapsed=0
    while (( elapsed < timeout )); do
        if "${COMPOSE[@]}" exec -T server test -d "${path}" >/dev/null 2>&1; then
            log "server mirror ready: ${path}"
            return 0
        fi
        sleep 2
        elapsed=$(( elapsed + 2 ))
    done
    echo "compose.sh: timed out waiting for server dir ${path}" >&2
    "${COMPOSE[@]}" logs --tail 120 server >&2 || true
    return 1
}

wait_for_server_mirror_dir /srv/sinkd/watch_alice 240
wait_for_server_mirror_dir /srv/sinkd/watch_bob 240

# snapshot_diff <user> <watch-path> <checkpoint-label>
snapshot_diff() {
    local user="$1" watch="$2" label="$3"
    local client_manifest="${ARTIFACTS}/${label}-${user}.client.manifest"
    local server_manifest="${ARTIFACTS}/${label}-${user}.server.manifest"

    "${COMPOSE[@]}" exec -T "client-${user}" sinkd-snapshot "${watch}" \
        > "${client_manifest}"
    # rsync -R preserves absolute paths -> /srv/sinkd<watch> on the server.
    "${COMPOSE[@]}" exec -T server sinkd-snapshot "/srv/sinkd${watch}" \
        > "${server_manifest}"

    if ! diff -u "${client_manifest}" "${server_manifest}"; then
        echo "compose.sh: ${label} drift for ${user}" >&2
        echo "  client manifest: ${client_manifest}" >&2
        echo "  server manifest: ${server_manifest}" >&2
        return 1
    fi
    log "${label} ${user}: tree equivalent"
}

log "checkpoint A"
snapshot_diff alice /watch_alice A
snapshot_diff bob   /watch_bob   A

log "phase B: second wave of events"
"${COMPOSE[@]}" exec -T client-alice bash -c '
    set -euo pipefail
    rm -f /watch_alice/file_alice_renamed.txt
'
sleep 3
"${COMPOSE[@]}" exec -T client-alice bash -c '
    set -euo pipefail
    echo "alice extra" > /watch_alice/extra_alice.txt
'
sleep 3
"${COMPOSE[@]}" exec -T client-bob bash -c '
    set -euo pipefail
    mv /watch_bob/file_bob_3.txt /watch_bob/file_bob_3_renamed.txt
'

log "settling for phase B"
sleep 20

log "waiting for server generation to catch up to phase B"
wait_for_generation 10 180

log "waiting for server generation to settle before checkpoint B"
settled=0
prev=""
for _ in $(seq 1 45); do
    cur=$("${COMPOSE[@]}" exec -T server sh -c \
        "awk -F'=' '/^current_generation/ {gsub(/[ \"]/,\"\",\$2); print \$2}' /srv/sinkd/generation_state.toml 2>/dev/null" \
        | tr -d '\r' || true)
    if [[ -n "${cur}" && "${cur}" == "${prev}" ]]; then
        settled=$(( settled + 1 ))
        if (( settled >= 4 )); then
            log "server generation stable at ${cur}"
            break
        fi
    else
        settled=0
        prev="${cur}"
    fi
    sleep 2
done
sleep 6

log "checkpoint B"
snapshot_diff alice /watch_alice B
snapshot_diff bob   /watch_bob   B

log "compose scenario OK"
