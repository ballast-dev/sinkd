UID := `id -u`
GID := `id -g`
TLD := `git rev-parse --show-toplevel`

# first rip, setup environment and do a build
all: image container build

# add yourself to the docker group for permissions
# sudo usermod -aG docker $(whoami)

# create image from Dockerfile
image:
    @docker build -t alpine -f Dockerfile src/

# spawn container with tld as /sinkd
container:
    @docker run \
      --name sinkd \
      --user {{UID}}:{{GID}} \
      -v {{TLD}}:/sinkd \
      -itd alpine    

# build app in container
build:
    @docker exec sinkd cargo build

# build app with no warnings in container
build-no-warn:
    @docker exec sinkd cargo rustc -- -Awarnings

# clean within container 
clean:
    @docker exec sinkd cargo clean

# deeper clean, rm container and image 
wipe: rm-container rm-image

rm-container:
    @docker container rm -f sinkd

# jump in!!! 
attach:
    # need to check if started 
    @docker container attach sinkd

rm-image:
    @docker rmi -f alpine

run *args:
    @./target/debug/sinkd {{args}}
