FROM rust:1.65-alpine3.16
RUN apk add musl-dev openssl-dev make cmake
COPY . /sinkd
WORKDIR /sinkd
ENV USER=tony
RUN cargo fetch