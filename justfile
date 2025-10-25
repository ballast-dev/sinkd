# run linter with strict flags
clippy:
    cargo clippy --fix --allow-dirty --allow-staged \
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


# build docker image
img:
    @docker build -t sinkd -< Dockerfile

# spawn container
sh:
    docker run -it --rm \
        --hostname sinkd \
        --workdir $(pwd) \
        -v $(pwd):$(pwd) \
        sinkd