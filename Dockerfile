# Dev/CI image for sinkd: Alpine, musl toolchain, rsync, OpenSSH client.
# Default: runs commands as user `sinkd`. For CI: docker run --entrypoint="" … IMAGE cargo …
FROM rust:1.92-alpine

RUN apk add --no-cache \
        build-base \
        curl \
        just \
        musl-dev \
        openssh-client \
        openssl \
        rsync \
        shadow \
        sudo \
    && rustup component add clippy rustfmt \
    && rustup target add x86_64-unknown-linux-musl

RUN printf '%s\n' '%wheel ALL=(ALL) NOPASSWD:ALL' >> /etc/sudoers

COPY <<'EOF' /entrypoint.sh
#!/bin/sh
set -e
USER=sinkd
RUSTUP_HOME=/usr/local/rustup
CARGO_HOME=/usr/local/cargo

if ! id -u "$USER" >/dev/null 2>&1; then
    adduser -D -s /bin/sh -G wheel "$USER"
    printf '%s:%s\n' "$USER" "$(openssl passwd -1 sinkd)" | chpasswd >/dev/null
fi

exec su -l "$USER" -c "\
cd \${WORKDIR:-~}; \
PATH=\${PATH} \
RUSTUP_HOME=${RUSTUP_HOME} \
CARGO_HOME=${CARGO_HOME} \
$*"
EOF

RUN chmod +x /entrypoint.sh

ENTRYPOINT ["/entrypoint.sh"]
CMD ["/bin/sh"]
