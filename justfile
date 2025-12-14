IMAGE := "ghcr.io/ballast-dev/sinkd:0.1.0"
export ARCH := if `uname -m` == "x86_64" { "amd64" } else { "arm64" }

_: 
    @just --list

# ensure to run this only on native architecture
lint:
    cargo clippy --all-targets --all-features

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
        -v $(pwd):$(pwd) \
        -w $(pwd) \
        {{IMAGE}} \
        "cargo build --workspace"


##################################
## Docker Multi-Instance Commands
##################################

# start the multi-instance docker setup
scenario:
    docker compose up -d

# stop the multi-instance docker setup
scenario-down:
    docker compose down

# view logs from all instances
scenario-logs:
    docker compose logs -f
