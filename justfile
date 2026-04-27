IMAGE := "ghcr.io/ballast-dev/sinkd:0.1.0"
export ARCH := if `uname -m` == "x86_64" { "amd64" } else { "arm64" }

_: 
    @just --list

# ensure to run this only on native architecture
lint:
    cargo clippy --all-targets --all-features

# fast local lane: unit + local integration (no docker required)
test-local:
    cargo test --workspace

# containerized multi-client/single-server scenario (requires docker + docker compose)
test-compose:
    rm -rf test_scenarios/compose/_artifacts
    cargo run -p scenario -- --spec scenario/specs/compose.toml --root test_scenarios/compose

# full lane: local + compose
test:
    just test-local
    just test-compose

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
