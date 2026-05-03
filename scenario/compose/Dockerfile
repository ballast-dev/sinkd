# syntax=docker/dockerfile:1.7
# Two-stage image used by the multi-client compose scenario.
#
# Stage 1: build static musl sinkd + sinkd-srv and package as a .deb.
# Stage 2: install the .deb on a slim Debian base alongside rsync + helpers.
#
# Build context is the repo root (see scenario/compose/compose.yml).

FROM rust:1.92-bookworm AS build

RUN apt-get update \
 && apt-get install -y --no-install-recommends \
        musl-tools \
        dpkg-dev \
        ca-certificates \
        pkg-config \
 && rm -rf /var/lib/apt/lists/*

RUN rustup target add x86_64-unknown-linux-musl

WORKDIR /src
COPY . .

# Detect target arch from Debian and build the matching musl binary, then
# package it. SINKD_VERSION lets us skip the optional `bump` dependency.
ARG SINKD_VERSION=0.0.0-compose
ENV SINKD_VERSION=${SINKD_VERSION}
RUN set -eux; \
    case "$(dpkg --print-architecture)" in \
      amd64) MUSL_TARGET=x86_64-unknown-linux-musl ;; \
      arm64) MUSL_TARGET=aarch64-unknown-linux-musl ;; \
      *) echo "unsupported arch $(dpkg --print-architecture)" >&2; exit 1 ;; \
    esac; \
    rustup target add "${MUSL_TARGET}"; \
    cargo build --release --target "${MUSL_TARGET}" -p sinkd -p sinkd-srv; \
    SINKD_BIN="/src/target/${MUSL_TARGET}/release/sinkd" \
        bash pkg/debian/create.sh

FROM debian:bookworm-slim

RUN apt-get update \
 && apt-get install -y --no-install-recommends \
        rsync \
        ca-certificates \
        coreutils \
        findutils \
        bash \
 && rm -rf /var/lib/apt/lists/*

COPY --from=build /src/pkg/artifacts/sinkd_*.deb /tmp/
RUN dpkg -i /tmp/sinkd_*.deb && rm /tmp/sinkd_*.deb

COPY scenario/compose/entrypoint.sh /usr/local/bin/entrypoint.sh
COPY scenario/compose/snapshot.sh   /usr/local/bin/sinkd-snapshot
RUN chmod +x /usr/local/bin/entrypoint.sh /usr/local/bin/sinkd-snapshot

ENTRYPOINT ["/usr/local/bin/entrypoint.sh"]
