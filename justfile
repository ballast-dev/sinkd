IMAGE_VERSION := "0.1.0"
ARCH := if `uname -m` == "x86_64" { "amd64" } else { "arm64" }
IMAGE_NAME := "registry.gitlab.com/ballast-dev/sinkd"

_:
    @just --list

_version:
    #!/usr/bin/env bash
    set -e
    VERSION=$(bump --print-base)
    sed -i "s|^version = \".*\"|version = \"${VERSION}\"|g" sinkd/Cargo.toml

# run linter with strict flags
clippy:
    cargo clippy --fix --allow-dirty --allow-staged --all-targets --all-features \
    -- -W clippy::perf -D clippy::pedantic -D clippy::correctness -D clippy::suspicious -D clippy::complexity

# the following commands are purely for debugging
client:
    cargo run -- -d client -s cfg/opt/sinkd/sinkd.conf -u cfg/user/sinkd.conf start

client-log:
    tail -f /tmp/sinkd/client.log

server:
    cargo run -- -d server

server-log:
    tail -f /tmp/sinkd/server.log

# ensure to run this only on native architecture
lint:
    cargo clippy --all-targets --all-features -- \
    -W clippy::perf -D clippy::pedantic -D clippy::correctness -D clippy::suspicious -D clippy::complexity


## Build Environment

# build docker image
img ARCH=ARCH:
    @echo "Building docker image for {{ARCH}}"
    @docker build --platform linux/{{ARCH}} \
        -t {{IMAGE_NAME}}/{{ARCH}}:{{IMAGE_VERSION}} \
        -< Dockerfile

# spawn container
_docker_run ARCH *ARGS:
    @docker run -it --rm \
        --platform linux/{{ARCH}} \
        --hostname sinkd \
        -e WORKDIR=$(pwd) \
        -v $(pwd):$(pwd) \
        {{IMAGE_NAME}}/{{ARCH}}:{{IMAGE_VERSION}} \
        {{ARGS}}

build ARCH=ARCH: (_docker_run ARCH 'cargo build')
sh ARCH=ARCH: (_docker_run ARCH '/bin/bash')

root ARCH=ARCH:
    @docker run -it --rm \
        --platform linux/{{ARCH}} \
        --hostname sinkd \
        -v $(pwd):$(pwd) \
        --workdir $(pwd) \
        --entrypoint "" \
        {{IMAGE_NAME}}/{{ARCH}}:{{IMAGE_VERSION}} \
        /bin/sh


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
