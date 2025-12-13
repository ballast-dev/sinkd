# Alpine/musl build - testing DDS proc-macro support
FROM rust:1.92-alpine

RUN apk add --no-cache \
  build-base \
  curl \
  fd \
  just \
  musl-dev \
  openssh-client \
  openssh-server \
  openssl \
  rsync \
  sudo \
  shadow

RUN rustup component add rustfmt clippy
RUN rustup target add x86_64-unknown-linux-musl

# Allow wheel group to run sudo without password
RUN echo '%wheel ALL=(ALL) NOPASSWD:ALL' >> /etc/sudoers

COPY <<'EOF' /entrypoint.sh
#!/bin/sh
USER=sinkd
PASSWORD=$(openssl passwd -1 sinkd)

RUSTUP_HOME=/usr/local/rustup
CARGO_HOME=/usr/local/cargo

adduser -D -s /bin/sh -G wheel sinkd
echo "sinkd:${PASSWORD}" | chpasswd > /dev/null 2>&1
exec su -l "${USER}" -c "\
  cd ${WORKDIR:-~}; \
  PATH=${PATH} \
  RUSTUP_HOME=${RUSTUP_HOME} \
  CARGO_HOME=${CARGO_HOME} \
  $*"
EOF

RUN chmod +x /entrypoint.sh

ENTRYPOINT ["/entrypoint.sh"]

CMD ["/bin/sh"]

