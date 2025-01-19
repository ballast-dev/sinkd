arch_var := arch()

set windows-shell := ["powershell.exe", "-NoLogo", "-Command"]

export OPENSSL_DIR = if os_family() == "windows" {
  if arch_var == "x86_64" {
    "C:\\Program Files\\OpenSSL-Win64"
  } else {
    "C:\\Program Files\\OpenSSL-Win64-ARM"
  }
} else {
  ""
}

export OPENSSL_LIB_DIR = if os_family() == "windows" {
  if arch_var == "x86_64" {
    "C:\\Program Files\\OpenSSL-Win64\\lib\\VC\\x64\\MT"
  } else {
    "C:\\Program Files\\OpenSSL-Win64-ARM\\lib\\VC\\arm64\\MT"
  }
} else {
  ""
}

export OPENSSL_INCLUDE_DIR = if os_family() == "windows" {
  if arch_var == "x86_64" {
    "C:\\Program Files\\OpenSSL-Win64\\include"
  } else {
    "C:\\Program Files\\OpenSSL-Win64-ARM\\include"
  }
} else {
  ""
}

export OPENSSL_STATIC = if os_family() == "windows" { "1" } else { "" }

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

build:
  @cargo build

# Clean the project
clean:
  cargo clean

# Run the project (conditional environment setup)
run args:
  @cargo run


#   _               
#  / )   _/_ '  _ _ 
# (__()/)/(///)(-/  
#


# UID := `id -u`
# GID := `id -g`
# TLD := `git rev-parse --show-toplevel`

# first rip, setup environment and do a build
# all: image container build

# add yourself to the docker group for permissions
# sudo usermod -aG docker $(whoami)

# # create image from Dockerfile
# image:
#     @docker build -t alpine -f Dockerfile src/

# # spawn container with tld as /sinkd
# container:
#     @docker run --name sinkd --user {{UID}}:{{GID}} -v {{TLD}}:/sinkd  -itd alpine    

# # build app in container
# build:
#     @docker exec sinkd cargo build

# # build app with no warnings in container
# build-no-warn:
#     @docker exec sinkd cargo rustc -- -Awarnings

# # clean within container 
# clean:
#     @docker exec sinkd cargo clean

# # deeper clean, rm container and image 
# wipe: rm-container rm-image

# rm-container:
#     @docker container rm -f sinkd

# # jump in!!!
# attach:
#     # need to check if started 
#     @docker container attach sinkd

# rm-image:
#     @docker rmi -f alpine

# # run *args: build-no-warn
# #     @./target/debug/sinkd {{args}}

# start:
#     sudo systemctl start docker
#     docker container start sinkd