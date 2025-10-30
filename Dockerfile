# FROM rust:1.90-alpine
FROM rust:1.90-slim

RUN apt-get update && apt-get install -y \
  build-essential \
  cmake \
  curl \
  fd-find \
  just \
  libmosquitto-dev \
  libssl-dev \
  mosquitto \
  openssh-client \
  openssh-server \
  pkg-config \
  rsync \
  sudo \
  && rm -rf /var/lib/apt/lists/*

RUN <<EOF
ARCH=$(arch)
if [ "$ARCH" = "x86_64" ]; then
  ARCH="amd64"
elif [ "$ARCH" = "aarch64" ]; then
  ARCH="arm64"
fi
curl -fsSL https://github.com/launchfirestorm/bump/releases/download/v5.0.0/bump-linux-$ARCH -o /usr/local/bin/bump
chmod +x /usr/local/bin/bump
EOF


RUN rustup component add rustfmt clippy
RUN rustup target add aarch64-unknown-linux-gnu
# x86_64-pc-windows-msvc \
# aarch64-pc-windows-msvc \
# x86_64-apple-darwin \
# aarch64-apple-darwin

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
