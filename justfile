IMAGE := "ghcr.io/ballast-dev/sinkd:0.1.0"
export ARCH := if `uname -m` == "x86_64" { "amd64" } else { "arm64" }

_: 
    @just --list

# ensure to run this only on native architecture
lint:
    cargo clippy --all-targets --all-features

# fast local lane: unit + local integration
test-local:
    cargo test --workspace

# unit/integration + CLI contract + generation smoke + multi-client interleave + behind-pull backup tree
test:
    just test-local
    rm -rf test_scenarios/cli
    cargo run -p scenario -- --spec scenario/specs/cli_smoke.toml --root test_scenarios/cli
    rm -rf test_scenarios/harness
    cargo run -p scenario -- --spec scenario/specs/sinkd_generation_smoke.toml --root test_scenarios/harness
    rm -rf test_scenarios/multi_client
    cargo run -p scenario -- --spec scenario/specs/multi_client_interleave.toml --root test_scenarios/multi_client
    rm -rf test_scenarios/behind_conflict
    cargo run -p scenario -- --spec scenario/specs/behind_pull_conflict_backup.toml --root test_scenarios/behind_conflict

# optional Zenoh router (see docker-compose.yml); use when debugging cross-host peers
zenoh:
    docker compose up -d zenoh

zenoh-down:
    docker compose down

# the following commands are purely for debugging
client:
    cargo run -p sinkd -- -d client -s cfg/system/sinkd.conf -u cfg/user/sinkd.conf start

client-log:
    tail -f /tmp/sinkd/client.log

server:
    cargo run -p sinkd -- -d server start

server-log:
    tail -f /tmp/sinkd/server.log


img:
    @docker buildx build \
    --platform linux/amd64,linux/arm64 \
    -t {{IMAGE}} \
    -< Dockerfile


sh *ARGS:
    @docker run -it --rm \
        --hostname sinkd \
        -e WORKDIR=$(pwd) \
        -v $(pwd):$(pwd) \
        -w $(pwd) \
        {{ARGS}} \
        {{IMAGE}}
