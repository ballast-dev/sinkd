FROM rust:1.90-alpine

RUN apk update && apk add \
  build-base \
  cmake \
  mosquitto-dev \
  openssl-dev \
  openssl-libs-static
