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

# distributed lane with real container events
test-e2e:
    @if docker info > /dev/null 2>&1; then \
        cargo run -p scenario -- --spec scenario/specs/distributed_edge.toml --root . ; \
    else \
        echo "Skipping test-e2e: Docker daemon unavailable" ; \
    fi

# full suite: local lane + declarative harness smoke
test:
    just test-local
    rm -rf test_scenarios/harness
    cargo run -p scenario -- --spec scenario/specs/local_smoke.toml --root test_scenarios/harness
    just test-e2e

# the following commands are purely for debugging
client:
    cargo run -- -d client -s cfg/opt/sinkd/sinkd.conf -u cfg/user/sinkd.conf start

client-log:
    tail -f /tmp/sinkd/client.log

server:
    cargo run -- -d server

server-log:
    tail -f /tmp/sinkd/server.log


# build docker image
img:
    @docker buildx build \
    --platform linux/amd64,linux/arm64 \
    -t {{IMAGE}} \
    -< Dockerfile

img-push:
    @docker buildx build \
    --platform linux/amd64,linux/arm64 \
    -t {{IMAGE}} \
    --push \
    -< Dockerfile


sh *ARGS:
    @docker run -it --rm \
        --hostname sinkd \
        -e WORKDIR=$(pwd) \
        -v $(pwd):$(pwd) \
        -w $(pwd) \
        {{ARGS}} \
        {{IMAGE}}

# build binaries inside container (for Alpine/musl compatibility)
build:
    @docker run --rm \
        --hostname sinkd \
        -e WORKDIR=$(pwd) \
        -e CARGO_TARGET_DIR=$(pwd)/target/docker \
        -v $(pwd):$(pwd) \
        -w $(pwd) \
        {{IMAGE}} \
        "cargo clean"
    @docker run --rm \
        --hostname sinkd \
        -e WORKDIR=$(pwd) \
        -e CARGO_TARGET_DIR=$(pwd)/target/docker \
        -v $(pwd):$(pwd) \
        -w $(pwd) \
        {{IMAGE}} \
        "cargo build --workspace"


##################################
## Docker Multi-Instance Commands
##################################

# start the multi-instance docker setup
scenario:
    just build
    docker compose up -d

# stop the multi-instance docker setup
scenario-down:
    docker compose down

# view logs from all instances
scenario-logs:
    docker compose logs -f
