#!/usr/bin/env bash
# Compose-scenario entrypoint. Branches on $ROLE:
#   server  -> `sinkd-srv init` + `sinkd-srv start` + tail log
#   client  -> `sinkd init` + `sinkd start` + deterministic
#              write/rename/delete loop + drop /WATCH/.events_done sentinel +
#              tail log
set -euo pipefail

# debian:bookworm-slim leaves $USER unset for root, but sinkd's Payload::new()
# calls config::get_username() which requires it. Default to the role-specific
# value so the daemon (forked off and inheriting our env) survives.
case "${ROLE:-}" in
    server) export USER="${USER:-server}" ;;
    client) export USER="${USER:-${USER_NAME:-sinkd-client}}" ;;
esac

log() { printf '[entrypoint:%s] %s\n' "${ROLE:-?}" "$*"; }

wait_for_log() {
    local path="$1"
    for _ in $(seq 1 50); do
        [[ -f "$path" ]] && return 0
        sleep 0.1
    done
    return 1
}

case "${ROLE:-}" in
    server)
        users="${SINKD_USERS:-alice,bob}"

        log "init system config (users=${users})"
        sinkd-srv init --users "${users}" --force

        log "starting daemon"
        sinkd-srv start

        wait_for_log /var/log/sinkd/server.log || {
            log "server log never appeared"; exit 1;
        }
        log "tailing server log"
        exec tail -F /var/log/sinkd/server.log
        ;;

    client)
        : "${USER_NAME:?USER_NAME required for ROLE=client}"
        : "${WATCH:?WATCH required for ROLE=client}"
        # For compose, `SINKD_SERVER_ADDR=/srv/sinkd` matches the read-only
        # `srv-data` mount so Behind pulls use local rsync (see pull_behind).
        SINKD_SERVER_ADDR="${SINKD_SERVER_ADDR:-server}"
        interval="${SINKD_INTERVAL:-1}"

        mkdir -p "${WATCH}"

        log "init configs (server_addr=${SINKD_SERVER_ADDR}, user=${USER_NAME}, watch=${WATCH})"
        sinkd init \
            --server-addr "${SINKD_SERVER_ADDR}" \
            --user "${USER_NAME}" \
            --watch "${WATCH}" \
            --interval "${interval}" \
            --force

        log "starting daemon"
        sinkd start

        wait_for_log /var/log/sinkd/client.log || {
            log "client log never appeared"; exit 1;
        }

        # Settle: let the daemon connect to the server and complete the initial
        # handshake before we start producing events.
        sleep 8

        log "phase A: 5 sequential writes"
        for i in 1 2 3 4 5; do
            echo "${USER_NAME} write $i $(date +%s)" \
                > "${WATCH}/file_${USER_NAME}_$i.txt"
            sleep 0.5
        done

        log "phase A: rename file_1 -> file_renamed"
        mv "${WATCH}/file_${USER_NAME}_1.txt" \
           "${WATCH}/file_${USER_NAME}_renamed.txt"
        sleep 1

        log "phase A: delete file_2"
        rm "${WATCH}/file_${USER_NAME}_2.txt"
        sleep 1

        log "phase A: append to file_3"
        echo "${USER_NAME} append $(date +%s)" \
            >> "${WATCH}/file_${USER_NAME}_3.txt"

        # Settle so the server applies the trailing event before the host runner
        # snapshots.
        sleep 4

        log "phase A complete; dropping sentinel"
        : > "${WATCH}/.events_done"

        log "tailing client log"
        exec tail -F /var/log/sinkd/client.log
        ;;

    *)
        echo "unknown ROLE='${ROLE:-}'" >&2
        exit 2
        ;;
esac
