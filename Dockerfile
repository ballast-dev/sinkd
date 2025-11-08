# FROM rust:1.90-alpine
FROM rust:1.90-slim

RUN apt-get update && apt-get install -y \
  build-essential \
  cmake \
  curl \
  fd-find \
  just \
  libmosquitto-dev \
  mosquitto \
  openssh-client \
  openssh-server \
  rsync \
  sudo \
  && rm -rf /var/lib/apt/lists/*

RUN rustup component add rustfmt clippy
RUN rustup target add \
  x86_64-unknown-linux-gnu \
  aarch64-unknown-linux-gnu

# Allow sudo group to run sudo without password
RUN echo '%sudo ALL=(ALL) NOPASSWD:ALL' >> /etc/sudoers

COPY <<'EOF' /entrypoint.sh
#!/bin/sh
USER=sinkd
PASSWORD=$(openssl passwd -1 sinkd)

RUSTUP_HOME=/usr/local/rustup
CARGO_HOME=/usr/local/cargo

useradd -m -s /bin/bash -G sudo sinkd
echo "sinkd:${PASSWORD}" | chpasswd > /dev/null 2>&1
exec su --pty -l "${USER}" -c "\
  cd ${WORKDIR:-~}; \
  PATH=${PATH} \
  RUSTUP_HOME=${RUSTUP_HOME} \
  CARGO_HOME=${CARGO_HOME} \
  $*"
EOF

RUN chmod +x /entrypoint.sh

ENTRYPOINT ["/entrypoint.sh"]
