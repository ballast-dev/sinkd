FROM rust:1.65-alpine3.16
RUN apk add musl-dev openssl-libs-static openssl-dev perl make cmake

### other libs
# openssl3-libs-static-3.0.7-r0
# openssl-libs-static-1.1.1s-r1
# llvm-dev
# clang
# clang-static

RUN mkdir /sinkd
WORKDIR /sinkd
ENV USER=$(whoami)