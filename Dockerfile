FROM rust:1.90-alpine

RUN apk update && apk add \
  build-base \
  cmake \
  curl \
  fd \
  just \
  mosquitto \
  mosquitto-dev \
  openssh \
  openssl \
  openssl-dev \
  openssl-libs-static \
  rsync \
  sudo

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
RUN rustup target add \
  x86_64-unknown-linux-musl \
  aarch64-unknown-linux-musl
# x86_64-pc-windows-msvc \
# aarch64-pc-windows-msvc \
# x86_64-apple-darwin \
# aarch64-apple-darwin

# Allow wheel group to run sudo without password
RUN echo '%wheel ALL=(ALL) NOPASSWD:ALL' >> /etc/sudoers


COPY <<'EOF' /entrypoint.sh
#!/bin/sh
USER=sinkd
PASSWORD=$(openssl passwd -1 sinkd)

RUSTUP_HOME=/usr/local/rustup
CARGO_HOME=/usr/local/cargo

adduser -D -h /home/sinkd -G wheel sinkd
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
