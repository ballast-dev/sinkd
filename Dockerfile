FROM rust:1.81-alpine3.20

RUN apk update && apk add \
  build-base \
  cmake \
  openssl-dev \
  perl \
  vim
